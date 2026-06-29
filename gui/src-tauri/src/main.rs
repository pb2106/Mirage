// Tauri v2: windows_subsystem is now handled per-target at the crate level.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use mirage_core::audit::AuditEngine;
use mirage_protocol::SignalValue;
use mirage_providers::{
    bluetooth::BluetoothProvider, dns::DnsProvider, geoclue::GeoClueProvider,
    hostname::HostnameProvider, locale::LocaleProvider, machine_id::MachineIdProvider,
    network::{Ipv4Provider, Ipv6Provider}, timezone::TimezoneProvider, webrtc::WebRtcProvider,
    wifi::WifiProvider,
};
use std::collections::HashMap;

// Tauri v2: #[tauri::command] is unchanged; generate_handler! is unchanged.
#[tauri::command]
fn run_audit() -> Result<HashMap<String, SignalValue>, String> {
    let mut engine = AuditEngine::new();
    engine.register_provider(Box::new(HostnameProvider));
    engine.register_provider(Box::new(MachineIdProvider));
    engine.register_provider(Box::new(TimezoneProvider));
    engine.register_provider(Box::new(LocaleProvider));
    engine.register_provider(Box::new(Ipv4Provider));
    engine.register_provider(Box::new(Ipv6Provider));
    engine.register_provider(Box::new(WebRtcProvider));
    engine.register_provider(Box::new(DnsProvider));
    engine.register_provider(Box::new(GeoClueProvider));
    engine.register_provider(Box::new(WifiProvider));
    engine.register_provider(Box::new(BluetoothProvider));

    match engine.run_audit(None) {
        Ok(results) => {
            let mut serialized = HashMap::new();
            for (k, v) in results {
                serialized.insert(format!("{:?}", k), v);
            }
            Ok(serialized)
        }
        Err(e) => Err(e.to_string()),
    }
}

// Tauri v2: Builder::default() → .invoke_handler() → .run() is unchanged.
// The `tauri::generate_context!()` macro is unchanged.
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![run_audit])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
