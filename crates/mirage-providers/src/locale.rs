use anyhow::{Context, Result};
use mirage_protocol::{Profile, SignalKind, SignalValue};
use mirage_sandbox::SandboxHandle;
use std::env;
use std::fs;

use crate::IdentityProvider;

pub struct LocaleProvider;

impl IdentityProvider for LocaleProvider {
    fn signal_kind(&self) -> SignalKind {
        SignalKind::Locale
    }

    fn real_value(&self) -> Result<SignalValue> {
        // Try getting LANG from the environment first
        if let Ok(lang) = env::var("LANG") {
            if !lang.is_empty() {
                return Ok(SignalValue::Locale(lang));
            }
        }
        
        // Fallback to /etc/locale.conf
        if let Ok(content) = fs::read_to_string("/etc/locale.conf") {
            for line in content.lines() {
                if line.starts_with("LANG=") {
                    let lang = line["LANG=".len()..].trim_matches('"').trim();
                    return Ok(SignalValue::Locale(lang.to_string()));
                }
            }
        }
        
        anyhow::bail!("Failed to determine system locale")
    }

    fn projected_value(&self, _profile: &Profile) -> Result<SignalValue> {
        anyhow::bail!("Not implemented yet (Phase 2)")
    }

    fn apply(&self, _ns: &SandboxHandle, _profile: &Profile) -> Result<()> {
        Ok(())
    }
}
