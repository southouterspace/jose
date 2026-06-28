//! [`CostRollup`] — the aggregation engine (a pure verb).
//!
//! Walks the cost lines to a direct-cost subtotal, then applies the [`Markup`] stack and
//! [`Allowance`]s deterministically to emit [`RollupNode`]s and a grand total. Each markup computes
//! against its OWN declared base in sequence order, so markup-on-markup (OH → profit → bond → tax)
//! is reproducible data, not baked-in order. The service stores no totals itself — it DERIVES
//! [`RollupNode`] values from the lines, so the result is snapshot-able and diff-able.

use crate::domain::classification::CostKind;
use crate::domain::lines::{MaterialLine, ResourceLine, SubcontractLine};
use crate::domain::markup::{Allowance, Markup};
use crate::domain::rollup::RollupNode;
use std::collections::BTreeMap;

/// The summarization axis: WBS (cost code) vs economic category (cost type), or both.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GroupBy {
    /// Group direct cost by [`CostCode`](crate::CostCode) (the WBS axis) — the default.
    #[default]
    CostCode,
    /// Group direct cost by [`CostType`](crate::CostType) (the economic-category axis).
    CostType,
    /// Both axes.
    Both,
}

/// The read-only inputs to a rollup: the cost lines, the markup stack, the carried allowances, and
/// the summarization axis.
#[derive(Clone, Copy, Debug)]
pub struct RollupInput<'a> {
    /// Priced material lines.
    pub material_lines: &'a [MaterialLine],
    /// Priced labor/equipment lines.
    pub resource_lines: &'a [ResourceLine],
    /// Subcontract / lump-sum lines.
    pub subcontract_lines: &'a [SubcontractLine],
    /// The applied markup stack.
    pub markups: &'a [Markup],
    /// Carried allowances/contingency.
    pub allowances: &'a [Allowance],
    /// The summarization axis.
    pub group_by: GroupBy,
}

/// The rollup result: the per-code (or per-type) direct-cost nodes, the estimate-wide grand node
/// (carrying the full markup + allowance stack), and the grand total.
#[derive(Clone, PartialEq, Debug)]
pub struct RollupOutput {
    /// Per-coordinate direct-cost nodes (the WBS or economic-category breakdown).
    pub nodes: Vec<RollupNode>,
    /// The single estimate-wide node carrying direct + markup + carried allowance.
    pub grand: RollupNode,
    /// The final all-in total in USD.
    pub grand_total: f64,
}

/// The aggregation engine. A stateless verb.
#[derive(Clone, Copy, Debug, Default)]
pub struct CostRollup;

impl CostRollup {
    /// A fresh rollup engine.
    pub fn new() -> CostRollup {
        CostRollup
    }

    /// Roll the lines up, apply the markup/allowance stack, and emit the nodes + grand total.
    pub fn roll(&self, input: &RollupInput) -> RollupOutput {
        let material_cost: f64 = input.material_lines.iter().map(|l| l.extended_cost).sum();
        let resource_cost: f64 = input.resource_lines.iter().map(|l| l.extended_cost).sum();
        let sub_cost: f64 = input
            .subcontract_lines
            .iter()
            .map(|l| l.extended_cost)
            .sum();
        let direct_from_lines = material_cost + resource_cost + sub_cost;

        // Allowances are direct cost: their DRAWN amount enters the markup base; the UNDRAWN
        // remainder is carried (not double-counted). The percent base is the lines' direct cost.
        let drawn_allowances: f64 = input
            .allowances
            .iter()
            .map(|a| a.drawn(direct_from_lines))
            .sum();
        let undrawn_allowances: f64 = input
            .allowances
            .iter()
            .map(|a| a.undrawn(direct_from_lines))
            .sum();
        let direct_cost_subtotal = direct_from_lines + drawn_allowances;

        // Apply the markup stack in sequence order, compounding the running subtotal so a markup
        // that declares `RunningSubtotal` sees the prior markups.
        let mut markups: Vec<&Markup> = input.markups.iter().collect();
        markups.sort_by_key(|m| m.sequence);
        let mut markup_total = 0.0;
        let mut running_subtotal = direct_cost_subtotal;
        for m in markups {
            let contribution = m.compute(
                direct_cost_subtotal,
                drawn_allowances,
                running_subtotal,
                material_cost,
            );
            markup_total += contribution;
            running_subtotal += contribution;
        }

        let waste_cost = self.waste_cost(input);
        let grand = RollupNode {
            cost_code_key: None,
            cost_type: None,
            direct_cost_subtotal,
            markup_total,
            allowance_total: undrawn_allowances,
            waste_cost,
            weight: 0.0,
            node_total: direct_cost_subtotal + markup_total + undrawn_allowances,
        };

        let nodes = match input.group_by {
            GroupBy::CostType => self.nodes_by_cost_type(input),
            // CostCode and Both both produce the WBS breakdown; the type axis is folded into Both
            // at the grand node for this scaffold.
            _ => self.nodes_by_cost_code(input),
        };

        RollupOutput {
            grand_total: grand.node_total,
            grand,
            nodes,
        }
    }

    /// Σ material cost attributable to a waste takeoff — surfaced separately for value engineering.
    /// Computed from the material lines whose [`MaterialLine::waste_factor`] applies; the exact
    /// cut-list waste is already inside `buy_qty`, so this is the allowance-style waste only.
    fn waste_cost(&self, input: &RollupInput) -> f64 {
        input
            .material_lines
            .iter()
            .filter_map(|l| l.waste_factor.map(|w| l.extended_cost * w / (1.0 + w)))
            .sum()
    }

    /// Per-cost-code direct-cost nodes (the WBS breakdown). Markup is applied only at the grand
    /// node in this scaffold; per-code markup scoping is a later refinement.
    fn nodes_by_cost_code(&self, input: &RollupInput) -> Vec<RollupNode> {
        let mut by_code: BTreeMap<String, f64> = BTreeMap::new();
        for l in input.resource_lines {
            *by_code
                .entry(l.cost_code_key.as_str().to_owned())
                .or_insert(0.0) += l.extended_cost;
        }
        for l in input.subcontract_lines {
            *by_code
                .entry(l.cost_code_key.as_str().to_owned())
                .or_insert(0.0) += l.extended_cost;
        }
        by_code
            .into_iter()
            .map(|(code, direct)| RollupNode {
                cost_code_key: Some(crate::keys::CostCodeKey::from(code)),
                cost_type: None,
                direct_cost_subtotal: direct,
                markup_total: 0.0,
                allowance_total: 0.0,
                waste_cost: 0.0,
                weight: 0.0,
                node_total: direct,
            })
            .collect()
    }

    /// Per-cost-type direct-cost nodes (the economic-category breakdown).
    fn nodes_by_cost_type(&self, input: &RollupInput) -> Vec<RollupNode> {
        let mut by_kind: BTreeMap<&'static str, (CostKind, f64)> = BTreeMap::new();
        let material: f64 = input.material_lines.iter().map(|l| l.extended_cost).sum();
        if material != 0.0 {
            by_kind.insert("material", (CostKind::Material, material));
        }
        for l in input.resource_lines {
            let label = match l.cost_type.kind {
                CostKind::Equipment => "equipment",
                _ => "labor",
            };
            by_kind.entry(label).or_insert((l.cost_type.kind, 0.0)).1 += l.extended_cost;
        }
        for l in input.subcontract_lines {
            let label = match l.cost_type.kind {
                CostKind::Overhead => "overhead",
                _ => "subcontract",
            };
            by_kind.entry(label).or_insert((l.cost_type.kind, 0.0)).1 += l.extended_cost;
        }
        by_kind
            .into_values()
            .map(|(kind, direct)| RollupNode {
                cost_code_key: None,
                cost_type: Some(crate::domain::classification::CostType::markup_base(kind)),
                direct_cost_subtotal: direct,
                markup_total: 0.0,
                allowance_total: 0.0,
                waste_cost: 0.0,
                weight: 0.0,
                node_total: direct,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::classification::{CostKind, CostType, UnitOfMeasure};
    use crate::domain::lines::{PriceQuoteRef, StockFormPath};
    use crate::domain::markup::{AppliesToBase, MarkupKind, MarkupMethod};
    use crate::keys::{
        AllowanceId, CostCodeKey, MarkupId, MaterialLineId, ResourceLineId, TakeoffId,
    };
    use materials::SkuKey;

    fn material_line(extended: f64) -> MaterialLine {
        MaterialLine {
            id: MaterialLineId(1),
            takeoff_ref: TakeoffId(1),
            sku_ref: SkuKey::from("HD-2x4-8"),
            price_ref: PriceQuoteRef {
                sku: SkuKey::from("HD-2x4-8"),
                as_of: "2026-01-01".to_owned(),
            },
            stock_form: StockFormPath::Linear,
            net_qty: 100.0,
            buy_qty: 110.0,
            applied_tier: None,
            waste_factor: None,
            unit_cost: extended / 110.0,
            extended_cost: extended,
        }
    }

    fn resource_line(extended: f64) -> ResourceLine {
        ResourceLine {
            id: ResourceLineId(1),
            takeoff_ref: None,
            cost_type: CostType::markup_base(CostKind::Labor),
            rate_ref: crate::keys::RateKey::from("LAB-CARP-JOUR"),
            hours: 10.0,
            burden_factor_applied: None,
            region_factor: None,
            unit_cost: extended / 10.0,
            extended_cost: extended,
            cost_code_key: CostCodeKey::from("MF-06-11-00"),
        }
    }

    #[test]
    fn markup_stack_compounds_deterministically() {
        // Direct = 1000 material + 500 labor = 1500.
        // OH (seq 0): 10% of direct = 150 → running 1650.
        // Profit (seq 1): 5% of running subtotal = 82.5 → running 1732.5.
        let materials = [material_line(1000.0)];
        let resources = [resource_line(500.0)];
        let oh = Markup::percent_of_direct(MarkupId(1), MarkupKind::Overhead, 0.10, 0);
        let mut profit = Markup::percent_of_direct(MarkupId(2), MarkupKind::Profit, 0.05, 1);
        profit.applies_to_base = AppliesToBase::RunningSubtotal;
        let input = RollupInput {
            material_lines: &materials,
            resource_lines: &resources,
            subcontract_lines: &[],
            markups: &[oh, profit],
            allowances: &[],
            group_by: GroupBy::CostType,
        };
        let out = CostRollup::new().roll(&input);
        assert_eq!(out.grand.direct_cost_subtotal, 1500.0);
        assert!((out.grand.markup_total - 232.5).abs() < 1e-9);
        assert!((out.grand_total - 1732.5).abs() < 1e-9);
        // Two economic-category nodes: material + labor.
        assert_eq!(out.nodes.len(), 2);
    }

    #[test]
    fn drawn_allowance_enters_the_markup_base_undrawn_is_carried() {
        // Direct lines = 1000. A 400 lump allowance, 100 drawn.
        // Markup base = 1000 + 100 drawn = 1100; 10% OH = 110.
        // Undrawn 300 carried on top. Total = 1100 + 110 + 300 = 1510.
        let materials = [material_line(1000.0)];
        let allowance = Allowance {
            id: AllowanceId(1),
            kind: crate::domain::markup::AllowanceKind::ScopeAllowance,
            title: "hardware".to_owned(),
            cost_code_key: None,
            method: crate::domain::markup::AllowanceMethod::Lump,
            amount: Some(400.0),
            rate: None,
            drawn_down: Some(100.0),
        };
        let mut oh = Markup::percent_of_direct(MarkupId(1), MarkupKind::Overhead, 0.10, 0);
        oh.method = MarkupMethod::Percent;
        let input = RollupInput {
            material_lines: &materials,
            resource_lines: &[],
            subcontract_lines: &[],
            markups: &[oh],
            allowances: &[allowance],
            group_by: GroupBy::CostType,
        };
        let out = CostRollup::new().roll(&input);
        assert!((out.grand.direct_cost_subtotal - 1100.0).abs() < 1e-9);
        assert!((out.grand.markup_total - 110.0).abs() < 1e-9);
        assert!((out.grand.allowance_total - 300.0).abs() < 1e-9);
        assert!((out.grand_total - 1510.0).abs() < 1e-9);
    }

    #[test]
    fn uom_is_a_value_object() {
        // A guard that the classification re-export resolves through the application module.
        assert_eq!(UnitOfMeasure::linear_feet().from_ticks(3840), Some(10.0));
    }
}
