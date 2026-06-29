//! `runner` — launches an application inside a bwrap-based sandbox,
//! applying identity projections from a [`Profile`].

use anyhow::{Context, Result};
use mirage_protocol::Profile;
use mirage_providers::{
    dns::DnsProvider, hostname::HostnameProvider, locale::LocaleProvider, machine_id::MachineIdProvider,
    timezone::TimezoneProvider, IdentityProvider,
};
use mirage_sandbox::SandboxHandle;
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

    // Let each provider apply its projection into the tmp dir
    for provider in projection_providers() {
        if let Err(e) = provider.apply(&ns, profile) {
            eprintln!(
                "Warning: provider {:?} apply() skipped: {}",
                provider.signal_kind(),
                e
            );
        }
    }

    let mut cmd = build_bwrap_command(&ns, profile, app, args)?;
    println!("Launching: {:?}", cmd);

    let status = cmd
        .status()
        .with_context(|| format!("Failed to execute bwrap for '{}'", app))?;

    // Clean up temp dir after the child exits
    let _ = std::fs::remove_dir_all(&tmp_dir);

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
