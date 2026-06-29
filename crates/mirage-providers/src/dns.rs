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

    fn projected_value(&self, profile: &Profile) -> Result<SignalValue> {
        let dns = profile
            .dns
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Profile '{}' has no dns field", profile.name))?;
        Ok(SignalValue::Dns(dns))
    }

    fn apply(&self, ns: &SandboxHandle, profile: &Profile) -> Result<()> {
        let dns = profile
            .dns
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Profile '{}' has no dns field", profile.name))?;

        let mut resolv_conf = String::new();
        for ns_ip in dns {
            resolv_conf.push_str(&format!("nameserver {}\n", ns_ip));
        }

        let tmp_path = ns.tmp_dir.join("resolv.conf");
        std::fs::write(&tmp_path, resolv_conf)?;

        Ok(())
    }
}
