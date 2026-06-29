use anyhow::Result;
use mirage_protocol::{Profile, SignalKind, SignalValue};
use mirage_sandbox::SandboxHandle;

pub mod bluetooth;
pub mod dns;
pub mod geoclue;
pub mod hostname;
pub mod locale;
pub mod machine_id;
pub mod network;
pub mod timezone;
pub mod webrtc;
pub mod wifi;

pub trait IdentityProvider {
    fn signal_kind(&self) -> SignalKind;
    fn real_value(&self) -> Result<SignalValue>; // for audit
    fn projected_value(&self, profile: &Profile) -> Result<SignalValue>;
    fn apply(&self, ns: &SandboxHandle, profile: &Profile) -> Result<()>;
}
