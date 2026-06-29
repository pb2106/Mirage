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

    fn projected_value(&self, profile: &Profile) -> Result<SignalValue> {
        profile
            .timezone
            .clone()
            .map(SignalValue::Timezone)
            .ok_or_else(|| anyhow::anyhow!("Profile '{}' has no timezone field", profile.name))
    }

    fn apply(&self, ns: &SandboxHandle, profile: &Profile) -> Result<()> {
        let tz = profile
            .timezone
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Profile '{}' has no timezone field", profile.name))?;

        // Runner handles localtime

        // Also write a fake /etc/timezone
        let tmp_tz = ns.tmp_dir.join("timezone");
        std::fs::write(&tmp_tz, format!("{tz}\n"))?;

        // Runner handles TZ env
        Ok(())
    }
}
