use anyhow::Result;
use mirage_protocol::{Profile, SignalKind, SignalValue};
use mirage_sandbox::SandboxHandle;

use crate::IdentityProvider;

pub struct WifiProvider;

impl IdentityProvider for WifiProvider {
    fn signal_kind(&self) -> SignalKind {
        SignalKind::WifiScan
    }

    fn real_value(&self) -> Result<SignalValue> {
        // Stub for Phase 1: Real Wi-Fi scan requires DBus NetworkManager or nl80211 querying
        Ok(SignalValue::WifiScan(Vec::new()))
    }

    fn projected_value(&self, _profile: &Profile) -> Result<SignalValue> {
        anyhow::bail!("Not implemented yet")
    }

    fn apply(&self, _ns: &SandboxHandle, _profile: &Profile) -> Result<()> {
        Ok(())
    }
}
