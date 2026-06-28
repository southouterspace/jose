//! [`TakeoffBuilder`] — the bottom-up handoff from the cut layer.
//!
//! Walks a [`cut_optimization::CutPlan`] into [`TakeoffItem`]s: one installed-quantity item per
//! cut, plus separate waste items for scrapped remainders ([`DomainRef::OffcutWaste`]) and saw kerf
//! ([`DomainRef::KerfWaste`]). This is the chain the estimate descends from — every dollar traces
//! to a cut, a stick, and ultimately the member that demanded it, so the takeoff is auditable
//! rather than a flat spreadsheet.

use crate::domain::takeoff::{DomainRef, TakeoffItem};
use crate::keys::{CostCodeKey, TakeoffId};
use cut_optimization::{CutPlan, RemainderFate};

/// Builds takeoff items from a solved cut plan. Mints sequential ids from a seed so a re-solve
/// produces stable, diffable ids.
#[derive(Clone, Copy, Debug)]
pub struct TakeoffBuilder {
    next_id: u128,
}

impl TakeoffBuilder {
    /// A builder minting ids from `seed`.
    pub fn new(seed: u128) -> TakeoffBuilder {
        TakeoffBuilder { next_id: seed }
    }

    fn mint(&mut self) -> TakeoffId {
        self.next_id += 1;
        TakeoffId(self.next_id)
    }

    /// Expand a cut plan into linear takeoff items under `cost_code`: installed cut lengths plus
    /// the paid-for waste (offcut scrap + kerf) that value engineering wants surfaced separately.
    pub fn from_cut_plan(&mut self, plan: &CutPlan, cost_code: &CostCodeKey) -> Vec<TakeoffItem> {
        let mut items = Vec::new();
        for assignment in &plan.assignments {
            // One installed-quantity item per cut, traced to the producing assignment.
            for cut in &assignment.cuts {
                let id = self.mint();
                items.push(TakeoffItem::linear(
                    id,
                    DomainRef::CutAssignment(assignment.id.raw()),
                    cut.length.raw(),
                    cost_code.clone(),
                ));
            }
            // Kerf loss is paid-for waste.
            if assignment.kerf_total.raw() > 0 {
                let id = self.mint();
                items.push(TakeoffItem::linear(
                    id,
                    DomainRef::KerfWaste(assignment.id.raw()),
                    assignment.kerf_total.raw(),
                    cost_code.clone(),
                ));
            }
            // A scrapped remainder is offcut waste; a pooled remainder is reusable supply, not waste.
            if assignment.remainder_fate == RemainderFate::Waste && assignment.remainder.raw() > 0 {
                let id = self.mint();
                items.push(TakeoffItem::linear(
                    id,
                    DomainRef::OffcutWaste(assignment.id.raw()),
                    assignment.remainder.raw(),
                    cost_code.clone(),
                ));
            }
        }
        items
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use building::MemberPlacementId;
    use cut_optimization::{
        CutEligibility, CutObjective, CutRequest, CuttingStockSolver, Demand, DemandLineKey,
        KerfSpec, OffcutId, OffcutPool, StockCatalog, StockOption,
    };
    use geometry_kernel::Tick;
    use materials::{Form, SkuKey, SpecKey};

    struct FlatCatalog;
    impl StockCatalog for FlatCatalog {
        fn stock_length(&self, _sku: &SkuKey) -> Option<Tick> {
            Some(Tick(3072)) // 8ft
        }
    }

    #[test]
    fn takeoff_items_descend_from_a_cut_plan() {
        // Solve a tiny cut plan: two 7ft cuts → two bought sticks, each with waste + kerf.
        let mut solver = CuttingStockSolver::new();
        let mut pool = OffcutPool::new(OffcutId(1), Tick(464));
        let demand = [Demand::new(
            DemandLineKey(1),
            Tick(2688),
            2,
            "stud".into(),
            SpecKey::from("SPF-STUD"),
            MemberPlacementId(1),
        )];
        let opts = [StockOption::buyable(
            SkuKey::from("HD-2x4-8"),
            SpecKey::from("SPF-STUD"),
        )];
        let kerf = KerfSpec::saw();
        let cat = FlatCatalog;
        let req = CutRequest {
            demand: &demand,
            options: &opts,
            kerf: &kerf,
            objective: CutObjective::min_waste(),
            eligibility: CutEligibility::classify(Form::Linear),
            catalog: &cat,
        };
        let plan = solver.solve(&req, &mut pool);
        assert_eq!(plan.assignments.len(), 2);

        let mut builder = TakeoffBuilder::new(0);
        let items = builder.from_cut_plan(&plan, &CostCodeKey::from("MF-06-11-00"));
        // Each stick contributes one installed cut + kerf + waste = 3 items × 2 sticks = 6.
        assert_eq!(items.len(), 6);
        assert!(items.iter().any(|i| i.waste_flag));
        // The installed cut traces back to its assignment.
        assert!(
            items
                .iter()
                .any(|i| matches!(i.source, DomainRef::CutAssignment(_)) && !i.waste_flag)
        );
    }
}
