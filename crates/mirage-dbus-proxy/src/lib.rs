//! `mirage-dbus-proxy` — A fake D-Bus system bus that intercepts well-known
//! freedesktop services and returns profile-consistent spoofed values.
//!
//! Intercepted interfaces:
//!   - org.freedesktop.GeoClue2        (GPS location)
//!   - org.freedesktop.hostname1       (hostname, machine-id, chassis, model)
//!   - org.freedesktop.timedate1       (timezone, NTP status)
//!   - org.freedesktop.locale1         (LANG, LC_* values)
//!   - org.freedesktop.login1          (session/user list — returns empty)
//!   - org.freedesktop.Accounts        (blocked — AccessDenied)
//!   - org.freedesktop.UPower          (device model strings stripped)

use anyhow::{Context, Result};
use mirage_protocol::{GpsCoord, Profile};
use std::path::{Path, PathBuf};
use tokio::runtime::Runtime;
use zbus::{dbus_interface, ConnectionBuilder};

// ── GeoClue2 ──────────────────────────────────────────────────────────────────

struct GeoClueManager;

#[dbus_interface(name = "org.freedesktop.GeoClue2.Manager")]
impl GeoClueManager {
    async fn get_client(&self) -> zbus::fdo::Result<zbus::zvariant::ObjectPath<'static>> {
        Ok(zbus::zvariant::ObjectPath::try_from("/org/freedesktop/GeoClue2/Client/1")
            .unwrap()
            .into())
    }
}

struct GeoClueClient {
    gps: GpsCoord,
}

#[dbus_interface(name = "org.freedesktop.GeoClue2.Client")]
impl GeoClueClient {
    #[dbus_interface(property)]
    async fn location(&self) -> zbus::fdo::Result<zbus::zvariant::ObjectPath<'static>> {
        Ok(zbus::zvariant::ObjectPath::try_from("/org/freedesktop/GeoClue2/Location/1")
            .unwrap()
            .into())
    }
    async fn start(&self) -> zbus::fdo::Result<()> { Ok(()) }
    async fn stop(&self) -> zbus::fdo::Result<()> { Ok(()) }
}

struct GeoClueLocation {
    gps: GpsCoord,
}

#[dbus_interface(name = "org.freedesktop.GeoClue2.Location")]
impl GeoClueLocation {
    #[dbus_interface(property)]
    async fn latitude(&self) -> f64 { self.gps.lat }
    #[dbus_interface(property)]
    async fn longitude(&self) -> f64 { self.gps.lon }
    #[dbus_interface(property)]
    async fn accuracy(&self) -> f64 { self.gps.accuracy }
    #[dbus_interface(property)]
    async fn altitude(&self) -> f64 { 0.0 }
}

// ── org.freedesktop.hostname1 ─────────────────────────────────────────────────

struct Hostname1 {
    hostname: String,
    machine_id: String,
    chassis: String,
    hardware_model: String,
}

#[dbus_interface(name = "org.freedesktop.hostname1")]
impl Hostname1 {
    #[dbus_interface(property)]
    async fn hostname(&self) -> &str { &self.hostname }
    #[dbus_interface(property)]
    async fn static_hostname(&self) -> &str { &self.hostname }
    #[dbus_interface(property)]
    async fn pretty_hostname(&self) -> &str { "" }
    #[dbus_interface(property)]
    async fn machine_id(&self) -> &str { &self.machine_id }
    #[dbus_interface(property)]
    async fn chassis(&self) -> &str { &self.chassis }
    #[dbus_interface(property)]
    async fn hardware_model(&self) -> &str { &self.hardware_model }
    #[dbus_interface(property)]
    async fn hardware_vendor(&self) -> &str { "Lenovo" }
    #[dbus_interface(property)]
    async fn os_pretty_name(&self) -> &str { "Ubuntu 22.04.3 LTS" }
    #[dbus_interface(property)]
    async fn icon_name(&self) -> &str { "computer-laptop" }
}

// ── org.freedesktop.timedate1 ─────────────────────────────────────────────────
//
// Note: Mirage does NOT spoof wall-clock time (TimeUSec). Only the TZ identifier
// is spoofed. Faking wall-clock time causes visible clock drift for the user.

struct Timedate1 {
    timezone: String,
}

#[dbus_interface(name = "org.freedesktop.timedate1")]
impl Timedate1 {
    #[dbus_interface(property)]
    async fn timezone(&self) -> &str { &self.timezone }
    #[dbus_interface(property)]
    async fn ntp_synchronized(&self) -> bool { true }
    #[dbus_interface(property)]
    async fn local_r_t_c(&self) -> bool { false }
    #[dbus_interface(property)]
    async fn ntp(&self) -> bool { true }
    // TimeUSec is intentionally NOT overridden; callers get real time from the kernel.
}

// ── org.freedesktop.locale1 ───────────────────────────────────────────────────

struct Locale1 {
    locale_env: Vec<String>,
}

#[dbus_interface(name = "org.freedesktop.locale1")]
impl Locale1 {
    /// Returns the full locale environment array, e.g.
    /// ["LANG=en_GB.UTF-8", "LC_TIME=de_DE.UTF-8"]
    #[dbus_interface(property)]
    async fn locale(&self) -> Vec<String> { self.locale_env.clone() }
    #[dbus_interface(property)]
    async fn x11_layout(&self) -> &str { "us" }
    #[dbus_interface(property)]
    async fn x11_model(&self) -> &str { "" }
    #[dbus_interface(property)]
    async fn x11_variant(&self) -> &str { "" }
    #[dbus_interface(property)]
    async fn x11_options(&self) -> &str { "" }
    #[dbus_interface(property)]
    async fn v_console_keymap(&self) -> &str { "us" }
    #[dbus_interface(property)]
    async fn v_console_keymap_toggle(&self) -> &str { "" }
}

// ── org.freedesktop.login1 ────────────────────────────────────────────────────
//
// Only identity-leaking properties are spoofed. Power management, inhibitor
// locks, and seat control are NOT intercepted — the user needs those to work.
// See docs/dbus-proxy.md for the complete list.

struct Login1Manager;

#[dbus_interface(name = "org.freedesktop.login1.Manager")]
impl Login1Manager {
    /// Returns an empty session list to prevent identity leakage.
    async fn list_sessions(
        &self,
    ) -> zbus::fdo::Result<Vec<(String, u32, String, String, zbus::zvariant::ObjectPath<'static>)>>
    {
        Ok(vec![])
    }

    /// Returns an empty user list to prevent identity leakage.
    async fn list_users(
        &self,
    ) -> zbus::fdo::Result<Vec<(u32, String, zbus::zvariant::ObjectPath<'static>)>>
    {
        Ok(vec![])
    }
}

// ── org.freedesktop.Accounts ──────────────────────────────────────────────────
//
// This interface is blocked entirely from the sandbox. Real account management
// must happen on the host, not through a sandboxed app. All calls return
// AccessDenied.

struct AccountsManager;

#[dbus_interface(name = "org.freedesktop.Accounts")]
impl AccountsManager {
    async fn find_user_by_name(&self, _name: &str) -> zbus::fdo::Result<zbus::zvariant::ObjectPath<'static>> {
        Err(zbus::fdo::Error::AccessDenied(
            "org.freedesktop.Accounts is not available inside the Mirage sandbox.".into(),
        ))
    }

    async fn find_user_by_id(&self, _id: i64) -> zbus::fdo::Result<zbus::zvariant::ObjectPath<'static>> {
        Err(zbus::fdo::Error::AccessDenied(
            "org.freedesktop.Accounts is not available inside the Mirage sandbox.".into(),
        ))
    }

    async fn create_user(&self, _name: &str, _fullname: &str, _account_type: i32) -> zbus::fdo::Result<zbus::zvariant::ObjectPath<'static>> {
        Err(zbus::fdo::Error::AccessDenied(
            "org.freedesktop.Accounts is not available inside the Mirage sandbox.".into(),
        ))
    }

    async fn list_cached_users(&self) -> zbus::fdo::Result<Vec<zbus::zvariant::ObjectPath<'static>>> {
        Err(zbus::fdo::Error::AccessDenied(
            "org.freedesktop.Accounts is not available inside the Mirage sandbox.".into(),
        ))
    }
}

// ── org.freedesktop.UPower ────────────────────────────────────────────────────
//
// Hardware model and serial number strings are stripped. Battery percentage,
// charge state, and time-to-empty are NOT faked — the user needs accurate
// power information.

struct UPowerDevice;

#[dbus_interface(name = "org.freedesktop.UPower.Device")]
impl UPowerDevice {
    #[dbus_interface(property)]
    async fn model(&self) -> &str { "Battery" }
    #[dbus_interface(property)]
    async fn serial(&self) -> &str { "" }
    #[dbus_interface(property)]
    async fn vendor(&self) -> &str { "" }
    // Percentage, state, time-to-empty are intentionally NOT faked.
}

struct UPowerManager;

#[dbus_interface(name = "org.freedesktop.UPower")]
impl UPowerManager {
    async fn get_devices(&self) -> zbus::fdo::Result<Vec<zbus::zvariant::ObjectPath<'static>>> {
        Ok(vec![
            zbus::zvariant::ObjectPath::try_from("/org/freedesktop/UPower/devices/battery_BAT0")
                .unwrap()
                .into(),
        ])
    }

    async fn get_display_device(&self) -> zbus::fdo::Result<zbus::zvariant::ObjectPath<'static>> {
        Ok(zbus::zvariant::ObjectPath::try_from("/org/freedesktop/UPower/devices/DisplayDevice")
            .unwrap()
            .into())
    }

    #[dbus_interface(property)]
    async fn on_battery(&self) -> bool { false }
    #[dbus_interface(property)]
    async fn lid_is_closed(&self) -> bool { false }
    #[dbus_interface(property)]
    async fn lid_is_present(&self) -> bool { true }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// A handle to the background thread running the fake D-Bus server.
/// Drops kill the `dbus-daemon` child process.
pub struct ProxyHandle {
    pub socket_path: PathBuf,
    _daemon: std::sync::Arc<std::sync::Mutex<std::process::Child>>,
}

impl Drop for ProxyHandle {
    fn drop(&mut self) {
        if let Ok(mut daemon) = self._daemon.lock() {
            let _ = daemon.kill();
        }
    }
}

/// Spawns a background `dbus-daemon` and registers fake handlers for all
/// intercepted freedesktop D-Bus interfaces.
///
/// The returned [`ProxyHandle`]'s `socket_path` must be bind-mounted into
/// the sandbox over `/run/dbus/system_bus_socket`, and `DBUS_SESSION_BUS_ADDRESS`
/// must be set to `unix:path=<socket_path>` inside the sandbox environment.
pub fn spawn_fake_system_bus(tmp_dir: &Path, profile: &Profile) -> Result<ProxyHandle> {
    let socket_path = tmp_dir.join("system_bus_socket");

    let config_path = tmp_dir.join("dbus.conf");
    let config = format!(
        r#"<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <type>session</type>
  <listen>unix:path={}</listen>
  <auth>EXTERNAL</auth>
  <policy context="default">
    <allow send_destination="*"/>
    <allow receive_sender="*"/>
    <allow own="*"/>
  </policy>
</busconfig>
"#,
        socket_path.display()
    );
    std::fs::write(&config_path, &config)?;

    let daemon = std::process::Command::new("dbus-daemon")
        .arg("--nofork")
        .arg(format!("--config-file={}", config_path.display()))
        .spawn()
        .context("Failed to spawn dbus-daemon — is dbus-daemon installed?")?;

    let daemon_arc = std::sync::Arc::new(std::sync::Mutex::new(daemon));
    let socket_path_clone = socket_path.clone();

    // Gather profile values for handlers (must be Send + 'static)
    let gps = profile.gps.clone().unwrap_or(GpsCoord { lat: 51.5074, lon: -0.1278, accuracy: 100.0 });
    let hostname = profile.hostname.clone().unwrap_or_else(|| "localhost".to_string());
    let machine_id = profile.machine_id.clone().unwrap_or_else(|| "00000000000000000000000000000000".to_string());
    let timezone = profile.timezone.clone().unwrap_or_else(|| "UTC".to_string());
    let locale_str = profile.locale.clone().unwrap_or_else(|| "en_US.UTF-8".to_string());
    let locale_env = vec![
        format!("LANG={}", locale_str),
        format!("LC_TIME={}", locale_str),
        format!("LC_NUMERIC={}", locale_str),
    ];

    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(200));

        let rt = Runtime::new().expect("Failed to create Tokio runtime for dbus-proxy");
        rt.block_on(async move {
            let address = format!("unix:path={}", socket_path_clone.display());

            let _conn = ConnectionBuilder::address(address.as_str())
                .expect("Valid dbus address")
                // GeoClue2
                .name("org.freedesktop.GeoClue2").expect("Valid name")
                .serve_at("/org/freedesktop/GeoClue2/Manager", GeoClueManager).expect("GeoClue2 Manager")
                .serve_at("/org/freedesktop/GeoClue2/Client/1", GeoClueClient { gps: gps.clone() }).expect("GeoClue2 Client")
                .serve_at("/org/freedesktop/GeoClue2/Location/1", GeoClueLocation { gps }).expect("GeoClue2 Location")
                // hostname1
                .serve_at("/org/freedesktop/hostname1", Hostname1 {
                    hostname,
                    machine_id,
                    chassis: "laptop".to_string(),
                    hardware_model: "ThinkPad X1 Carbon Gen 9".to_string(),
                }).expect("hostname1")
                // timedate1
                .serve_at("/org/freedesktop/timedate1", Timedate1 { timezone }).expect("timedate1")
                // locale1
                .serve_at("/org/freedesktop/locale1", Locale1 { locale_env }).expect("locale1")
                // login1 — only identity-leaking list methods; power/inhibitor not intercepted
                .serve_at("/org/freedesktop/login1", Login1Manager).expect("login1")
                // Accounts — fully blocked
                .serve_at("/org/freedesktop/Accounts", AccountsManager).expect("Accounts")
                // UPower — model/serial stripped
                .serve_at("/org/freedesktop/UPower", UPowerManager).expect("UPower manager")
                .serve_at("/org/freedesktop/UPower/devices/battery_BAT0", UPowerDevice).expect("UPower device")
                .build()
                .await
                .expect("Failed to connect to fake dbus-daemon");

            eprintln!("[mirage-dbus-proxy] Fake D-Bus system bus is live (hostname1, timedate1, locale1, login1, Accounts, UPower, GeoClue2)");

            std::future::pending::<()>().await;
        });
    });

    Ok(ProxyHandle {
        socket_path,
        _daemon: daemon_arc,
    })
}
