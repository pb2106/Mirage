use anyhow::Result;
use mirage_protocol::{Profile, SignalKind, SignalValue};
use mirage_sandbox::SandboxHandle;

use crate::IdentityProvider;

pub struct BluetoothProvider;

impl IdentityProvider for BluetoothProvider {
    fn signal_kind(&self) -> SignalKind {
        SignalKind::BluetoothScan
    }

    fn real_value(&self) -> Result<SignalValue> {
        // Stub for Phase 1: Real Bluetooth scan requires DBus BlueZ querying
        Ok(SignalValue::BluetoothScan(Vec::new()))
    }

    fn projected_value(&self, _profile: &Profile) -> Result<SignalValue> {
        anyhow::bail!("Not implemented yet")
    }

    fn apply(&self, _ns: &SandboxHandle, _profile: &Profile) -> Result<()> {
        Ok(())
    }
}
