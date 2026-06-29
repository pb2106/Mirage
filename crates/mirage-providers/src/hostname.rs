use anyhow::{Context, Result};
use mirage_protocol::{Profile, SignalKind, SignalValue};
use mirage_sandbox::SandboxHandle;

use crate::IdentityProvider;

pub struct HostnameProvider;

impl IdentityProvider for HostnameProvider {
    fn signal_kind(&self) -> SignalKind {
        SignalKind::Hostname
    }

    fn real_value(&self) -> Result<SignalValue> {
        let hostname_os = nix::unistd::gethostname()
            .context("Failed to get hostname")?;
        
        let hostname_str = hostname_os
            .into_string()
            .map_err(|_| anyhow::anyhow!("Hostname is not valid UTF-8"))?;
            
        Ok(SignalValue::Hostname(hostname_str))
    }

    fn projected_value(&self, _profile: &Profile) -> Result<SignalValue> {
        // Stub for Phase 2
        anyhow::bail!("Not implemented yet (Phase 2)")
    }

    fn apply(&self, _ns: &SandboxHandle, _profile: &Profile) -> Result<()> {
        // Stub for Phase 2
        Ok(())
    }
}
