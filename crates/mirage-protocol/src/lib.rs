use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    // Add stub fields for profile data
}
