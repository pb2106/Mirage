//! Example Mirage plugin — writes a fake `/proc/version` inside the sandbox.
//!
//! Build with:
//!   cargo build --release --manifest-path examples/plugin-example/Cargo.toml
//!
//! Install with:
//!   mkdir -p ~/.config/mirage/plugins
//!   cp target/release/libmirage_example_plugin.so ~/.config/mirage/plugins/

use mirage_protocol::{
    MiragePluginResult, MiragePluginVtable, MIRAGE_ERR_GENERAL, MIRAGE_OK, Profile,
};
use std::ffi::{CStr, CString, c_char};

// ── Vtable strings ─────────────────────────────────────────────────────────

static NAME: &[u8] = b"example-kernel-spoofer\0";
static DESCRIPTION: &[u8] = b"Writes a fake /proc/version string into the sandbox\0";

// ── Hook implementations ───────────────────────────────────────────────────

extern "C" fn plugin_init() -> MiragePluginResult {
    eprintln!("[example-plugin] init");
    MIRAGE_OK
}

extern "C" fn plugin_destroy() {
    eprintln!("[example-plugin] destroy");
}

extern "C" fn plugin_apply(
    profile_json: *const c_char,
    tmp_dir: *const c_char,
) -> MiragePluginResult {
    // SAFETY: Both pointers come from the host which guarantees null-terminated UTF-8.
    let profile: Profile = unsafe {
        let json = CStr::from_ptr(profile_json).to_string_lossy();
        match serde_json::from_str(&json) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[example-plugin] Failed to parse profile JSON: {}", e);
                return MIRAGE_ERR_GENERAL;
            }
        }
    };

    let tmp_dir_str = unsafe { CStr::from_ptr(tmp_dir).to_string_lossy() };
    let fake_version_path = std::path::Path::new(tmp_dir_str.as_ref()).join("proc_version");

    // Build a plausible-looking kernel version string
    let kernel_str = format!(
        "Linux version 6.1.0-generic (gcc version 12.3.0) \
         #1 SMP PREEMPT_DYNAMIC hostname={}\n",
        profile.hostname.as_deref().unwrap_or("localhost")
    );

    if let Err(e) = std::fs::write(&fake_version_path, &kernel_str) {
        eprintln!("[example-plugin] Failed to write fake /proc/version: {}", e);
        return MIRAGE_ERR_GENERAL;
    }

    eprintln!("[example-plugin] Wrote fake /proc/version → {:?}", fake_version_path);
    MIRAGE_OK
}

extern "C" fn plugin_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        // SAFETY: ptr was allocated by CString::into_raw in this same library.
        unsafe { drop(CString::from_raw(ptr)) };
    }
}

// ── Vtable ─────────────────────────────────────────────────────────────────

static VTABLE: MiragePluginVtable = MiragePluginVtable {
    abi_version: 1,
    name: NAME.as_ptr() as *const c_char,
    description: DESCRIPTION.as_ptr() as *const c_char,
    init: plugin_init,
    destroy: plugin_destroy,
    apply: plugin_apply,
    read_real: None,
    free_string: plugin_free_string,
};

/// The single required export.
#[unsafe(no_mangle)]
pub extern "C" fn mirage_plugin_vtable() -> *const MiragePluginVtable {
    &VTABLE
}
