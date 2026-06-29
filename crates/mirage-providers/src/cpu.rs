use crate::IdentityProvider;
use anyhow::{Context, Result};
use mirage_protocol::{Profile, SignalKind, SignalValue};
use mirage_sandbox::SandboxHandle;
use std::fs;

pub struct CpuProvider;

impl IdentityProvider for CpuProvider {
    fn signal_kind(&self) -> SignalKind {
        // We'll reuse Hostname or create a new SignalKind if needed. 
        // For now, let's just use a dummy or add Cpu to SignalKind.
        // Wait, SignalKind doesn't have Cpu! I will add it to mirage_protocol.
        SignalKind::Cpu
    }

    fn real_value(&self) -> Result<SignalValue> {
        let content = fs::read_to_string("/proc/cpuinfo")?;
        let mut model = String::new();
        for line in content.lines() {
            if line.starts_with("model name") {
                if let Some(val) = line.split(':').nth(1) {
                    model = val.trim().to_string();
                    break;
                }
            }
        }
        Ok(SignalValue::Cpu(model))
    }

    fn projected_value(&self, profile: &Profile) -> Result<SignalValue> {
        let cpu = profile
            .cpu_model
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Profile '{}' has no cpu_model", profile.name))?;
        Ok(SignalValue::Cpu(cpu))
    }

    fn apply(&self, ns: &SandboxHandle, profile: &Profile) -> Result<()> {
        let cpu = profile
            .cpu_model
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Profile '{}' has no cpu_model field", profile.name))?;

        // Read real /proc/cpuinfo, replace model name lines, write to tmp
        let real_cpuinfo = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
        let mut fake_cpuinfo = String::new();
        
        for line in real_cpuinfo.lines() {
            if line.starts_with("model name") {
                fake_cpuinfo.push_str(&format!("model name\t: {}\n", cpu));
            } else {
                fake_cpuinfo.push_str(line);
                fake_cpuinfo.push('\n');
            }
        }

        let tmp_path = ns.tmp_dir.join("cpuinfo");
        fs::write(&tmp_path, fake_cpuinfo)?;
        Ok(())
    }
}
