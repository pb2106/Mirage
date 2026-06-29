use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, std::hash::Hash, Serialize, Deserialize)]
pub enum SignalKind {
    Gps,
    Timezone,
    Locale,
    Dns,
    Ipv4,
    Ipv6,
    Hostname,
    MachineId,
    WebRtc,
    WifiScan,
    BluetoothScan,
    Cpu,
    MacAddress,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SignalValue {
    Gps { lat: f64, lon: f64, accuracy: f64 },
    Timezone(String),
    Locale(String),
    Dns(Vec<String>),
    Ipv4(String),
    Ipv6(String),
    Hostname(String),
    MachineId(String),
    WebRtc { public_ips: Vec<String>, local_ips: Vec<String> },
    WifiScan(Vec<WifiNetwork>),
    BluetoothScan(Vec<BluetoothDevice>),
    Cpu(String),
    MacAddress(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WifiNetwork {
    pub ssid: String,
    pub bssid: String,
    pub signal_level: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BluetoothDevice {
    pub name: Option<String>,
    pub address: String,
    pub rssi: i16,
}

/// A named identity profile loaded from a YAML file.
/// Phase 2: providers read from this to produce projected values.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    /// Human-readable profile name (e.g. "london-vpn").
    pub name: String,
    /// IANA timezone string (e.g. "Europe/London").
    pub timezone: Option<String>,
    /// Locale string (e.g. "en_GB.UTF-8").
    pub locale: Option<String>,
    /// Hostname to project inside the sandbox.
    pub hostname: Option<String>,
    /// Machine-ID to project (32 hex chars).
    pub machine_id: Option<String>,
    /// Fake CPU model name (e.g. "Intel(R) Core(TM) i9-10900K CPU @ 3.70GHz").
    pub cpu_model: Option<String>,
    /// Fake MAC address (e.g. "00:11:22:33:44:55").
    pub mac_address: Option<String>,
    /// DNS resolvers to expose inside the sandbox.
    pub dns: Option<Vec<String>>,
    /// Fake GPS coordinates.
    pub gps: Option<GpsCoord>,
    /// Whether to isolate the network into a separate netns (Phase 3).
    #[serde(default)]
    pub isolate_network: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpsCoord {
    pub lat: f64,
    pub lon: f64,
    pub accuracy: f64,
}

/// Load a [`Profile`] from a YAML file on disk.
///
/// # Errors
/// Returns an error if the file cannot be read or if the YAML is malformed.
pub fn load_profile(path: &Path) -> anyhow::Result<Profile> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read profile {:?}: {}", path, e))?;
    let profile: Profile = serde_yaml::from_str(&contents)
        .map_err(|e| anyhow::anyhow!("Failed to parse profile {:?}: {}", path, e))?;
    Ok(profile)
}

// ── Plugin ABI ────────────────────────────────────────────────────────────
// Stable C ABI that every Mirage plugin `.so` must export.
// Both the plugin crate and the plugin-host crate depend on these types
// from mirage-protocol, guaranteeing ABI alignment.

/// Opaque result type returned by plugin methods over FFI.
/// 0 = success, non-zero = error code.
pub type MiragePluginResult = i32;

pub const MIRAGE_OK: MiragePluginResult = 0;
pub const MIRAGE_ERR_NOT_SUPPORTED: MiragePluginResult = 1;
pub const MIRAGE_ERR_GENERAL: MiragePluginResult = 2;

/// Vtable that every plugin must populate and return from `mirage_plugin_vtable()`.
///
/// Layout must never change between versions — add new fields at the end only,
/// and bump `abi_version` accordingly.
#[repr(C)]
pub struct MiragePluginVtable {
    /// Monotonically-increasing ABI version. Currently 1.
    pub abi_version: u32,
    /// Null-terminated UTF-8 plugin name (e.g. "my-custom-provider").
    pub name: *const std::ffi::c_char,
    /// Null-terminated UTF-8 description.
    pub description: *const std::ffi::c_char,
    /// Called once after load so the plugin can initialise its own state.
    pub init: extern "C" fn() -> MiragePluginResult,
    /// Called before unload so the plugin can free its own state.
    pub destroy: extern "C" fn(),
    /// Run a projection step.  `profile_json` is the serialised [`Profile`]
    /// as a null-terminated UTF-8 JSON string.  `tmp_dir` is the sandbox
    /// temp directory path (null-terminated UTF-8).
    /// Returns [`MIRAGE_OK`] on success.
    pub apply: extern "C" fn(
        profile_json: *const std::ffi::c_char,
        tmp_dir: *const std::ffi::c_char,
    ) -> MiragePluginResult,
    /// Optional: return a null-terminated UTF-8 JSON string describing the
    /// current real value of this signal.  Caller must free via `free_string`.
    /// May be null if auditing is not supported.
    pub read_real: Option<extern "C" fn() -> *mut std::ffi::c_char>,
    /// Free a string that was allocated by the plugin (e.g. from `read_real`).
    pub free_string: extern "C" fn(ptr: *mut std::ffi::c_char),
}

// SAFETY: The vtable is a read-only bag of function pointers after init.
unsafe impl Send for MiragePluginVtable {}
unsafe impl Sync for MiragePluginVtable {}

/// The single exported symbol every plugin `.so` must provide.
/// ```no_run
/// #[no_mangle]
/// pub extern "C" fn mirage_plugin_vtable() -> *const MiragePluginVtable { ... }
/// ```
pub const MIRAGE_PLUGIN_VTABLE_SYMBOL: &[u8] = b"mirage_plugin_vtable\0";
