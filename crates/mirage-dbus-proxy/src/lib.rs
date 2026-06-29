//! `mirage-dbus-proxy` — Provides a fake D-Bus system bus socket that implements
//! spoofed services (like GeoClue2) to isolate sandboxed applications.

use anyhow::{Context, Result};
use mirage_protocol::GpsCoord;
use std::path::{Path, PathBuf};
use tokio::runtime::Runtime;
use zbus::{dbus_interface, ConnectionBuilder};

// ── GeoClue2 Manager Stub ──────────────────────────────────────────────────

struct GeoClueManager;

#[dbus_interface(name = "org.freedesktop.GeoClue2.Manager")]
impl GeoClueManager {
    /// Sandboxed apps call this to get a Client object to track location.
    async fn get_client(&self) -> zbus::fdo::Result<zbus::zvariant::ObjectPath<'static>> {
        Ok(zbus::zvariant::ObjectPath::try_from("/org/freedesktop/GeoClue2/Client/1")
            .unwrap()
            .into())
    }
}

// ── GeoClue2 Client Stub ───────────────────────────────────────────────────

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

    async fn start(&self) -> zbus::fdo::Result<()> {
        Ok(())
    }

    async fn stop(&self) -> zbus::fdo::Result<()> {
        Ok(())
    }
}

// ── GeoClue2 Location Stub ─────────────────────────────────────────────────

struct GeoClueLocation {
    gps: GpsCoord,
}

#[dbus_interface(name = "org.freedesktop.GeoClue2.Location")]
impl GeoClueLocation {
    #[dbus_interface(property)]
    async fn latitude(&self) -> f64 {
        self.gps.lat
    }

    #[dbus_interface(property)]
    async fn longitude(&self) -> f64 {
        self.gps.lon
    }

    #[dbus_interface(property)]
    async fn accuracy(&self) -> f64 {
        self.gps.accuracy
    }

    #[dbus_interface(property)]
    async fn altitude(&self) -> f64 {
        0.0
    }
}

// ── D-Bus Proxy Runner ─────────────────────────────────────────────────────

/// A handle to the background thread running the fake D-Bus server.
/// Automatically kills the `dbus-daemon` child process on drop.
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

/// Spawns a background `dbus-daemon` and a Tokio task that registers
/// a fake `org.freedesktop.GeoClue2` service on it.
///
/// Returns a [`ProxyHandle`] whose `socket_path` should be bind-mounted
/// into the sandbox over `/run/dbus/system_bus_socket`.
pub fn spawn_fake_system_bus(tmp_dir: &Path, gps: Option<GpsCoord>) -> Result<ProxyHandle> {
    let socket_path = tmp_dir.join("system_bus_socket");

    // Write a minimal dbus-daemon config that listens on our socket
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

    // Launch dbus-daemon in the background (forks internally; --nofork keeps it in foreground)
    let daemon = std::process::Command::new("dbus-daemon")
        .arg("--nofork")
        .arg(format!("--config-file={}", config_path.display()))
        .spawn()
        .context("Failed to spawn dbus-daemon — is dbus-daemon installed?")?;

    let daemon_arc = std::sync::Arc::new(std::sync::Mutex::new(daemon));
    let socket_path_clone = socket_path.clone();

    let gps = gps.unwrap_or(GpsCoord {
        lat: 51.5074,
        lon: -0.1278,
        accuracy: 100.0,
    });

    std::thread::spawn(move || {
        // Give dbus-daemon a moment to create the socket
        std::thread::sleep(std::time::Duration::from_millis(200));

        let rt = Runtime::new().expect("Failed to create Tokio runtime for dbus-proxy");
        rt.block_on(async move {
            let address = format!("unix:path={}", socket_path_clone.display());

            let _conn = ConnectionBuilder::address(address.as_str())
                .expect("Valid dbus address")
                .name("org.freedesktop.GeoClue2")
                .expect("Valid well-known name")
                .serve_at("/org/freedesktop/GeoClue2/Manager", GeoClueManager)
                .expect("Failed to register GeoClue2 Manager")
                .serve_at(
                    "/org/freedesktop/GeoClue2/Client/1",
                    GeoClueClient { gps: gps.clone() },
                )
                .expect("Failed to register GeoClue2 Client")
                .serve_at(
                    "/org/freedesktop/GeoClue2/Location/1",
                    GeoClueLocation { gps },
                )
                .expect("Failed to register GeoClue2 Location")
                .build()
                .await
                .expect("Failed to connect to fake dbus-daemon");

            eprintln!("[mirage-dbus-proxy] Fake GeoClue2 service is live");

            // Keep the runtime — and the connection — alive until the thread is killed
            std::future::pending::<()>().await;
        });
    });

    Ok(ProxyHandle {
        socket_path,
        _daemon: daemon_arc,
    })
}
