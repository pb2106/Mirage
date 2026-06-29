use anyhow::{Context, Result};
use mirage_protocol::{Profile, SignalKind, SignalValue};
use mirage_sandbox::SandboxHandle;
use std::fs;

use crate::IdentityProvider;

pub struct DnsProvider;

impl IdentityProvider for DnsProvider {
    fn signal_kind(&self) -> SignalKind {
        SignalKind::Dns
    }

    fn real_value(&self) -> Result<SignalValue> {
        let content = fs::read_to_string("/etc/resolv.conf")
            .context("Failed to read /etc/resolv.conf")?;
            
        let mut servers = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("nameserver") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() > 1 {
                    servers.push(parts[1].to_string());
                }
            }
        }
        
        Ok(SignalValue::Dns(servers))
    }

    fn projected_value(&self, _profile: &Profile) -> Result<SignalValue> {
        anyhow::bail!("Not implemented yet")
    }

    fn apply(&self, _ns: &SandboxHandle, _profile: &Profile) -> Result<()> {
        Ok(())
    }
}
