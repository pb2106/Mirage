use anyhow::Result;
use mirage_protocol::{SignalKind, SignalValue};
use mirage_providers::IdentityProvider;
use std::collections::HashMap;

pub mod session;

pub struct AuditEngine {
    providers: Vec<Box<dyn IdentityProvider>>,
}

impl AuditEngine {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    pub fn register_provider(&mut self, provider: Box<dyn IdentityProvider>) {
        self.providers.push(provider);
    }

    /// Audit a specific process ID (or the host if None).
    /// For Phase 1 MVP, we just read the real values from the system.
    pub fn run_audit(&self, _pid: Option<u32>) -> Result<HashMap<SignalKind, SignalValue>> {
        let mut results = HashMap::new();
        
        for provider in &self.providers {
            match provider.real_value() {
                Ok(val) => {
                    results.insert(provider.signal_kind(), val);
                }
                Err(e) => {
                    eprintln!("Failed to read signal {:?}: {}", provider.signal_kind(), e);
                }
            }
        }
        
        Ok(results)
    }
}
