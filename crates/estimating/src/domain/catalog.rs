//! The cost-side flyweights: [`ResourceRate`] (labor/equipment unit rates) and [`AssemblyCost`]
//! (RSMeans-style composite pay-item recipes). Both are looked up by key, never copied per use —
//! the cost-side analog of `StockSpec` / `SupplierSku`.

use crate::domain::classification::{CostType, UnitOfMeasure};
use crate::keys::{AssemblyKey, CostCodeKey, RateKey};
use materials::SkuKey;

/// Which flavor of productive resource a [`ResourceRate`] prices.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ResourceKind {
    /// A labor trade crew.
    Labor,
    /// A piece of equipment.
    Equipment,
}

/// A shared flyweight unit rate for a productive resource — the INTRINSIC published cost-per-unit.
/// The contextual quantity and any per-estimate burden/region override live on the consuming
/// [`ResourceLine`](crate::ResourceLine), so this row stays uncopied and undrifted.
#[derive(Clone, PartialEq, Debug)]
pub struct ResourceRate {
    /// Intrinsic flyweight key, e.g. `LAB-CARP-JOUR`.
    pub key: RateKey,
    /// `labor | equipment`.
    pub resource_kind: ResourceKind,
    /// Trade/discipline tag (carpenter, ironworker, operator). Material-agnostic.
    pub discipline: Option<String>,
    /// "USD".
    pub currency: String,
    /// Bare/base published rate per UOM (typically $/HR). Real decimal — never cents.
    pub rate: f64,
    /// Almost always HR; equipment may be HR or DAY.
    pub uom: UnitOfMeasure,
    /// Published fringe/burden/tax multiplier (e.g. 1.35); an estimate may OVERRIDE it on the line.
    pub base_burden_factor: Option<f64>,
    /// Published labor-market region the bare rate is keyed to.
    pub region: Option<String>,
    /// Snapshot timestamp; rates go stale.
    pub as_of: String,
}

/// One constituent of an [`AssemblyCost`]: a per-assembly-unit material/labor/equipment quantity
/// that expands into a [`MaterialLine`](crate::MaterialLine) or
/// [`ResourceLine`](crate::ResourceLine). Material-blind — a CFS or CMU assembly is just different
/// components, not a new type.
#[derive(Clone, PartialEq, Debug)]
pub struct AssemblyComponent {
    /// The economic category this component contributes.
    pub cost_type: CostType,
    /// → a material SKU, when this is a material component.
    pub sku_ref: Option<SkuKey>,
    /// → a resource rate, when this is a labor/equipment component.
    pub rate_ref: Option<RateKey>,
    /// Quantity per assembly unit.
    pub qty_per_unit: f64,
    /// The component's own unit of measure.
    pub uom: UnitOfMeasure,
}

/// How an assembly computes its cost — a data variant, not a new type.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AssemblyMethod {
    /// Crew output × resource rate (the RSMeans labor model).
    Productivity,
    /// Published all-in $/unit.
    UnitPrice,
    /// A flat lump sum.
    LumpSum,
}

/// A shared flyweight RSMeans-style assembly recipe: a composite pay-item that expands one
/// assembly quantity into its constituent lines via embedded productivity. The unit of top-down
/// benchmarking and pay-item granularity.
#[derive(Clone, PartialEq, Debug)]
pub struct AssemblyCost {
    /// Intrinsic flyweight key, e.g. `RSM-061110-STUDWALL-2x4-16OC`.
    pub key: AssemblyKey,
    /// `Wood Stud Wall, 2x4, 16" OC`.
    pub title: String,
    /// → the [`CostCode`](crate::CostCode) the assembly classifies under.
    pub cost_code_key: CostCodeKey,
    /// Pay-item unit the assembly is quantified in (LF of wall, SF of partition, EA).
    pub uom: UnitOfMeasure,
    /// Per-assembly-unit material/labor/equipment breakdown.
    pub components: Vec<AssemblyComponent>,
    /// Crew output (e.g. LF/HR) used with a `ResourceRate` to derive labor hours.
    pub productivity_rate: Option<f64>,
    /// How the assembly computes cost.
    pub method: AssemblyMethod,
    /// Published all-in $/unit for the row's region; feeds the top-down benchmark validator.
    pub benchmark_unit_cost: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_resource_rate_carries_an_intrinsic_burden() {
        let r = ResourceRate {
            key: RateKey::from("LAB-CARP-JOUR"),
            resource_kind: ResourceKind::Labor,
            discipline: Some("carpenter".to_owned()),
            currency: "USD".to_owned(),
            rate: 62.50,
            uom: UnitOfMeasure::hour(),
            base_burden_factor: Some(1.35),
            region: Some("national".to_owned()),
            as_of: "2026-01-01".to_owned(),
        };
        assert_eq!(r.resource_kind, ResourceKind::Labor);
        assert_eq!(r.base_burden_factor, Some(1.35));
    }
}
