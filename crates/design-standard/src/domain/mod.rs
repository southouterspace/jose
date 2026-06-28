//! The design-standard domain: the value objects the seam passes across the interface
//! (philosophy, section basis, factor stack, limit-state records, connection topology, sizing
//! I/O). Pure — no material conditionals live here.

pub mod connection;
pub mod factors;
pub mod limit_state;
pub mod philosophy;
pub mod section;
pub mod sizing;
