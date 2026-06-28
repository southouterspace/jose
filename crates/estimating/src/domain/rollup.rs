//! [`RollupNode`] — an immutable per-node total record.
//!
//! The computed subtotal at one [`CostCode`](crate::CostCode) (or [`CostType`](crate::CostType))
//! coordinate after direct cost + markup + allowance. The OUTPUT of
//! [`CostRollup`](crate::CostRollup) and the cached, snapshot-able total an
//! [`Estimate`](crate::Estimate) stores. A value object, not service state.

use crate::domain::classification::CostType;
use crate::keys::CostCodeKey;

/// An immutable per-node total: direct cost (incl. drawn allowance) + markup + carried allowance.
#[derive(Clone, PartialEq, Debug)]
pub struct RollupNode {
    /// → the [`CostCode`](crate::CostCode) this total is for; `None` for a pure cost-type-axis node.
    pub cost_code_key: Option<CostCodeKey>,
    /// Set when this node summarizes the economic-category axis instead of WBS.
    pub cost_type: Option<CostType>,
    /// Σ extended costs PLUS drawn allowance amounts at this node. Allowances are direct cost, so
    /// they sit in the markup base.
    pub direct_cost_subtotal: f64,
    /// Σ applied markups, each against its own base, in sequence order. Deterministic.
    pub markup_total: f64,
    /// Σ UNDRAWN allowance carried at this node (drawn allowance is already in the direct subtotal).
    pub allowance_total: f64,
    /// Σ material cost attributable to waste (offcut + kerf) — surfaced for value engineering.
    pub waste_cost: f64,
    /// Σ derived material weight at this node (lb) — a named running total alongside cost.
    pub weight: f64,
    /// `direct_cost_subtotal + markup_total + undrawn allowance_total`. Real USD.
    pub node_total: f64,
}

impl RollupNode {
    /// Recompute `node_total` from its parts — the one definition of the node total.
    pub fn rolled_total(&self) -> f64 {
        self.direct_cost_subtotal + self.markup_total + self.allowance_total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_total_is_direct_plus_markup_plus_carried() {
        let node = RollupNode {
            cost_code_key: Some(CostCodeKey::from("MF-06-11-00")),
            cost_type: None,
            direct_cost_subtotal: 1000.0,
            markup_total: 150.0,
            allowance_total: 200.0,
            waste_cost: 0.0,
            weight: 0.0,
            node_total: 1350.0,
        };
        assert_eq!(node.rolled_total(), 1350.0);
    }
}
