//! Adapters — concrete implementations of this context's ports.
//!
//! [`rsmeans::RsMeansBenchmark`] is one leaf behind the [`CostBenchmark`](crate::CostBenchmark)
//! seam (the cost-side mirror of a `design-standard` material leaf). ENR / supplier-index /
//! historical-job adapters plug in beside it with zero core edits.

pub mod rsmeans;
