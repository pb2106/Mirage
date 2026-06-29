use crate::IdentityProvider;
use anyhow::Result;
use mirage_protocol::{Profile, SignalKind, SignalValue};
use mirage_sandbox::SandboxHandle;
use std::fs;

pub struct MacProvider;

impl IdentityProvider for MacProvider {
    fn signal_kind(&self) -> SignalKind {
        SignalKind::MacAddress
    }

    fn real_value(&self) -> Result<SignalValue> {
        // Read MAC of the default interface. For a stub, we'll try to find eth0 or wlan0
        let mac = fs::read_to_string("/sys/class/net/eth0/address")
            .or_else(|_| fs::read_to_string("/sys/class/net/wlan0/address"))
            .unwrap_or_else(|_| "00:00:00:00:00:00\n".to_string());
        
        Ok(SignalValue::MacAddress(mac.trim().to_string()))
    }

    fn projected_value(&self, profile: &Profile) -> Result<SignalValue> {
        let mac = profile
            .mac_address
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Profile '{}' has no mac_address", profile.name))?;
        Ok(SignalValue::MacAddress(mac))
    }

    fn apply(&self, ns: &SandboxHandle, profile: &Profile) -> Result<()> {
        let mac = profile
            .mac_address
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Profile '{}' has no mac_address field", profile.name))?;

        let tmp_path = ns.tmp_dir.join("mac_address");
        fs::write(&tmp_path, format!("{}\n", mac))?;
        Ok(())
    }
}
