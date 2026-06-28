//! [`Stock`] (the immutable raw member) and its [`StockSpec`] flyweight.
//!
//! `Stock` holds identity, discriminators, geometry and a flyweight spec pointer — but **no**
//! mechanical design values (those resolve through the `DesignStandard` seam) and **no** inline
//! price (that lives on [`SupplierSku`](crate::SupplierSku) → [`PriceQuote`](crate::PriceQuote)).

use crate::domain::discriminators::{MaterialClass, StockForm};
use crate::domain::geometry::Dimensions;
use crate::keys::{SkuKey, SpecKey, StockId};
use geometry_kernel::{Tick, Transform};
use reference_data::{CitationKey, DesignValueKey, Flyweight};

/// Service/processing treatment of stock, extensible per material.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Treatment {
    None,
    PressureTreated,
    FireRetardant,
    Galvanized,
    EpoxyCoated,
}

/// The shared flyweight every [`Stock`] points at: resolves material + designation + condition
/// to an **opaque design-value key** plus density and allowable lengths. Holds no mechanical
/// design values inline — only a `design_value_ref` the `DesignStandard` strategy resolves — so
/// a wood, steel or concrete spec is the *same* type (the single biggest S1-A fix).
#[derive(Clone, PartialEq, Debug)]
pub struct StockSpec {
    /// Opaque flyweight key, looked up never copied per instance.
    pub key: SpecKey,
    /// Material discriminator (matches the owning [`Stock`]); the strategy uses it to pick a leaf.
    pub material: MaterialClass,
    /// Material-neutral product designation (wood species+grade; steel section call-out).
    pub designation: String,
    /// Service/processing condition (wood moisture S-DRY/KD; steel coating). Material-blind.
    pub condition: Option<String>,
    /// Treatment, if any.
    pub treatment: Option<Treatment>,
    /// Opaque key the `DesignStandard` seam resolves to design values. **No** adjustment factors
    /// live here — they are contextual, on the placement (S1-B).
    pub design_value_ref: DesignValueKey,
    /// Self-weight density at the stated condition (lb/ft³), if catalogued. Feeds `Weight`.
    pub density: Option<f64>,
    /// Manufacturable/stock length set in ticks (linear stock only); constrains the cut optimizer.
    pub allowable_lengths: Option<Vec<Tick>>,
    /// Provenance into the reference library (NDS/AISI/ACI table, IRC species table).
    pub source_ref: Option<CitationKey>,
}

impl Flyweight for StockSpec {
    type Key = SpecKey;
    fn flyweight_key(&self) -> SpecKey {
        self.key.clone()
    }
}

/// The immutable original member — a length/sheet/cast of raw stock, material-agnostic. The
/// provenance root for any [`Piece`](crate::Piece) cut from it.
#[derive(Clone, PartialEq, Debug)]
pub struct Stock {
    /// Stable identity; survives transforms and placement. Provenance root.
    pub id: StockId,
    /// Material discriminator — routes to the `DesignStandard` leaf.
    pub material: MaterialClass,
    /// Form discriminator — decides cut vs nest vs formwork output path.
    pub stock_form: StockForm,
    /// Pointer into the [`StockSpec`] flyweight catalog. Mechanicals resolve via spec→seam.
    pub spec_ref: SpecKey,
    /// Parametric geometry (length is the free variable).
    pub dimensions: Dimensions,
    /// Keys into the [`SupplierSku`](crate::SupplierSku) catalog; many suppliers per member.
    pub supplier_refs: Vec<SkuKey>,
    /// As-printed mill/heat stamp. Per-instance contextual data, kept out of the spec flyweight.
    pub grade_stamp: Option<String>,
    /// Where this instance sits in 3D; absent for catalog/stock-only members.
    pub placement: Option<Transform>,
    /// Optimistic-lock / change-tracking revision.
    pub revision: Option<u32>,
}

impl Stock {
    /// A catalog stock member (no placement, no supplier refs yet).
    pub fn new(
        id: StockId,
        material: MaterialClass,
        stock_form: StockForm,
        spec_ref: SpecKey,
        dimensions: Dimensions,
    ) -> Stock {
        Stock {
            id,
            material,
            stock_form,
            spec_ref,
            dimensions,
            supplier_refs: Vec::new(),
            grade_stamp: None,
            placement: None,
            revision: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::discriminators::Form;
    use reference_data::{MaterialFamilyKey, Registry, StandardKey};

    fn wood() -> MaterialClass {
        MaterialClass {
            material: MaterialFamilyKey {
                key: "wood".into(),
                standard_key: StandardKey::from("nds"),
                default_stock_form: reference_data::StockForm::from("dimensional-lumber"),
            },
            design_standard_ref: StandardKey::from("nds"),
        }
    }

    fn spec() -> StockSpec {
        StockSpec {
            key: SpecKey::from("SPF-STUD-SDRY"),
            material: wood(),
            designation: "SPF / Stud".to_owned(),
            condition: Some("S-DRY".to_owned()),
            treatment: Some(Treatment::None),
            design_value_ref: DesignValueKey::from("spf-stud"),
            density: Some(31.2),
            allowable_lengths: Some(vec![Tick(3072), Tick(3348), Tick(3732)]),
            source_ref: Some(CitationKey::book("NDS").at("Table 4A")),
        }
    }

    #[test]
    fn spec_is_a_flyweight() {
        let reg = Registry::index([spec()]);
        assert_eq!(reg.len(), 1);
        assert_eq!(
            reg.get(&SpecKey::from("SPF-STUD-SDRY"))
                .unwrap()
                .designation,
            "SPF / Stud"
        );
    }

    #[test]
    fn stock_points_at_spec_without_mechanicals() {
        let s = Stock::new(
            StockId(1),
            wood(),
            StockForm::for_form(Form::Linear),
            SpecKey::from("SPF-STUD-SDRY"),
            Dimensions::rectangular("2x4", Tick(3072), Tick(112), Tick(48)),
        );
        assert_eq!(s.spec_ref, SpecKey::from("SPF-STUD-SDRY"));
        assert!(s.placement.is_none());
    }
}
