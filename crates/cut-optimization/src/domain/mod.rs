//! The cut layer's value objects + the one entity (the [`OffcutPool`]).
//!
//! Everything here is a pure, material-blind value object except [`OffcutPool`], the sole
//! stateful entity. Linear lengths are integer [`Tick`](geometry_kernel::Tick)s; waste fractions,
//! utilization, and rolled cost are derived reals — no length is ever typed tick².

pub mod assignment;
pub mod demand;
pub mod eligibility;
pub mod kerf;
pub mod objective;
pub mod offcut;
pub mod option;
pub mod plan;
pub mod provenance;
