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

    fn projected_value(&self, _profile: &Profile) -> Result<SignalValue> {
        anyhow::bail!("Not implemented yet (Phase 2)")
    }

    fn apply(&self, _ns: &SandboxHandle, _profile: &Profile) -> Result<()> {
        Ok(())
    }
}
