//! The materials domain: pure data model with no outward dependencies (geometry-kernel and
//! reference-data are the two shared kernels it leans on). No ports/adapters exist at this
//! phase — the context is pure compute with no I/O of its own.

pub mod cut;
pub mod discriminators;
pub mod geometry;
pub mod pricing;
pub mod stock;
pub mod weight;
