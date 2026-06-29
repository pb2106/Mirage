use anyhow::Result;
use mirage_protocol::{Profile, SignalKind, SignalValue};
use mirage_sandbox::SandboxHandle;

use crate::IdentityProvider;

pub struct GeoClueProvider;

impl IdentityProvider for GeoClueProvider {
    fn signal_kind(&self) -> SignalKind {
        SignalKind::Gps
    }

    fn real_value(&self) -> Result<SignalValue> {
        // Stub for Phase 1: Real GeoClue requires DBus zbus querying
        // We will just return a placeholder or error for now, because writing a full
        // GeoClue DBus client is non-trivial for the MVP stub.
        Ok(SignalValue::Gps {
            lat: 0.0,
            lon: 0.0,
            accuracy: 0.0,
        })
    }

    fn projected_value(&self, _profile: &Profile) -> Result<SignalValue> {
        anyhow::bail!("Not implemented yet")
    }

    fn apply(&self, _ns: &SandboxHandle, _profile: &Profile) -> Result<()> {
        Ok(())
    }
}
