//! `mirage-plugin-host` — loads and manages Mirage plugin `.so` shared libraries.
//!
//! ## Plugin discovery
//! Plugins are scanned from `~/.config/mirage/plugins/*.so` at startup.
//!
//! ## Plugin ABI
//! Every plugin must export a single symbol:
//! ```c
//! const MiragePluginVtable *mirage_plugin_vtable(void);
//! ```
//! The vtable layout is defined in `mirage-protocol::MiragePluginVtable`.

use anyhow::{Context, Result};
use libloading::{Library, Symbol};
use mirage_protocol::{
    MiragePluginVtable, MiragePluginResult, MIRAGE_OK, MIRAGE_PLUGIN_VTABLE_SYMBOL, Profile,
};
use std::ffi::{CStr, CString};
use std::path::{Path, PathBuf};

// ── Loaded plugin ──────────────────────────────────────────────────────────

/// A single successfully loaded and initialised plugin.
pub struct LoadedPlugin {
    /// Human-readable plugin name extracted from the vtable.
    pub name: String,
    /// Human-readable description extracted from the vtable.
    pub description: String,
    // The vtable pointer (valid for the lifetime of `_lib`).
    vtable: *const MiragePluginVtable,
    // Keep the library alive — dropping it would unload the `.so`.
    _lib: Library,
}

// SAFETY: We never call vtable methods from multiple threads simultaneously.
unsafe impl Send for LoadedPlugin {}

impl Drop for LoadedPlugin {
    fn drop(&mut self) {
        // Call the plugin's teardown hook before the library is unloaded.
        unsafe {
            ((*self.vtable).destroy)();
        }
    }
}

impl LoadedPlugin {
    /// Invoke the plugin's `apply` function with the serialised profile.
    pub fn apply(&self, profile: &Profile, tmp_dir: &Path) -> Result<()> {
        let profile_json = serde_json::to_string(profile)
            .context("Failed to serialise profile to JSON for plugin")?;
        let profile_cstr = CString::new(profile_json)
            .context("Profile JSON contained a null byte")?;
        let tmp_cstr = CString::new(tmp_dir.to_string_lossy().as_bytes())
            .context("tmp_dir path contained a null byte")?;

        let rc: MiragePluginResult = unsafe {
            ((*self.vtable).apply)(profile_cstr.as_ptr(), tmp_cstr.as_ptr())
        };

        if rc != MIRAGE_OK {
            anyhow::bail!(
                "Plugin '{}' apply() returned error code {}",
                self.name, rc
            );
        }
        Ok(())
    }

    /// Invoke the plugin's optional `read_real` function.
    /// Returns `None` if the plugin does not support auditing.
    pub fn read_real(&self) -> Option<String> {
        unsafe {
            let read_real_fn = (*self.vtable).read_real?;
            let raw = read_real_fn();
            if raw.is_null() {
                return None;
            }
            let result = CStr::from_ptr(raw).to_string_lossy().into_owned();
            ((*self.vtable).free_string)(raw);
            Some(result)
        }
    }
}

// ── Plugin host ────────────────────────────────────────────────────────────

/// Manages a collection of dynamically loaded plugins.
pub struct PluginHost {
    plugins: Vec<LoadedPlugin>,
}

impl PluginHost {
    /// Create an empty host.
    pub fn new() -> Self {
        Self { plugins: Vec::new() }
    }

    /// Discover and load all `.so` files from the default plugin directory:
    /// `~/.config/mirage/plugins/`.
    ///
    /// Logs warnings for individual plugins that fail to load rather than
    /// aborting the whole process — a bad plugin should not prevent the sandbox
    /// from running.
    pub fn load_from_default_dir(&mut self) {
        let Some(home) = std::env::var_os("HOME") else {
            eprintln!("[plugin-host] $HOME not set, skipping plugin discovery");
            return;
        };
        let plugin_dir = PathBuf::from(home)
            .join(".config")
            .join("mirage")
            .join("plugins");
        self.load_from_dir(&plugin_dir);
    }

    /// Discover and load all `.so` files from `dir`.
    pub fn load_from_dir(&mut self, dir: &Path) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // No plugin directory — silently skip
                return;
            }
            Err(e) => {
                eprintln!("[plugin-host] Cannot read plugin dir {:?}: {}", dir, e);
                return;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("so") {
                continue;
            }
            match self.load_plugin(&path) {
                Ok(plugin) => {
                    println!("[plugin-host] Loaded plugin '{}' from {:?}", plugin.name, path);
                    self.plugins.push(plugin);
                }
                Err(e) => {
                    eprintln!("[plugin-host] Failed to load {:?}: {:#}", path, e);
                }
            }
        }
    }

    /// Load a single plugin `.so` from `path`.
    pub fn load_plugin(&self, path: &Path) -> Result<LoadedPlugin> {
        // SAFETY: Loading arbitrary shared libraries is inherently unsafe.
        // We validate the ABI version immediately after loading.
        let lib = unsafe {
            Library::new(path)
                .with_context(|| format!("dlopen failed for {:?}", path))?
        };

        // Resolve the single required export
        let vtable_ptr: *const MiragePluginVtable = unsafe {
            let sym: Symbol<extern "C" fn() -> *const MiragePluginVtable> = lib
                .get(MIRAGE_PLUGIN_VTABLE_SYMBOL)
                .with_context(|| format!(
                    "Plugin {:?} does not export `mirage_plugin_vtable`",
                    path
                ))?;
            sym()
        };

        if vtable_ptr.is_null() {
            anyhow::bail!("Plugin {:?} returned a null vtable", path);
        }

        // SAFETY: We just confirmed the pointer is non-null and comes from the library.
        let vtable = unsafe { &*vtable_ptr };

        // Check ABI version
        if vtable.abi_version != 1 {
            anyhow::bail!(
                "Plugin {:?} has ABI version {} but host requires 1",
                path, vtable.abi_version
            );
        }

        // Extract name and description before we call init()
        let name = unsafe {
            if vtable.name.is_null() {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_owned()
            } else {
                CStr::from_ptr(vtable.name).to_string_lossy().into_owned()
            }
        };
        let description = unsafe {
            if vtable.description.is_null() {
                String::new()
            } else {
                CStr::from_ptr(vtable.description).to_string_lossy().into_owned()
            }
        };

        // Call init()
        let rc = (vtable.init)();
        if rc != MIRAGE_OK {
            anyhow::bail!("Plugin '{}' init() returned error code {}", name, rc);
        }

        Ok(LoadedPlugin {
            name,
            description,
            vtable: vtable_ptr,
            _lib: lib,
        })
    }

    /// Run `apply()` on every loaded plugin in order.
    /// Errors from individual plugins are logged but do not abort the others.
    pub fn apply_all(&self, profile: &Profile, tmp_dir: &Path) {
        for plugin in &self.plugins {
            if let Err(e) = plugin.apply(profile, tmp_dir) {
                eprintln!("[plugin-host] Plugin '{}' apply error: {:#}", plugin.name, e);
            }
        }
    }

    /// Returns a slice of all loaded plugins (for inspection / auditing).
    pub fn plugins(&self) -> &[LoadedPlugin] {
        &self.plugins
    }

    /// Returns true if no plugins are loaded.
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

impl Default for PluginHost {
    fn default() -> Self {
        Self::new()
    }
}
