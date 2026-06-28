//! [`CostBenchmark`] — the top-down validator seam (Strategy «interface»).
//!
//! Given an estimate's quantities and cost codes, an implementation returns an independent
//! top-down cost (RSMeans assembly $/unit × quantity, ENR index, supplier index, or historical
//! job) to validate the bottom-up rollup. This keeps the estimate honest without coupling the core
//! to any one cost database — the cost-side analog of the design-standard material seam.

use crate::domain::rollup::RollupNode;
use crate::domain::takeoff::TakeoffItem;

/// Which benchmark database an implementation provides.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BenchmarkSource {
    /// RSMeans assembly unit costs.
    RsMeans,
    /// Engineering News-Record cost index.
    Enr,
    /// A supplier price index.
    SupplierIndex,
    /// A historical-job database.
    HistoricalJob,
    /// A project-custom source.
    Custom,
}

/// The independent top-down cost an implementation returns, and its variance vs the bottom-up
/// rollup.
#[derive(Clone, PartialEq, Debug)]
pub struct BenchmarkResult {
    /// Which source produced this result.
    pub source: BenchmarkSource,
    /// The independent top-down total in USD.
    pub top_down_total: f64,
    /// Per-cost-code top-down totals (for code-level comparison).
    pub per_code_totals: Vec<RollupNode>,
    /// Fractional delta vs the bottom-up total: `(top_down − bottom_up) / bottom_up`.
    pub variance: f64,
}

impl BenchmarkResult {
    /// Whether the variance is within an acceptable threshold (e.g. 0.15).
    pub fn within(&self, threshold: f64) -> bool {
        self.variance.abs() <= threshold
    }
}

/// The validator hook: an independent top-down cost for a set of takeoff quantities.
pub trait CostBenchmark {
    /// Which database this implementation provides.
    fn source(&self) -> BenchmarkSource;

    /// Return the top-down cost for `takeoff` (optionally region-adjusted) and its variance against
    /// the supplied `bottom_up_total`.
    fn benchmark(&self, takeoff: &[TakeoffItem], bottom_up_total: f64) -> BenchmarkResult;
}
