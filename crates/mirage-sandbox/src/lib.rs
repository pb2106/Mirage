//! mirage-sandbox — Sandbox abstraction layer for Mirage.
//!
//! Phase 1: `SandboxHandle` is a stub representing an isolated execution
//! environment. Providers accept it in their `apply()` method so the trait
//! signature is stable, but actual bwrap integration is deferred to Phase 2.

use anyhow::Result;
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
/// In Phase 1 this is a no-op stub — it holds a PID of the process it wraps
/// once a sandbox is launched (Phase 2+), and exposes helper methods that
/// providers use to bind-mount fake files or set environment variables inside
/// the sandbox.
#[derive(Debug, Default)]
pub struct SandboxHandle {
    /// PID of the sandboxed process, if running.
    pub pid: Option<u32>,
    /// Ephemeral root directory used for the overlay (set by bwrap in Phase 2).
    pub root: Option<std::path::PathBuf>,
}

impl SandboxHandle {
    /// Create a new, inactive handle (Phase 1 stub).
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if the sandbox is currently running.
    pub fn is_active(&self) -> bool {
        self.pid.is_some()
    }

    /// Bind-mount a source path over a destination inside the sandbox root.
    ///
    /// Phase 1: no-op. Phase 2 will invoke bwrap's `--bind` flag or write
    /// into the overlay tmpfs.
    pub fn bind_mount(
        &self,
        _source: &std::path::Path,
        _dest: &std::path::Path,
    ) -> Result<()> {
        // Phase 2: implement via bwrap overlay
        Ok(())
    }

    /// Set an environment variable inside the sandbox.
    ///
    /// Phase 1: no-op. Phase 2 will pass `--setenv` to bwrap.
    pub fn set_env(&self, _key: &str, _value: &str) -> Result<()> {
        // Phase 2: implement via bwrap --setenv
        Ok(())
    }
}
