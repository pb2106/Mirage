use anyhow::{Context, Result};
use mirage_protocol::{Profile, SignalKind, SignalValue};
use mirage_sandbox::SandboxHandle;
use std::fs;

use crate::IdentityProvider;

pub struct MachineIdProvider;

impl IdentityProvider for MachineIdProvider {
    fn signal_kind(&self) -> SignalKind {
        SignalKind::MachineId
    }

    fn real_value(&self) -> Result<SignalValue> {
        // usually in /etc/machine-id, fallback to /var/lib/dbus/machine-id
        let id = fs::read_to_string("/etc/machine-id")
            .or_else(|_| fs::read_to_string("/var/lib/dbus/machine-id"))
            .context("Failed to read machine-id from /etc or /var/lib/dbus")?;
            
        Ok(SignalValue::MachineId(id.trim().to_string()))
    }

    fn projected_value(&self, profile: &Profile) -> Result<SignalValue> {
        profile
            .machine_id
            .clone()
            .map(SignalValue::MachineId)
            .ok_or_else(|| anyhow::anyhow!("Profile '{}' has no machine_id field", profile.name))
    }

    fn apply(&self, ns: &SandboxHandle, profile: &Profile) -> Result<()> {
        let id = profile
            .machine_id
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Profile '{}' has no machine_id field", profile.name))?;

        // Validate: must be 32 lowercase hex chars
        let stripped = id.replace('-', "");
        if stripped.len() != 32 || !stripped.chars().all(|c| c.is_ascii_hexdigit()) {
            anyhow::bail!("machine_id '{}' is not a valid 32-char hex string", id);
        }

        let tmp_path = ns.tmp_dir.join("machine-id");
        std::fs::write(&tmp_path, format!("{stripped}\n"))?;
        Ok(())
    }
}
