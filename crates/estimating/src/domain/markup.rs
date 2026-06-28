//! [`Markup`] and [`Allowance`] — the add-ons and carried budgets that sit atop direct cost.
//!
//! Order + scope make the markup stack auditable rather than a single fudge %. Each markup declares
//! its OWN [`AppliesToBase`], so markup-on-markup is deterministic data, not baked into the schema
//! (OH → profit → bond → tax). An allowance is a DIRECT COST, not a markup (v1.0.1): its *drawn*
//! amount rolls into the direct-cost subtotal and is markup-eligible; the undrawn remainder is
//! carried until scope is modeled.

use crate::domain::classification::CostKind;
use crate::keys::{AllowanceId, CostCodeKey, MarkupId};

/// The economic add-on a [`Markup`] represents.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MarkupKind {
    /// Indirect overhead.
    Overhead,
    /// Contractor profit.
    Profit,
    /// General conditions.
    GeneralConditions,
    /// Payment/performance bond.
    Bond,
    /// Insurance.
    Insurance,
    /// Contingency-as-percent.
    Contingency,
    /// Sales/use tax (a markup with a jurisdiction-configurable base).
    Tax,
}

/// Whether a markup is a percentage of a base or a flat amount.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MarkupMethod {
    /// Percent of the declared base.
    Percent,
    /// A flat dollar amount.
    Fixed,
}

/// The base a markup applies to — each markup declares its own, so order is data and markup-on-
/// markup is deterministic.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppliesToBase {
    /// Direct cost only.
    DirectCost,
    /// Direct cost plus drawn allowances.
    DirectCostPlusAllowances,
    /// Compound on prior markups in sequence (the running subtotal).
    RunningSubtotal,
    /// Material cost only.
    MaterialCostOnly,
}

/// A proportional or fixed add-on applied at a rollup node. Scoped so different cost types or codes
/// can carry different markups. Computed into [`RollupNode::markup_total`](crate::RollupNode), never
/// folded silently into unit costs.
#[derive(Clone, PartialEq, Debug)]
pub struct Markup {
    /// Stable identity.
    pub id: MarkupId,
    /// `overhead | profit | … | tax`.
    pub kind: MarkupKind,
    /// `percent | fixed`.
    pub method: MarkupMethod,
    /// Fraction when `method=percent` (0.10 = 10%).
    pub rate: Option<f64>,
    /// Flat dollar amount when `method=fixed`. Real USD.
    pub amount: Option<f64>,
    /// Optional filter: restrict this markup to one cost kind. `None` = all.
    pub applies_to_cost_kind: Option<CostKind>,
    /// Optional WBS scope: applies only under this code subtree. `None` = whole estimate.
    pub applies_to_cost_code_key: Option<CostCodeKey>,
    /// Application order within a node.
    pub sequence: i32,
    /// The base this markup computes against — makes the stack deterministic.
    pub applies_to_base: AppliesToBase,
}

impl Markup {
    /// Compute this markup's dollar contribution given the candidate bases at its node. `Percent`
    /// markups read the base named by [`Markup::applies_to_base`]; `Fixed` markups ignore it.
    pub fn compute(
        &self,
        direct_cost: f64,
        drawn_allowances: f64,
        running_subtotal: f64,
        material_cost: f64,
    ) -> f64 {
        match self.method {
            MarkupMethod::Fixed => self.amount.unwrap_or(0.0),
            MarkupMethod::Percent => {
                let base = match self.applies_to_base {
                    AppliesToBase::DirectCost => direct_cost,
                    AppliesToBase::DirectCostPlusAllowances => direct_cost + drawn_allowances,
                    AppliesToBase::RunningSubtotal => running_subtotal,
                    AppliesToBase::MaterialCostOnly => material_cost,
                };
                self.rate.unwrap_or(0.0) * base
            }
        }
    }

    /// A percent-of-direct-cost markup.
    pub fn percent_of_direct(id: MarkupId, kind: MarkupKind, rate: f64, sequence: i32) -> Markup {
        Markup {
            id,
            kind,
            method: MarkupMethod::Percent,
            rate: Some(rate),
            amount: None,
            applies_to_cost_kind: None,
            applies_to_cost_code_key: None,
            sequence,
            applies_to_base: AppliesToBase::DirectCost,
        }
    }
}

/// The kind of carried budget an [`Allowance`] represents.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AllowanceKind {
    /// Scope not yet modeled/quantified.
    ScopeAllowance,
    /// Design contingency.
    DesignContingency,
    /// Estimating contingency.
    EstimatingContingency,
}

/// How an allowance is sized.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AllowanceMethod {
    /// A flat lump amount.
    Lump,
    /// A fraction of a base.
    PercentOfBase,
}

/// A carried budget placeholder for scope not yet modeled, plus contingency. A DIRECT COST: its
/// drawn amount rolls into the direct-cost subtotal and is markup-eligible. The undrawn remainder
/// is carried until scope is modeled and converted to real lines.
#[derive(Clone, PartialEq, Debug)]
pub struct Allowance {
    /// Stable identity.
    pub id: AllowanceId,
    /// `scopeAllowance | designContingency | estimatingContingency`.
    pub kind: AllowanceKind,
    /// 'Connection hardware allowance', 'Unmodeled blocking'.
    pub title: String,
    /// → the [`CostCode`](crate::CostCode) the allowance sits under.
    pub cost_code_key: Option<CostCodeKey>,
    /// `lump | percentOfBase`.
    pub method: AllowanceMethod,
    /// Lump amount when `method=lump`. Real USD.
    pub amount: Option<f64>,
    /// Fraction of base when `method=percentOfBase`.
    pub rate: Option<f64>,
    /// Running amount converted to real lines as scope is modeled; remaining = amount − drawn.
    pub drawn_down: Option<f64>,
}

impl Allowance {
    /// The drawn amount (already inside direct cost), defaulting to zero.
    pub fn drawn(&self, base: f64) -> f64 {
        self.drawn_down.unwrap_or(0.0).min(self.total(base))
    }

    /// The total budgeted amount of this allowance against a base.
    pub fn total(&self, base: f64) -> f64 {
        match self.method {
            AllowanceMethod::Lump => self.amount.unwrap_or(0.0),
            AllowanceMethod::PercentOfBase => self.rate.unwrap_or(0.0) * base,
        }
    }

    /// The undrawn remainder still carried (not yet a real line).
    pub fn undrawn(&self, base: f64) -> f64 {
        (self.total(base) - self.drawn(base)).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_markup_reads_its_declared_base() {
        let m = Markup::percent_of_direct(MarkupId(1), MarkupKind::Overhead, 0.10, 0);
        // 10% of direct cost (1000) = 100, ignoring other bases.
        assert_eq!(m.compute(1000.0, 500.0, 9999.0, 800.0), 100.0);
    }

    #[test]
    fn running_subtotal_compounds() {
        let mut m = Markup::percent_of_direct(MarkupId(2), MarkupKind::Profit, 0.05, 1);
        m.applies_to_base = AppliesToBase::RunningSubtotal;
        // 5% of the running subtotal (1100), i.e. compounding on the prior overhead.
        assert_eq!(m.compute(1000.0, 0.0, 1100.0, 0.0), 55.0);
    }

    #[test]
    fn allowance_splits_drawn_and_undrawn() {
        let a = Allowance {
            id: AllowanceId(1),
            kind: AllowanceKind::ScopeAllowance,
            title: "hardware".to_owned(),
            cost_code_key: None,
            method: AllowanceMethod::Lump,
            amount: Some(2000.0),
            rate: None,
            drawn_down: Some(500.0),
        };
        assert_eq!(a.drawn(0.0), 500.0);
        assert_eq!(a.undrawn(0.0), 1500.0);
    }
}
