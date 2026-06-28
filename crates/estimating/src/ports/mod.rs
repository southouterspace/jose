//! Ports — the traits this context needs from the outside.
//!
//! [`CostBenchmark`](cost_benchmark::CostBenchmark) is the top-down validator seam — the cost-side
//! mirror of `design-standard`'s `DesignStandard` strategy. The bottom-up rollup machinery depends
//! on this trait; concrete cost databases (RSMeans, ENR, supplier-index, historical-job) are
//! adapters behind it, so the core stays open to new cost models without edits.

pub mod cost_benchmark;
