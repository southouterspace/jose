//! [`RsMeansBenchmark`] — the RSMeans leaf of the [`CostBenchmark`] seam.
//!
//! A structurally-complete reference adapter: it prices a takeoff top-down by applying a
//! per-unit-of-measure published unit cost (the RSMeans assembly $/unit), region-adjusted, and
//! reports the variance against the bottom-up rollup. A production leaf would resolve real
//! `AssemblyCost.benchmark_unit_cost` rows; here the unit costs are an injected table so the seam
//! and the variance math are exercised without bundling a cost database.

use crate::domain::classification::Dimension;
use crate::domain::takeoff::TakeoffItem;
use crate::keys::UomKey;
use crate::ports::cost_benchmark::{BenchmarkResult, BenchmarkSource, CostBenchmark};
use std::collections::BTreeMap;

/// An RSMeans top-down validator over an injected published-unit-cost table.
#[derive(Clone, Debug, Default)]
pub struct RsMeansBenchmark {
    /// Published $/unit keyed by UOM code (e.g. `LF` → 1.85, `EA` → 0.42).
    unit_costs: BTreeMap<String, f64>,
    /// City-cost-index multiplier applied to every published unit cost.
    region_factor: f64,
}

impl RsMeansBenchmark {
    /// A benchmark with the given region factor and no unit costs yet.
    pub fn new(region_factor: f64) -> RsMeansBenchmark {
        RsMeansBenchmark {
            unit_costs: BTreeMap::new(),
            region_factor,
        }
    }

    /// Register a published unit cost for a UOM code.
    pub fn with_unit_cost(mut self, uom: UomKey, published: f64) -> RsMeansBenchmark {
        self.unit_costs.insert(uom.as_str().to_owned(), published);
        self
    }

    /// The region-adjusted published unit cost for a UOM, if known.
    fn unit_cost(&self, uom: &UomKey) -> Option<f64> {
        self.unit_costs
            .get(uom.as_str())
            .map(|c| c * self.region_factor)
    }
}

impl CostBenchmark for RsMeansBenchmark {
    fn source(&self) -> BenchmarkSource {
        BenchmarkSource::RsMeans
    }

    fn benchmark(&self, takeoff: &[TakeoffItem], bottom_up_total: f64) -> BenchmarkResult {
        // Top-down = Σ over installed (non-waste) quantities of region-adjusted published $/unit.
        // Waste takeoff is excluded: a benchmark prices installed work, not the saw loss.
        let top_down_total: f64 = takeoff
            .iter()
            .filter(|t| !t.waste_flag && t.uom.dimension != Dimension::LumpSum)
            .filter_map(|t| self.unit_cost(&t.uom.code).map(|c| c * t.quantity))
            .sum();

        let variance = if bottom_up_total.abs() > f64::EPSILON {
            (top_down_total - bottom_up_total) / bottom_up_total
        } else {
            0.0
        };

        BenchmarkResult {
            source: BenchmarkSource::RsMeans,
            top_down_total,
            per_code_totals: Vec::new(),
            variance,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::takeoff::DomainRef;
    use crate::keys::{CostCodeKey, TakeoffId};

    #[test]
    fn rsmeans_prices_installed_quantity_top_down() {
        // 100 LF installed at a published $1.85/LF, region factor 1.10 → $203.50 top-down.
        let bench = RsMeansBenchmark::new(1.10).with_unit_cost(UomKey::from("LF"), 1.85);
        let mut item = TakeoffItem::linear(
            TakeoffId(1),
            DomainRef::CutAssignment(1),
            38400, // 100 ft
            CostCodeKey::from("MF-06-11-00"),
        );
        item.quantity = 100.0;
        let result = bench.benchmark(&[item], 200.0);
        assert_eq!(result.source, BenchmarkSource::RsMeans);
        assert!((result.top_down_total - 203.5).abs() < 1e-9);
        // Variance vs a 200 bottom-up: +1.75%.
        assert!(result.within(0.05));
    }

    #[test]
    fn waste_takeoff_is_excluded_from_the_benchmark() {
        let bench = RsMeansBenchmark::new(1.0).with_unit_cost(UomKey::from("LF"), 2.0);
        let waste = TakeoffItem::linear(
            TakeoffId(1),
            DomainRef::KerfWaste(1),
            4,
            CostCodeKey::from("MF-06-11-00"),
        );
        let result = bench.benchmark(&[waste], 100.0);
        assert_eq!(result.top_down_total, 0.0);
    }
}
