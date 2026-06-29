//! `runner` — launches an application inside a bwrap-based sandbox,
//! applying identity projections from a [`Profile`].

use anyhow::{Context, Result};
use mirage_protocol::Profile;
use mirage_providers::{
    cpu::CpuProvider, dns::DnsProvider, hostname::HostnameProvider, locale::LocaleProvider, 
    mac::MacProvider, machine_id::MachineIdProvider, timezone::TimezoneProvider, IdentityProvider,
};
use mirage_sandbox::SandboxHandle;
use mirage_netns::Netns;
use mirage_dbus_proxy::spawn_fake_system_bus;
use mirage_plugin_host::PluginHost;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Providers whose `apply()` we invoke before spawning the sandbox.
fn projection_providers() -> Vec<Box<dyn IdentityProvider>> {
    vec![
        Box::new(HostnameProvider),
        Box::new(MachineIdProvider),
        Box::new(TimezoneProvider),
        Box::new(LocaleProvider),
        Box::new(DnsProvider),
        Box::new(CpuProvider),
        Box::new(MacProvider),
    ]
}

/// Unique temp directory for this sandbox run, keyed by PID so concurrent
/// runs don't clobber each other's files.
fn sandbox_tmp_dir() -> PathBuf {
    PathBuf::from(format!("/tmp/mirage-{}", std::process::id()))
}

/// Launch `app` with `args` inside a bwrap sandbox configured by `profile`.
pub fn run_in_sandbox(app: &str, args: &[String], profile: &Profile) -> Result<()> {
    println!("Preparing sandbox for profile '{}'...", profile.name);

    let tmp_dir = sandbox_tmp_dir();
    std::fs::create_dir_all(&tmp_dir)
        .with_context(|| format!("Failed to create sandbox temp dir {:?}", tmp_dir))?;

    // SandboxHandle holds the per-run tmp dir so providers write there
    let ns = SandboxHandle::with_tmp_dir(tmp_dir.clone());

    // Let each built-in provider apply its projection into the tmp dir
    for provider in projection_providers() {
        if let Err(e) = provider.apply(&ns, profile) {
            eprintln!(
                "Warning: provider {:?} apply() skipped: {}",
                provider.signal_kind(),
                e
            );
        }
    }

    // ── Plugin providers ──────────────────────────────────────────────────
    // Load plugins from ~/.config/mirage/plugins/*.so and run their apply()
    let mut plugin_host = PluginHost::new();
    plugin_host.load_from_default_dir();
    if !plugin_host.is_empty() {
        println!("[plugin-host] Applying {} plugin(s)...", plugin_host.plugins().len());
        plugin_host.apply_all(profile, &tmp_dir);
    }

    // ── Fake D-Bus (GeoClue2) ─────────────────────────────────────────────
    // Must start BEFORE build_bwrap_command so the socket file exists when
    // we check for it and add the --bind argument.
    let _dbus_proxy = if profile.gps.is_some() {
        println!("Starting fake D-Bus system bus for location spoofing...");
        let proxy = spawn_fake_system_bus(&ns.tmp_dir, profile.gps.clone())?;
        // Wait for dbus-daemon to create the socket file
        let socket = ns.tmp_dir.join("system_bus_socket");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while !socket.exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        if !socket.exists() {
            eprintln!("Warning: dbus-daemon socket never appeared — D-Bus spoofing disabled");
            None
        } else {
            Some(proxy)
        }
    } else {
        None
    };

    let mut cmd = build_bwrap_command(&ns, profile, app, args)?;

    // ── Network namespace ─────────────────────────────────────────────────
    let _netns = if profile.isolate_network.unwrap_or(false) {
        println!("Setting up isolated network namespace (may prompt for sudo)...");
        let netns = Netns::create(std::process::id(), profile.mac_address.as_deref())?;

        let user = std::env::var("USER").unwrap_or_else(|_| "nobody".to_string());

        let mut wrapped_cmd = Command::new("sudo");
        wrapped_cmd.args(["ip", "netns", "exec", &netns.name, "sudo", "-E", "-u", &user]);
        wrapped_cmd.arg(cmd.get_program());
        wrapped_cmd.args(cmd.get_args());

        cmd = wrapped_cmd;
        Some(netns)
    } else {
        None
    };

    println!("Launching: {:?}", cmd);

    let status = cmd
        .status()
        .with_context(|| format!("Failed to execute bwrap for '{}'", app))?;

    // Clean up temp dir after the child exits
    let _ = std::fs::remove_dir_all(&tmp_dir);
    // _netns drops here and cleans up the namespace

    if !status.success() {
        anyhow::bail!("Sandboxed process exited with status: {}", status);
    }

    Ok(())
}

/// Assemble the `bwrap` [`Command`].
fn build_bwrap_command(
    ns: &SandboxHandle,
    profile: &Profile,
    app: &str,
    args: &[String],
) -> Result<Command> {
    let mut cmd = Command::new("bwrap");

    // ── Filesystem skeleton ────────────────────────────────────────────────
    // Bind the real root so the app sees the full system
    cmd.args(["--bind", "/", "/"]);

    // Mount real /dev (gives /dev/null, /dev/pts, etc.)
    cmd.args(["--dev-bind", "/dev", "/dev"]);

    // Mount /proc so tools like ps, /proc/self work
    cmd.args(["--proc", "/proc"]);

    // Writable tmpfs for /tmp inside the sandbox
    cmd.args(["--tmpfs", "/tmp"]);

    // ── UTS / hostname ─────────────────────────────────────────────────────
    cmd.arg("--unshare-uts");
    if let Some(ref hostname) = profile.hostname {
        cmd.args(["--hostname", hostname]);
    }

    // ── Bind-mount fake identity files ────────────────────────────────────
    let tmp_dir = &ns.tmp_dir;

    // /etc/machine-id
    let fake_machine_id = tmp_dir.join("machine-id");
    if fake_machine_id.exists() {
        bind_over(&mut cmd, &fake_machine_id, Path::new("/etc/machine-id"));
    }

    // /etc/hostname
    let fake_hostname = tmp_dir.join("hostname");
    if fake_hostname.exists() {
        bind_over(&mut cmd, &fake_hostname, Path::new("/etc/hostname"));
    }

    // /etc/timezone — only if the target actually exists on this host
    let fake_timezone = tmp_dir.join("timezone");
    if fake_timezone.exists() && Path::new("/etc/timezone").exists() {
        bind_over(&mut cmd, &fake_timezone, Path::new("/etc/timezone"));
    }

    // /etc/localtime — bind the correct zoneinfo file
    if let Some(ref tz) = profile.timezone {
        let zone_src = PathBuf::from(format!("/usr/share/zoneinfo/{tz}"));
        if zone_src.exists() {
            bind_over(&mut cmd, &zone_src, Path::new("/etc/localtime"));
        }
        cmd.args(["--setenv", "TZ", tz]);
    }

    // /etc/resolv.conf
    let fake_resolv = tmp_dir.join("resolv.conf");
    if fake_resolv.exists() {
        bind_over(&mut cmd, &fake_resolv, Path::new("/etc/resolv.conf"));
    }

    // /proc/cpuinfo
    let fake_cpuinfo = tmp_dir.join("cpuinfo");
    if fake_cpuinfo.exists() {
        bind_over(&mut cmd, &fake_cpuinfo, Path::new("/proc/cpuinfo"));
    }

    // MAC Address (we'll bind it over eth0 and wlan0 if they exist)
    // ONLY do this if we are not isolating the network, because inside an isolated netns,
    // the host's eth0/wlan0 do not exist, and bwrap will fail to bind mount over them.
    let fake_mac = tmp_dir.join("mac_address");
    if fake_mac.exists() && !profile.isolate_network.unwrap_or(false) {
        bind_over(&mut cmd, &fake_mac, Path::new("/sys/class/net/eth0/address"));
        bind_over(&mut cmd, &fake_mac, Path::new("/sys/class/net/wlan0/address"));
    }

    // D-Bus System Bus — bind our fake socket over the canonical path only.
    // /var/run is a symlink to /run on modern systems so one bind suffices.
    let fake_dbus = tmp_dir.join("system_bus_socket");
    if fake_dbus.exists() {
        bind_over(&mut cmd, &fake_dbus, Path::new("/run/dbus/system_bus_socket"));
    }

    // ── Locale data ────────────────────────────────────────────────────────
    // Bind /usr/lib/locale so installed locale data is visible inside the sandbox
    if Path::new("/usr/lib/locale").exists() {
        cmd.args(["--bind", "/usr/lib/locale", "/usr/lib/locale"]);
    }

    // Inject locale env vars
    if let Some(ref locale) = profile.locale {
        cmd.args(["--setenv", "LANG", locale]);
        cmd.args(["--setenv", "LC_ALL", locale]);
    }

    // ── Application ───────────────────────────────────────────────────────
    cmd.arg("--");
    cmd.arg(app);
    cmd.args(args);

    Ok(cmd)
}

/// Add a `--bind <src> <dst>` pair to the command, only if the destination
/// path exists on the host (bwrap requires the target to already exist).
fn bind_over(cmd: &mut Command, src: &Path, dst: &Path) {
    if dst.exists() {
        cmd.args([
            "--bind",
            src.to_str().unwrap_or(""),
            dst.to_str().unwrap_or(""),
        ]);
    }
}
