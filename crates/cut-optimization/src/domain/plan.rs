//! [`CutPlan`] — the solver's aggregate output, the single handoff to estimating.
//!
//! Every [`CutAssignment`], the de-duplicated pack-rounded [`BuyLine`] list, and the DERIVED
//! waste / material-quantity / cost rollups. A genuinely-missing type in the original solver,
//! which emitted a bare `CutAssignment[]` with no costed, quantity-rolled aggregate; estimating
//! needs one. All money/quantity/fraction fields are derived real per the base-unit invariant;
//! only `total_waste` (a length) stays in ticks.

use crate::domain::assignment::CutAssignment;
use crate::domain::objective::CutObjective;
use crate::keys::CutPlanId;
use geometry_kernel::Tick;
use materials::SkuKey;

/// A de-duplicated purchase line: a SKU and how many to buy after pack rounding. Reuses are
/// excluded (their cost was already booked). Inline VO — no identity, no reuse.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BuyLine {
    /// → [`materials::SupplierSku`] to buy.
    pub sku_ref: SkuKey,
    /// How many sticks to buy.
    pub count: u32,
    /// The count after rounding up to the SKU's pack size (≥ `count`).
    pub packs_rounded_to: u32,
}

/// A generic derived material rollup for the estimate: an amount and the unit that names it. The
/// unit comes from the spec's `quantity_basis` (board-ft for wood, lb/lineal-ft for steel, …), so
/// a new material contributes a quantity *unit*, not a schema edit. Replaces the former
/// wood-specific `board_feet` field.
#[derive(Clone, PartialEq, Debug)]
pub struct DerivedQuantity {
    /// The measured amount (derived real).
    pub value: f64,
    /// The unit name, from `StockSpec.quantity_basis`.
    pub unit: String,
}

/// The solver's aggregate output bundle — the bottom-up takeoff the estimating layer consumes.
#[derive(Clone, PartialEq, Debug)]
pub struct CutPlan {
    /// Identity of this solve result (a versionable snapshot).
    pub id: CutPlanId,
    /// Every stick plan, buys and reuses both.
    pub assignments: Vec<CutAssignment>,
    /// De-duplicated purchases only (reuses excluded).
    pub buy_list: Vec<BuyLine>,
    /// Σ remainder where the fate is waste. Linear, so ticks — the only summed length here.
    pub total_waste: Tick,
    /// Derived: `total_waste / Σ bought stock length`. The headline waste %, a ratio. `None` when
    /// nothing was bought.
    pub waste_fraction: Option<f64>,
    /// Generic derived material rollup for the estimate (board-ft / lb / lineal-ft / …).
    pub material_quantity: Option<DerivedQuantity>,
    /// Derived USD through each buy line's SupplierSku → PriceQuote. `None` until priced.
    pub rolled_cost: Option<f64>,
    /// What this plan optimized for — recorded for reproducibility.
    pub objective: CutObjective,
}

impl CutPlan {
    /// Total number of sticks purchased across the buy list (post-rounding).
    pub fn sticks_bought(&self) -> u32 {
        self.buy_list.iter().map(|b| b.packs_rounded_to).sum()
    }

    /// Number of stick plans that consumed an offcut rather than new stock.
    pub fn reuse_count(&self) -> usize {
        self.assignments
            .iter()
            .filter(|a| a.source.is_reuse())
            .count()
    }
}
