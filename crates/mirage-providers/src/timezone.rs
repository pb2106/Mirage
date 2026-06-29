use anyhow::{Context, Result};
use mirage_protocol::{Profile, SignalKind, SignalValue};
use mirage_sandbox::SandboxHandle;
use std::fs;

use crate::IdentityProvider;

pub struct TimezoneProvider;

impl IdentityProvider for TimezoneProvider {
    fn signal_kind(&self) -> SignalKind {
        SignalKind::Timezone
    }

    fn real_value(&self) -> Result<SignalValue> {
        // Read /etc/timezone or the symlink target of /etc/localtime
        if let Ok(tz) = fs::read_to_string("/etc/timezone") {
            let tz = tz.trim();
            if !tz.is_empty() {
                return Ok(SignalValue::Timezone(tz.to_string()));
            }
        }
        
        if let Ok(target) = fs::read_link("/etc/localtime") {
            let path_str = target.to_string_lossy();
            // Typically points to something like /usr/share/zoneinfo/Europe/London
            if let Some(idx) = path_str.find("zoneinfo/") {
                let tz = &path_str[idx + "zoneinfo/".len()..];
                return Ok(SignalValue::Timezone(tz.to_string()));
            }
        }
        
        anyhow::bail!("Failed to determine system timezone")
    }

    fn projected_value(&self, _profile: &Profile) -> Result<SignalValue> {
        anyhow::bail!("Not implemented yet (Phase 2)")
    }

    fn apply(&self, _ns: &SandboxHandle, _profile: &Profile) -> Result<()> {
        Ok(())
    }
}
