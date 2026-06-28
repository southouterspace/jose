//! [`CuttingStockSolver`] — the coupled 1D cutting-stock solver.
//!
//! Takes the full `Demand[]` (pre-filtered by [`CutEligibility`]) + the catalog `StockOption[]` +
//! the [`OffcutPool`] + a [`KerfSpec`] + a [`CutObjective`], and chooses stock mix AND cut pattern
//! together. The two coupled problems — which sticks to buy and how to cut them — are solved
//! jointly; that coupling is what minimizes waste. The pool is consulted before opening new stock,
//! so leftovers become supply.
//!
//! The method here is **first-fit-decreasing + offcut pool** (the schema's `ffdPlusPool`): cuts
//! are placed longest-first into the tightest open stick, then an offcut, then a fresh bought
//! stick. It is material-blind throughout — it never inspects species or gauge, only spec-match,
//! length, kerf, and the eligibility verdict.

use crate::domain::assignment::{CutAssignment, CutLine, RemainderFate, StickSource};
use crate::domain::demand::Demand;
use crate::domain::eligibility::CutEligibility;
use crate::domain::kerf::KerfSpec;
use crate::domain::objective::CutObjective;
use crate::domain::offcut::{Offcut, OffcutPool};
use crate::domain::option::StockOption;
use crate::domain::plan::{BuyLine, CutPlan};
use crate::keys::{AssignmentId, CutPlanId, OffcutId};
use crate::ports::stock_catalog::StockCatalog;
use geometry_kernel::Tick;
use materials::{SkuKey, SpecKey};
use std::collections::BTreeMap;

/// The cutting-stock solver. Owns only the monotonic id counters for the artifacts it mints; all
/// canonical state lives on the [`OffcutPool`] and the returned [`CutPlan`].
#[derive(Clone, Debug, Default)]
pub struct CuttingStockSolver {
    next_assignment: u128,
    next_offcut: u128,
    next_plan: u128,
}

/// The inputs to one [`CuttingStockSolver::solve`] call, bundled into a request value the way the
/// design-standard seam bundles a `SizingQuery`. The stateful [`OffcutPool`] is passed separately
/// because it is *in/out* — consulted and mutated — whereas everything here is read-only input.
pub struct CutRequest<'a> {
    /// The COMPLETE demand list, already filtered to one [`CutEligibility`] form. Partial lists
    /// pack worse.
    pub demand: &'a [Demand],
    /// The buyable constraint set from the supplier layer. The solver uses nothing outside it.
    pub options: &'a [StockOption],
    /// Per-cut loss, counted into every cut plus end-trim per new stick.
    pub kerf: &'a KerfSpec,
    /// The min-waste-then-cost dial + method, recorded on the plan for reproducibility.
    pub objective: CutObjective,
    /// The required `stockForm=linear` gate (gotcha #3); an ineligible batch is routed away empty.
    pub eligibility: CutEligibility,
    /// The port the solver reads SKU length / pack / price through (flyweight discipline).
    pub catalog: &'a dyn StockCatalog,
}

impl std::fmt::Debug for CutRequest<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The catalog is a trait object (no Debug); summarize the request shape instead.
        f.debug_struct("CutRequest")
            .field("demand", &self.demand)
            .field("options", &self.options)
            .field("kerf", &self.kerf)
            .field("objective", &self.objective)
            .field("eligibility", &self.eligibility)
            .field("catalog", &"<dyn StockCatalog>")
            .finish()
    }
}

/// The immutable per-solve context threaded through placement: the buyable constraint set, the
/// kerf, and the SKU-fact port. Bundled so the placement verb stays a tidy `(cut, ctx)` call.
struct Batch<'a> {
    buyable: BTreeMap<SpecKey, SkuKey>,
    kerf: &'a KerfSpec,
    catalog: &'a dyn StockCatalog,
}

/// A stick being packed during the solve, before it is frozen into a [`CutAssignment`].
#[derive(Clone, Debug)]
struct OpenStick {
    id: AssignmentId,
    spec: SpecKey,
    source: StickSource,
    usable: Tick,
    remaining: Tick,
    origin_sku: SkuKey,
    cuts: Vec<CutLine>,
}

impl CuttingStockSolver {
    /// A fresh solver.
    pub fn new() -> CuttingStockSolver {
        CuttingStockSolver::default()
    }

    fn mint_assignment(&mut self) -> AssignmentId {
        self.next_assignment += 1;
        AssignmentId(self.next_assignment)
    }

    fn mint_offcut(&mut self) -> OffcutId {
        self.next_offcut += 1;
        OffcutId(self.next_offcut)
    }

    fn mint_plan(&mut self) -> CutPlanId {
        self.next_plan += 1;
        CutPlanId(self.next_plan)
    }

    /// Solve the cutting-stock problem and emit a [`CutPlan`].
    ///
    /// `eligibility` is the **required gate** (gotcha #3): if the batch's stock form is not
    /// linear the solver places nothing and returns an empty, routed-away plan. The caller is
    /// expected to pass demand already filtered to one eligible form.
    pub fn solve(&mut self, req: &CutRequest, pool: &mut OffcutPool) -> CutPlan {
        let plan_id = self.mint_plan();
        if !req.eligibility.eligible {
            // Ineligible demand is routed away (nest / formwork / direct) — nothing to cut.
            return empty_plan(plan_id, req.objective);
        }

        // One representative buyable option per spec — the constraint set the solver may draw from.
        let buyable: BTreeMap<SpecKey, SkuKey> = req
            .options
            .iter()
            .filter(|o| o.buyable)
            .map(|o| (o.spec_ref.clone(), o.sku_ref.clone()))
            .collect();
        let ctx = Batch {
            buyable,
            kerf: req.kerf,
            catalog: req.catalog,
        };

        // Expand every demand line into individual cuts, longest first (FFD). Drop any whose spec
        // has no buyable option (cannot be sourced).
        let mut cuts: Vec<(Tick, &Demand)> = req
            .demand
            .iter()
            .filter(|d| ctx.buyable.contains_key(&d.spec_ref))
            .flat_map(|d| std::iter::repeat_n((d.length, d), d.qty as usize))
            .collect();
        cuts.sort_by(|a, b| b.0.raw().cmp(&a.0.raw()));

        let mut open: Vec<OpenStick> = Vec::new();
        for (length, d) in cuts {
            self.place_cut(length, d, &ctx, pool, &mut open);
        }

        self.finalize(plan_id, open, pool, req.objective, req.catalog)
    }

    /// Place one cut: into an open stick, else an offcut, else a fresh bought stick.
    fn place_cut(
        &mut self,
        length: Tick,
        d: &Demand,
        ctx: &Batch,
        pool: &mut OffcutPool,
        open: &mut Vec<OpenStick>,
    ) {
        let cost = length.raw() + ctx.kerf.kerf.raw(); // each cut consumes its length plus one kerf

        // (a) tightest open stick of this spec with capacity.
        let fit = open
            .iter_mut()
            .filter(|s| s.spec == d.spec_ref && s.remaining.raw() >= cost)
            .min_by_key(|s| s.remaining.raw());
        if let Some(stick) = fit {
            stick.remaining = Tick(stick.remaining.raw() - cost);
            stick.cuts.push(CutLine {
                demand_ref: d.line_key,
                length,
            });
            return;
        }

        // (b) an offcut from the pool (consulted before new stock).
        if let Some(idx) = pool.best_fit(Tick(cost), &d.spec_ref) {
            let offcut = pool.take(idx);
            let id = self.mint_assignment();
            let mut stick = OpenStick {
                id,
                spec: d.spec_ref.clone(),
                source: StickSource::Reused(offcut.id),
                usable: offcut.length,
                remaining: offcut.length,
                origin_sku: offcut.origin_sku_ref.clone(),
                cuts: Vec::new(),
            };
            stick.remaining = Tick(stick.remaining.raw() - cost);
            stick.cuts.push(CutLine {
                demand_ref: d.line_key,
                length,
            });
            open.push(stick);
            return;
        }

        // (c) a fresh bought stick. spec is guaranteed buyable (filtered upstream).
        let sku = ctx.buyable[&d.spec_ref].clone();
        let stock_len = ctx.catalog.stock_length(&sku).unwrap_or(Tick::ZERO);
        let usable = Tick((stock_len.raw() - ctx.kerf.end_trim.raw()).max(0));
        let id = self.mint_assignment();
        let mut stick = OpenStick {
            id,
            spec: d.spec_ref.clone(),
            source: StickSource::Bought(sku.clone()),
            usable,
            remaining: usable,
            origin_sku: sku,
            cuts: Vec::new(),
        };
        // Even if a single cut overflows the stick, place it (the demand must be satisfied); the
        // remainder simply clamps to zero. A real ILP pass would flag an oversize demand here.
        stick.remaining = Tick((stick.remaining.raw() - cost).max(0));
        stick.cuts.push(CutLine {
            demand_ref: d.line_key,
            length,
        });
        open.push(stick);
    }

    /// Freeze the open sticks into assignments, push pooled remainders, and roll up the aggregate.
    fn finalize(
        &mut self,
        plan_id: CutPlanId,
        open: Vec<OpenStick>,
        pool: &mut OffcutPool,
        objective: CutObjective,
        catalog: &dyn StockCatalog,
    ) -> CutPlan {
        let mut assignments = Vec::with_capacity(open.len());
        let mut total_waste = Tick::ZERO;
        let mut bought_length = Tick::ZERO;
        let mut buy_counts: BTreeMap<SkuKey, u32> = BTreeMap::new();

        for stick in open {
            let kerf_total =
                Tick(stick.usable.raw() - stick.remaining.raw() - cut_total(&stick.cuts).raw());
            let remainder = stick.remaining;
            let pooled = pool.worth_pooling(remainder);
            let fate = if pooled {
                RemainderFate::Pooled
            } else {
                RemainderFate::Waste
            };

            if let StickSource::Bought(sku) = &stick.source {
                *buy_counts.entry(sku.clone()).or_insert(0) += 1;
                bought_length = Tick(bought_length.raw() + stick.usable.raw());
            }
            if pooled && remainder.raw() > 0 {
                let offcut_id = self.mint_offcut();
                pool.push(Offcut {
                    id: offcut_id,
                    length: remainder,
                    spec_ref: stick.spec.clone(),
                    parent_assignment_ref: stick.id,
                    origin_sku_ref: stick.origin_sku.clone(),
                });
            } else {
                total_waste = Tick(total_waste.raw() + remainder.raw());
            }

            assignments.push(CutAssignment {
                id: stick.id,
                source: stick.source,
                cuts: stick.cuts,
                kerf_total,
                remainder,
                remainder_fate: fate,
            });
        }

        let buy_list: Vec<BuyLine> = buy_counts
            .into_iter()
            .map(|(sku_ref, count)| {
                let pack = catalog.pack_size(&sku_ref).max(1);
                let packs_rounded_to = count.div_ceil(pack) * pack;
                BuyLine {
                    sku_ref,
                    count,
                    packs_rounded_to,
                }
            })
            .collect();

        let waste_fraction = if bought_length.raw() > 0 {
            Some(total_waste.to_inches() / bought_length.to_inches())
        } else {
            None
        };

        let rolled_cost = roll_cost(&buy_list, catalog);

        CutPlan {
            id: plan_id,
            assignments,
            buy_list,
            total_waste,
            waste_fraction,
            material_quantity: None, // named by StockSpec.quantity_basis — the estimating layer fills it.
            rolled_cost,
            objective,
        }
    }
}

/// Σ of the cut lengths on a stick.
fn cut_total(cuts: &[CutLine]) -> Tick {
    Tick(cuts.iter().map(|c| c.length.raw()).sum())
}

/// Roll the buy list to a USD total, or `None` if any line is unpriced (the catalog opts out).
fn roll_cost(buy_list: &[BuyLine], catalog: &dyn StockCatalog) -> Option<f64> {
    if buy_list.is_empty() {
        return None;
    }
    let mut total = 0.0;
    for line in buy_list {
        let unit = catalog.unit_price(&line.sku_ref, line.packs_rounded_to)?;
        total += unit * line.packs_rounded_to as f64;
    }
    Some(total)
}

/// An empty plan for an ineligible / non-linear batch routed away from the optimizer.
fn empty_plan(id: CutPlanId, objective: CutObjective) -> CutPlan {
    CutPlan {
        id,
        assignments: Vec::new(),
        buy_list: Vec::new(),
        total_waste: Tick::ZERO,
        waste_fraction: None,
        material_quantity: None,
        rolled_cost: None,
        objective,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::demand::CutRole;
    use crate::domain::offcut::Offcut;
    use crate::keys::DemandLineKey;
    use building::MemberPlacementId;
    use materials::Form;

    /// A tiny in-memory catalog: every SKU is an 8ft (3072-tick) stick at a flat price.
    struct TestCatalog {
        length: Tick,
        price: Option<f64>,
        pack: u32,
    }

    impl StockCatalog for TestCatalog {
        fn stock_length(&self, _sku: &SkuKey) -> Option<Tick> {
            Some(self.length)
        }
        fn pack_size(&self, _sku: &SkuKey) -> u32 {
            self.pack
        }
        fn unit_price(&self, _sku: &SkuKey, _count: u32) -> Option<f64> {
            self.price
        }
    }

    fn demand(line: u128, len: i32, qty: u32) -> Demand {
        Demand::new(
            DemandLineKey(line),
            Tick(len),
            qty,
            CutRole::from("stud"),
            SpecKey::from("SPF-STUD"),
            MemberPlacementId(line),
        )
    }

    fn options() -> Vec<StockOption> {
        vec![StockOption::buyable(
            SkuKey::from("HD-2x4-8"),
            SpecKey::from("SPF-STUD"),
        )]
    }

    /// Build a linear, waste-first request over the given demand/options/kerf/catalog.
    fn req<'a>(
        demand: &'a [Demand],
        options: &'a [StockOption],
        kerf: &'a KerfSpec,
        catalog: &'a dyn StockCatalog,
    ) -> CutRequest<'a> {
        CutRequest {
            demand,
            options,
            kerf,
            objective: CutObjective::min_waste(),
            eligibility: CutEligibility::classify(Form::Linear),
            catalog,
        }
    }

    #[test]
    fn two_short_cuts_share_one_eight_foot_stick() {
        // Two 3ft cuts (1152 ticks each) fit on one 8ft stick (3072) with kerf to spare.
        let mut solver = CuttingStockSolver::new();
        let mut pool = OffcutPool::new(OffcutId(1), Tick(464));
        let cat = TestCatalog {
            length: Tick(3072),
            price: Some(4.50),
            pack: 1,
        };
        let kerf = KerfSpec::saw();
        let demand = [demand(1, 1152, 2)];
        let opts = options();
        let plan = solver.solve(&req(&demand, &opts, &kerf, &cat), &mut pool);
        assert_eq!(plan.assignments.len(), 1, "both cuts pack onto one stick");
        assert_eq!(plan.assignments[0].cuts.len(), 2);
        assert_eq!(plan.sticks_bought(), 1);
        // Cost rolled: one 8ft stick at $4.50.
        assert_eq!(plan.rolled_cost, Some(4.50));
        // Remainder 3072 - 2*1152 - 2*4 = 760 ≥ 464 → pooled, not waste.
        assert_eq!(plan.total_waste, Tick::ZERO);
        assert_eq!(plan.assignments[0].remainder_fate, RemainderFate::Pooled);
    }

    #[test]
    fn long_cuts_open_separate_sticks_and_buy_two() {
        // Two 7ft cuts (2688) cannot share an 8ft stick → two bought sticks.
        let mut solver = CuttingStockSolver::new();
        let mut pool = OffcutPool::new(OffcutId(1), Tick(464));
        let cat = TestCatalog {
            length: Tick(3072),
            price: Some(4.50),
            pack: 1,
        };
        let kerf = KerfSpec::saw();
        let demand = [demand(1, 2688, 2)];
        let opts = options();
        let plan = solver.solve(&req(&demand, &opts, &kerf, &cat), &mut pool);
        assert_eq!(plan.assignments.len(), 2);
        assert_eq!(plan.sticks_bought(), 2);
        assert_eq!(plan.rolled_cost, Some(9.00));
    }

    #[test]
    fn a_seeded_offcut_is_reused_before_buying() {
        // Seed the pool (project scope) with a 4ft offcut; a single 3ft cut should reuse it,
        // buying nothing.
        let mut solver = CuttingStockSolver::new();
        let mut pool = OffcutPool::new(OffcutId(1), Tick(464));
        pool.push(Offcut {
            id: OffcutId(99),
            length: Tick(1536), // 4ft
            spec_ref: SpecKey::from("SPF-STUD"),
            parent_assignment_ref: AssignmentId(7),
            origin_sku_ref: SkuKey::from("HD-2x4-8"),
        });
        let cat = TestCatalog {
            length: Tick(3072),
            price: Some(4.50),
            pack: 1,
        };
        let kerf = KerfSpec::saw();
        let demand = [demand(1, 1152, 1)];
        let opts = options();
        let plan = solver.solve(&req(&demand, &opts, &kerf, &cat), &mut pool);
        assert_eq!(plan.assignments.len(), 1);
        assert_eq!(plan.reuse_count(), 1, "the offcut was reused");
        assert!(plan.buy_list.is_empty(), "no new stock bought");
        assert_eq!(plan.rolled_cost, None, "a pure reuse has no purchase cost");
    }

    #[test]
    fn pack_rounding_rounds_up_to_the_pack() {
        // Three bought sticks at pack size 2 → round up to 4.
        let mut solver = CuttingStockSolver::new();
        let mut pool = OffcutPool::new(OffcutId(1), Tick(464));
        let cat = TestCatalog {
            length: Tick(3072),
            price: Some(4.0),
            pack: 2,
        };
        let kerf = KerfSpec::saw();
        let demand = [demand(1, 2688, 3)];
        let opts = options();
        let plan = solver.solve(&req(&demand, &opts, &kerf, &cat), &mut pool);
        assert_eq!(plan.buy_list.len(), 1);
        assert_eq!(plan.buy_list[0].count, 3);
        assert_eq!(plan.buy_list[0].packs_rounded_to, 4);
    }

    #[test]
    fn an_ineligible_batch_is_routed_away_empty() {
        let mut solver = CuttingStockSolver::new();
        let mut pool = OffcutPool::new(OffcutId(1), Tick(464));
        let cat = TestCatalog {
            length: Tick(3072),
            price: Some(4.50),
            pack: 1,
        };
        let kerf = KerfSpec::saw();
        let demand = [demand(1, 1152, 2)];
        let opts = options();
        let request = CutRequest {
            demand: &demand,
            options: &opts,
            kerf: &kerf,
            objective: CutObjective::min_waste(),
            eligibility: CutEligibility::classify(Form::Cast), // concrete — bypasses the optimizer
            catalog: &cat,
        };
        let plan = solver.solve(&request, &mut pool);
        assert!(plan.assignments.is_empty());
        assert!(plan.buy_list.is_empty());
    }
}
