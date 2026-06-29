use anyhow::{Context, Result};
use mirage_protocol::{Profile, SignalKind, SignalValue};
use mirage_sandbox::SandboxHandle;

use crate::IdentityProvider;

pub struct Ipv4Provider;
pub struct Ipv6Provider;

impl IdentityProvider for Ipv4Provider {
    fn signal_kind(&self) -> SignalKind {
        SignalKind::Ipv4
    }

    fn real_value(&self) -> Result<SignalValue> {
        // Query a public API for our IPv4 address
        let response = reqwest::blocking::get("https://api.ipify.org")
            .context("Failed to get public IPv4")?;
        let ip = response.text()?.trim().to_string();
        Ok(SignalValue::Ipv4(ip))
    }

    fn projected_value(&self, _profile: &Profile) -> Result<SignalValue> {
        anyhow::bail!("Not implemented yet")
    }

    fn apply(&self, _ns: &SandboxHandle, _profile: &Profile) -> Result<()> {
        Ok(())
    }
}

impl IdentityProvider for Ipv6Provider {
    fn signal_kind(&self) -> SignalKind {
        SignalKind::Ipv6
    }

    fn real_value(&self) -> Result<SignalValue> {
        // Query a public API for our IPv6 address
        let response = reqwest::blocking::get("https://api64.ipify.org")
            .context("Failed to get public IPv6")?;
        let ip = response.text()?.trim().to_string();
        Ok(SignalValue::Ipv6(ip))
    }

    fn projected_value(&self, _profile: &Profile) -> Result<SignalValue> {
        anyhow::bail!("Not implemented yet")
    }

    fn apply(&self, _ns: &SandboxHandle, _profile: &Profile) -> Result<()> {
        Ok(())
    }
}
