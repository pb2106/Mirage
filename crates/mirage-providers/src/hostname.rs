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

    fn projected_value(&self, profile: &Profile) -> Result<SignalValue> {
        profile
            .hostname
            .clone()
            .map(SignalValue::Hostname)
            .ok_or_else(|| anyhow::anyhow!("Profile '{}' has no hostname field", profile.name))
    }

    fn apply(&self, ns: &SandboxHandle, profile: &Profile) -> Result<()> {
        let hostname = profile
            .hostname
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Profile '{}' has no hostname field", profile.name))?;

        // Write fake /etc/hostname into the tmp_dir
        let tmp_path = ns.tmp_dir.join("hostname");
        std::fs::write(&tmp_path, format!("{hostname}\n"))?;
        // Runner handles the bind mount
        Ok(())
    }
}
