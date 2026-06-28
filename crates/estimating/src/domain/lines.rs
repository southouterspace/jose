//! The cost-line family: [`MaterialLine`], [`ResourceLine`], [`SubcontractLine`], unified as a
//! [`CostLine`] and aggregated by [`PayItem`].
//!
//! Material and labor/equipment are deliberately *separate* line types (correcting the draft's one
//! overloaded `lines` array): a material line prices a takeoff quantity against a SupplierSku /
//! PriceQuote; a resource line prices resource-time against a [`ResourceRate`] flyweight with any
//! per-estimate burden/region override applied *on the line*, not by mutating the shared rate.

use crate::domain::classification::CostType;
use crate::keys::{
    CostCodeKey, MaterialLineId, PayItemId, RateKey, ResourceLineId, TakeoffId, UomKey,
};
use materials::{PriceTier, SkuKey};

/// A handle to the `materials::PriceQuote` snapshot a material line priced against — the SKU whose
/// quote was used, plus its freshness stamp. References materials' single money source; never
/// inlines a price.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PriceQuoteRef {
    /// → the `materials::SupplierSku` whose `PriceQuote` was resolved.
    pub sku: SkuKey,
    /// The quote's snapshot timestamp (pricing freshness).
    pub as_of: String,
}

/// The pricing path selected by the carried `stockForm` discriminator — exactly the design-standard
/// caveat: linear → exact cut-list waste; sheet → nest yield; cast → formwork+volume; unit → count.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StockFormPath {
    /// Linear — buy quantity derives from exact cut-list remainder + kerf.
    Linear,
    /// Sheet — buy quantity from nest yield.
    Sheet,
    /// Cast — formwork + volume (bypasses the cut optimizer).
    Cast,
    /// Unit — a prefab count off the connection graph.
    Unit,
}

/// A single MATERIAL cost line: a takeoff quantity priced against a SupplierSku / PriceQuote, with
/// break-tier selection, packaging rounding, and waste applied. Material-agnostic via the carried
/// `stockForm` path.
#[derive(Clone, PartialEq, Debug)]
pub struct MaterialLine {
    /// Stable identity.
    pub id: MaterialLineId,
    /// → the [`TakeoffItem`](crate::TakeoffItem) measured quantity and its source.
    pub takeoff_ref: TakeoffId,
    /// → the buyable product (its unit-of-sale and pack size).
    pub sku_ref: SkuKey,
    /// → the PriceQuote snapshot used (resolved via the SKU). Real USD.
    pub price_ref: PriceQuoteRef,
    /// Carried (referenced) from `Stock.stockForm` — selects the pricing path.
    pub stock_form: StockFormPath,
    /// Installed (net) quantity before waste/rounding, from the takeoff. Derived-real.
    pub net_qty: f64,
    /// Purchase quantity AFTER waste allowance and packaging rounding — what gets ordered.
    pub buy_qty: f64,
    /// Which break tier `buy_qty` landed in (audit).
    pub applied_tier: Option<PriceTier>,
    /// Fractional allowance over net (e.g. 0.10), used ONLY for sheet/cast/unit or when no cut-list
    /// exists. For linear stock, `buy_qty` derives from the EXACT cut-list, not this factor.
    pub waste_factor: Option<f64>,
    /// Effective unit cost after tier selection. Real USD.
    pub unit_cost: f64,
    /// `buy_qty × unit_cost`. Derived-real; never cents.
    pub extended_cost: f64,
}

impl MaterialLine {
    /// `buy_qty × unit_cost` — the canonical extended cost, recomputed (never stored stale).
    pub fn compute_extended(&self) -> f64 {
        self.buy_qty * self.unit_cost
    }
}

/// A single LABOR or EQUIPMENT cost line: resource-time priced against a [`ResourceRate`]
/// flyweight, with any per-estimate burden/region override applied HERE (contextually) rather than
/// mutating the shared rate row.
#[derive(Clone, PartialEq, Debug)]
pub struct ResourceLine {
    /// Stable identity.
    pub id: ResourceLineId,
    /// → the [`TakeoffItem`](crate::TakeoffItem) install quantity; `None` for crew-day lump lines.
    pub takeoff_ref: Option<TakeoffId>,
    /// `kind=labor` or `kind=equipment`.
    pub cost_type: CostType,
    /// → the shared published [`ResourceRate`](crate::ResourceRate).
    pub rate_ref: RateKey,
    /// Resource quantity (typically HR, or DAY for equipment). Derived from productivity × install
    /// quantity, or entered directly.
    pub hours: f64,
    /// Per-estimate burden multiplier actually used; defaults to the rate's intrinsic factor.
    pub burden_factor_applied: Option<f64>,
    /// Per-estimate city-cost-index multiplier applied to the published region.
    pub region_factor: Option<f64>,
    /// Effective fully-loaded rate = base × burden × region. Real USD.
    pub unit_cost: f64,
    /// `hours × unit_cost`. Derived-real; never cents.
    pub extended_cost: f64,
    /// → the [`CostCode`](crate::CostCode) the resource cost rolls up under.
    pub cost_code_key: CostCodeKey,
}

impl ResourceLine {
    /// `hours × unit_cost` — the canonical extended cost.
    pub fn compute_extended(&self) -> f64 {
        self.hours * self.unit_cost
    }
}

/// A subcontract / lump-sum scope line (`CostType=subcontract|overhead`). Generalizes the cost-line
/// family so non-material, non-resource scope is representable.
#[derive(Clone, PartialEq, Debug)]
pub struct SubcontractLine {
    /// `subcontract` or `overhead`.
    pub cost_type: CostType,
    /// → the [`CostCode`](crate::CostCode) this scope rolls up under.
    pub cost_code_key: CostCodeKey,
    /// Human description of the scope.
    pub description: String,
    /// Derived real USD.
    pub extended_cost: f64,
}

/// The cost-line sum type: a [`PayItem`] aggregates `MaterialLine | ResourceLine | SubcontractLine`.
#[derive(Clone, PartialEq, Debug)]
pub enum CostLine {
    /// A material line.
    Material(MaterialLine),
    /// A labor/equipment line.
    Resource(ResourceLine),
    /// A subcontract/lump-sum line.
    Subcontract(SubcontractLine),
}

impl CostLine {
    /// The line's extended cost, regardless of family.
    pub fn extended_cost(&self) -> f64 {
        match self {
            CostLine::Material(l) => l.extended_cost,
            CostLine::Resource(l) => l.extended_cost,
            CostLine::Subcontract(l) => l.extended_cost,
        }
    }

    /// The economic category of the line.
    pub fn cost_type(&self) -> CostType {
        match self {
            CostLine::Material(_) => {
                CostType::markup_base(crate::domain::classification::CostKind::Material)
            }
            CostLine::Resource(l) => l.cost_type,
            CostLine::Subcontract(l) => l.cost_type,
        }
    }
}

/// A CostCode-coordinated, UOM-quantified estimate line that aggregates its constituent cost lines
/// into an extended cost. The unit a schedule-of-values / pay application reads.
#[derive(Clone, PartialEq, Debug)]
pub struct PayItem {
    /// Stable identity.
    pub id: PayItemId,
    /// → the [`CostCode`](crate::CostCode) coordinate.
    pub cost_code_key: CostCodeKey,
    /// The pay-item unit.
    pub uom: UomKey,
    /// Derived real; linear quantities trace to source ticks.
    pub quantity: f64,
    /// The constituent `MaterialLine | ResourceLine | SubcontractLine`.
    pub lines: Vec<CostLine>,
    /// Σ of the constituent extended costs. Derived real USD.
    pub extended_cost: f64,
}

impl PayItem {
    /// Σ of the constituent line extended costs.
    pub fn compute_extended(&self) -> f64 {
        self.lines.iter().map(CostLine::extended_cost).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::classification::CostKind;

    #[test]
    fn pay_item_sums_its_lines() {
        let sub = SubcontractLine {
            cost_type: CostType::markup_base(CostKind::Subcontract),
            cost_code_key: CostCodeKey::from("MF-06-11-00"),
            description: "framing sub".to_owned(),
            extended_cost: 1000.0,
        };
        let pay = PayItem {
            id: PayItemId(1),
            cost_code_key: CostCodeKey::from("MF-06-11-00"),
            uom: UomKey::from("LS"),
            quantity: 1.0,
            lines: vec![CostLine::Subcontract(sub)],
            extended_cost: 0.0,
        };
        assert_eq!(pay.compute_extended(), 1000.0);
    }
}
