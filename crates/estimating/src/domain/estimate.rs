//! [`Estimate`] — the project-level aggregate root.
//!
//! The persisted, versioned, top-level deliverable that owns the takeoff, all cost lines, the
//! markup/allowance stack, the cached rollup results, and the change-order log. Identity persists
//! across revisions (snapshotted to Postgres/Drizzle by `apps/api`). The terminus of the bottom-up
//! pipeline and the subject of top-down validation. The single entity in this layer.

use crate::domain::change_order::ChangeOrder;
use crate::domain::classification::CodeSystem;
use crate::domain::lines::{MaterialLine, ResourceLine};
use crate::domain::markup::{Allowance, Markup};
use crate::domain::rollup::RollupNode;
use crate::domain::takeoff::TakeoffItem;
use crate::keys::{EstimateId, ProjectRef};

/// The project-level estimate aggregate root.
#[derive(Clone, PartialEq, Debug)]
pub struct Estimate {
    /// Stable identity; survives revisions.
    pub id: EstimateId,
    /// 'Smith Residence — Framing Estimate'.
    pub name: String,
    /// → the project/model snapshot this estimate prices.
    pub project_ref: ProjectRef,
    /// Optimistic-lock / version; a re-solve or re-quote bumps this.
    pub revision: u32,
    /// "USD".
    pub currency: String,
    /// The cost-code classification this estimate is organized by.
    pub code_system: CodeSystem,
    /// The full bottom-up measured quantity set traced to domain objects.
    pub takeoff_items: Vec<TakeoffItem>,
    /// Priced material lines.
    pub material_lines: Vec<MaterialLine>,
    /// Priced labor/equipment lines (separated from material).
    pub resource_lines: Vec<ResourceLine>,
    /// Applied markup stack.
    pub markups: Vec<Markup>,
    /// Carried allowances/contingency.
    pub allowances: Vec<Allowance>,
    /// Approved/pending deltas since baseline.
    pub change_orders: Vec<ChangeOrder>,
    /// Cached `CostRollup` output per cost-code node.
    pub rollups: Vec<RollupNode>,
    /// Grand direct cost before markup/allowance.
    pub direct_cost_total: Option<f64>,
    /// Final bid total = direct + markups + allowances + approved change orders. Real USD.
    pub grand_total: f64,
    /// Snapshot timestamp; pricing freshness for the whole estimate.
    pub as_of: String,
    /// Fractional delta of `grand_total` vs the top-down benchmark total — the validator's verdict.
    pub benchmark_variance: Option<f64>,
}

impl Estimate {
    /// A fresh, empty estimate for a project, revision 1, organized by `code_system`.
    pub fn new(
        id: EstimateId,
        name: impl Into<String>,
        project_ref: ProjectRef,
        code_system: CodeSystem,
        as_of: impl Into<String>,
    ) -> Estimate {
        Estimate {
            id,
            name: name.into(),
            project_ref,
            revision: 1,
            currency: "USD".to_owned(),
            code_system,
            takeoff_items: Vec::new(),
            material_lines: Vec::new(),
            resource_lines: Vec::new(),
            markups: Vec::new(),
            allowances: Vec::new(),
            change_orders: Vec::new(),
            rollups: Vec::new(),
            direct_cost_total: None,
            grand_total: 0.0,
            as_of: as_of.into(),
            benchmark_variance: None,
        }
    }

    /// Σ of the approved change-order deltas — what the change log adds to the grand total.
    pub fn approved_change_total(&self) -> f64 {
        self.change_orders
            .iter()
            .filter(|c| c.is_in_effect())
            .map(|c| c.delta_total)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_estimate_starts_at_revision_one() {
        let e = Estimate::new(
            EstimateId(1),
            "Smith Residence — Framing",
            ProjectRef(7),
            CodeSystem::MasterFormat,
            "2026-06-28",
        );
        assert_eq!(e.revision, 1);
        assert_eq!(e.currency, "USD");
        assert_eq!(e.approved_change_total(), 0.0);
    }
}
