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

    // ── Fake D-Bus (all intercepted interfaces) ──────────────────────────
    // Always start the proxy regardless of whether GPS is set — hostname1,
    // timedate1, locale1, login1, Accounts, and UPower are intercepted for
    // every profile to prevent identity leakage through D-Bus queries.
    println!("Starting fake D-Bus system bus for identity isolation...");
    let _dbus_proxy = match spawn_fake_system_bus(&ns.tmp_dir, profile) {
        Ok(proxy) => {
            // Wait for dbus-daemon to create the socket file
            let socket = ns.tmp_dir.join("system_bus_socket");
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
            while !socket.exists() && std::time::Instant::now() < deadline {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            if socket.exists() {
                Some(proxy)
            } else {
                eprintln!("Warning: dbus-daemon socket never appeared — D-Bus interception disabled");
                None
            }
        }
        Err(e) => {
            eprintln!("Warning: could not start D-Bus proxy: {} — D-Bus interception disabled", e);
            None
        }
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

    // ── Namespace isolation ───────────────────────────────────────────────
    cmd.arg("--unshare-uts");
    if let Some(ref hostname) = profile.hostname {
        cmd.args(["--hostname", hostname]);
    }

    // --unshare-net gives the sandbox its own network namespace. Crucially,
    // abstract Unix domain sockets (\0-prefixed names) are scoped per network
    // namespace in Linux. This means the host's abstract D-Bus session socket
    // (e.g. the real DBUS_SESSION_BUS_ADDRESS=unix:abstract=...) is completely
    // invisible inside the sandbox — the D-Bus proxy cannot be bypassed via
    // abstract sockets when this flag is present.
    // NOTE: if isolate_network is also true, the netns wrapping below supersedes
    // this flag. For profiles without network isolation we still add it here to
    // guarantee abstract socket isolation.
    if !profile.isolate_network.unwrap_or(false) {
        cmd.arg("--unshare-net");
    }

    // Writable tmpfs for /tmp inside the sandbox (size sampled per profile)
    cmd.args(["--tmpfs", "/tmp"]);

    // ── Bind-mount fake identity files ────────────────────────────────────
    let tmp_dir = &ns.tmp_dir;
    let mut dynamic_args: Vec<Vec<String>> = Vec::new();

    let fake_machine_id = tmp_dir.join("machine-id");
    if fake_machine_id.exists() {
        dynamic_args.push(vec!["--bind".to_string(), fake_machine_id.to_string_lossy().into_owned(), "/etc/machine-id".to_string()]);
    }

    let fake_hostname = tmp_dir.join("hostname");
    if fake_hostname.exists() {
        dynamic_args.push(vec!["--bind".to_string(), fake_hostname.to_string_lossy().into_owned(), "/etc/hostname".to_string()]);
    }

    let fake_timezone = tmp_dir.join("timezone");
    if fake_timezone.exists() && Path::new("/etc/timezone").exists() {
        dynamic_args.push(vec!["--bind".to_string(), fake_timezone.to_string_lossy().into_owned(), "/etc/timezone".to_string()]);
    }

    if let Some(ref tz) = profile.timezone {
        let zone_src = PathBuf::from(format!("/usr/share/zoneinfo/{tz}"));
        if zone_src.exists() {
            dynamic_args.push(vec!["--bind".to_string(), zone_src.to_string_lossy().into_owned(), "/etc/localtime".to_string()]);
        }
        dynamic_args.push(vec!["--setenv".to_string(), "TZ".to_string(), tz.clone()]);
    }

    let fake_resolv = tmp_dir.join("resolv.conf");
    if fake_resolv.exists() {
        dynamic_args.push(vec!["--bind".to_string(), fake_resolv.to_string_lossy().into_owned(), "/etc/resolv.conf".to_string()]);
    }

    let fake_cpuinfo = tmp_dir.join("cpuinfo");
    if fake_cpuinfo.exists() {
        dynamic_args.push(vec!["--bind".to_string(), fake_cpuinfo.to_string_lossy().into_owned(), "/proc/cpuinfo".to_string()]);
    }

    let fake_mac = tmp_dir.join("mac_address");
    if fake_mac.exists() && !profile.isolate_network.unwrap_or(false) {
        dynamic_args.push(vec!["--bind".to_string(), fake_mac.to_string_lossy().into_owned(), "/sys/class/net/eth0/address".to_string()]);
        dynamic_args.push(vec!["--bind".to_string(), fake_mac.to_string_lossy().into_owned(), "/sys/class/net/wlan0/address".to_string()]);
    }

    let fake_dbus = tmp_dir.join("system_bus_socket");
    if fake_dbus.exists() {
        let socket_str = fake_dbus.to_string_lossy().into_owned();
        let dbus_addr = format!("unix:path={}", socket_str);
        // Bind over the filesystem path so processes resolving the socket from /run see the proxy.
        dynamic_args.push(vec!["--bind".to_string(), socket_str, "/run/dbus/system_bus_socket".to_string()]);
        // Also set the environment variables so processes that use the abstract socket address
        // (bypassing /run/dbus) are redirected to our proxy unix:path socket instead.
        // Combined with --unshare-net this closes the abstract socket bypass completely.
        dynamic_args.push(vec!["--setenv".to_string(), "DBUS_SESSION_BUS_ADDRESS".to_string(), dbus_addr.clone()]);
        dynamic_args.push(vec!["--setenv".to_string(), "DBUS_SYSTEM_BUS_ADDRESS".to_string(), dbus_addr]);
    }

    if Path::new("/usr/lib/locale").exists() {
        dynamic_args.push(vec!["--bind".to_string(), "/usr/lib/locale".to_string(), "/usr/lib/locale".to_string()]);
    }

    if let Some(ref locale) = profile.locale {
        dynamic_args.push(vec!["--setenv".to_string(), "LANG".to_string(), locale.clone()]);
        dynamic_args.push(vec!["--setenv".to_string(), "LC_ALL".to_string(), locale.clone()]);
    }

    if let Ok(home) = std::env::var("HOME") {
        dynamic_args.push(vec!["--setenv".to_string(), "HOME".to_string(), home]);
    }
    if let Ok(user) = std::env::var("USER") {
        dynamic_args.push(vec!["--setenv".to_string(), "USER".to_string(), user]);
    }

    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();
    dynamic_args.shuffle(&mut rng);

    for arg_set in dynamic_args {
        cmd.args(arg_set);
    }

    // ── Application ───────────────────────────────────────────────────────
    cmd.arg("--");
    cmd.arg(app);
    cmd.args(args);

    Ok(cmd)
}



/// Launch an interactive session shell inside the sandbox
pub fn run_shell_in_sandbox(profile: &Profile, use_tmux: bool) -> Result<()> {
    let mut script = String::new();
    script.push_str(&format!("export MIRAGE_PROFILE=\"{}\"\n", profile.name));
    // Do NOT forward the real host D-Bus socket to the sandbox tmux session
    script.push_str("unset DBUS_SESSION_BUS_ADDRESS\n");
    script.push_str("unset DBUS_SYSTEM_BUS_ADDRESS\n");

    if use_tmux {
        script.push_str("if ! command -v tmux >/dev/null 2>&1; then\n");
        script.push_str("  echo \"[mirage] Error: tmux not found in the sandbox. Install tmux or omit --tmux.\"\n");
        script.push_str("  exit 1\n");
        script.push_str("fi\n");
        script.push_str("cat << 'EOF'\n");
        script.push_str("    ╔══════════════════════════════════════════════════════╗\n");
        // We use format to align the profile name nicely.
        script.push_str(&format!("    ║  MIRAGE SESSION (tmux): {:<27} ║\n", profile.name));
        script.push_str("    ║  New tmux panes inherit the sandbox (they are        ║\n");
        script.push_str("    ║  real children of this tmux server).                 ║\n");
        script.push_str("    ║                                                      ║\n");
        script.push_str("    ║  SHARP EDGE: terminal emulator 'New Tab' / 'New      ║\n");
        script.push_str("    ║  Window' buttons spawn outside the sandbox — they    ║\n");
        script.push_str("    ║  ask the GUI app (not this shell) to fork a new      ║\n");
        script.push_str("    ║  process, which is NOT in the namespace.             ║\n");
        script.push_str("    ║  Always use tmux panes (Ctrl-B %) inside this        ║\n");
        script.push_str("    ║  session, or a new `mirage shell` invocation.        ║\n");
        script.push_str("    ╚══════════════════════════════════════════════════════╝\n");
        script.push_str("EOF\n");
        script.push_str(&format!("exec tmux new-session -s \"mirage-{}\"\n", profile.name));
    } else {
        script.push_str("cat << 'EOF'\n");
        script.push_str("    ╔══════════════════════════════════════════════════════╗\n");
        script.push_str(&format!("    ║  MIRAGE SESSION: {:<34} ║\n", profile.name));
        script.push_str("    ║  All child processes inherit this profile.           ║\n");
        script.push_str("    ║  WARNING: terminal 'New Tab' opens outside sandbox.  ║\n");
        script.push_str("    ║  Use tmux panes or a new `mirage shell` invocation.  ║\n");
        script.push_str("    ║  Type 'exit' or Ctrl-D to leave the session.         ║\n");
        script.push_str("    ╚══════════════════════════════════════════════════════╝\n");
        script.push_str("EOF\n");
        script.push_str("exec bash\n");
    }

    let args = vec!["-c".to_string(), script];
    let res = run_in_sandbox("bash", &args, profile);

    println!("[mirage] Session '{}' ended. You are now on the host.", profile.name);

    res
}
