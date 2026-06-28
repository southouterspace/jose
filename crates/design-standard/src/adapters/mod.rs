//! Adapters: the material leaves that implement the [`DesignStandard`](crate::ports) port. The
//! wood leaf ([`NdsWood`](nds_wood::NdsWood)) is fully populated; the four others are
//! structurally complete stubs (every method present; lookup tables empty).

pub mod nds_wood;
pub mod stubs;
