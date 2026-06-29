//! mirage-sandbox — Sandbox abstraction layer for Mirage.
//!
//! Phase 2: `SandboxHandle` tracks the per-run temporary directory where
//! providers write fake files before launching the sandbox.

use anyhow::Result;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SandboxError {
    #[error("sandbox is not active")]
    NotActive,
    #[error("bwrap error: {0}")]
    Bwrap(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// A handle to an active (or pending) sandbox environment.
///
/// In Phase 2, this holds the temporary directory where providers write their
/// fake files. The actual `bwrap` execution is handled by `mirage-core::runner`.
#[derive(Debug)]
pub struct SandboxHandle {
    /// PID of the sandboxed process, if running.
    pub pid: Option<u32>,
    /// Ephemeral root directory used for the overlay (set by bwrap in Phase 2).
    pub root: Option<PathBuf>,
    /// Temporary directory for this run.
    pub tmp_dir: PathBuf,
}

impl Default for SandboxHandle {
    fn default() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let hex: String = (0..8).map(|_| format!("{:x}", rng.gen::<u8>() % 16)).collect();
        let base = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
        
        Self {
            pid: None,
            root: None,
            tmp_dir: PathBuf::from(base).join(hex),
        }
    }
}

impl SandboxHandle {
    /// Create a new handle with the given temporary directory.
    pub fn with_tmp_dir(tmp_dir: PathBuf) -> Self {
        Self {
            pid: None,
            root: None,
            tmp_dir,
        }
    }

    /// Create a new, inactive handle (default tmp dir).
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if the sandbox is currently running.
    pub fn is_active(&self) -> bool {
        self.pid.is_some()
    }

    /// Bind-mount a source path over a destination inside the sandbox root.
    ///
    /// Phase 2: Currently handled directly in the runner builder.
    pub fn bind_mount(&self, _source: &Path, _dest: &Path) -> Result<()> {
        Ok(())
    }

    /// Set an environment variable inside the sandbox.
    ///
    /// Phase 2: Currently handled directly in the runner builder.
    pub fn set_env(&self, _key: &str, _value: &str) -> Result<()> {
        Ok(())
    }
}
