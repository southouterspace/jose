//! Material design-value flyweights.

use crate::citation::CitationKey;
use crate::keys::{DesignValueKey, StandardKey, StockForm};
use crate::registry::Flyweight;
use std::collections::BTreeMap;

/// SINGLE canonical source for a wood species+grade's intrinsic reference design values
/// (NDS / AWC). An immutable value object, resolved from a stock spec by key and shared by
/// every placement of that species+grade.
///
/// All values are reference (unadjusted) stresses in psi. The contextual NDS adjustment
/// stack (`CD`, `Cr`, `Cp`, …) is **not** here — it is applied per placement, which is what
/// fixed the original intrinsic/contextual flyweight violation (finding S1-B).
#[derive(Clone, PartialEq, Debug)]
pub struct MechanicalProperties {
    /// Bending, psi.
    pub fb: f64,
    /// Tension parallel to grain, psi.
    pub ft: f64,
    /// Compression parallel to grain, psi.
    pub fc: f64,
    /// Compression perpendicular to grain, psi.
    pub fc_perp: f64,
    /// Shear parallel to grain, psi.
    pub fv: f64,
    /// Modulus of elasticity, psi.
    pub e: f64,
    /// Minimum modulus of elasticity (stability), psi.
    pub e_min: f64,
    /// Specific gravity (unitless), if catalogued.
    pub g: Option<f64>,
    /// Provenance.
    pub source_ref: Option<CitationKey>,
}

/// The material-extensible seam for design-value catalogs: one row per `(material, standard)`
/// family, keyed by [`DesignValueKey`]. New materials are new rows, not new code.
#[derive(Clone, PartialEq, Debug)]
pub struct MaterialDesignValueTable {
    /// This row's key.
    pub key: DesignValueKey,
    /// Material family this row belongs to.
    pub material: MaterialFamilyKey,
    /// Design standard the values follow.
    pub standard: StandardKey,
    /// Pointer to the resolved reference values (e.g. the [`MechanicalProperties`] catalog).
    pub values_ref: DesignValueKey,
    /// Material-specific attributes (species, grade, gauge, …) as open key/value data.
    pub spec_attributes: BTreeMap<String, String>,
    /// Provenance.
    pub source_ref: Option<CitationKey>,
}

impl Flyweight for MaterialDesignValueTable {
    type Key = DesignValueKey;
    fn flyweight_key(&self) -> DesignValueKey {
        self.key.clone()
    }
}

/// Open registry key naming a material family (wood, cold-formed-steel, concrete, …),
/// replacing the closed `material` enum that would otherwise need editing per new material.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct MaterialFamilyKey {
    /// Family identifier (e.g. `wood`).
    pub key: String,
    /// Default design standard for this family.
    pub standard_key: StandardKey,
    /// Default stock form for this family.
    pub default_stock_form: StockForm,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn design_value_table_keys_itself() {
        let row = MaterialDesignValueTable {
            key: DesignValueKey::from("df-l-no2"),
            material: MaterialFamilyKey {
                key: "wood".into(),
                standard_key: StandardKey::from("nds"),
                default_stock_form: StockForm::from("dimensional-lumber"),
            },
            standard: StandardKey::from("nds"),
            values_ref: DesignValueKey::from("df-l-no2"),
            spec_attributes: BTreeMap::from([
                ("species".to_owned(), "Douglas Fir-Larch".to_owned()),
                ("grade".to_owned(), "No.2".to_owned()),
            ]),
            source_ref: Some(CitationKey::book("NDS").at("Table 4A")),
        };
        assert_eq!(row.flyweight_key(), DesignValueKey::from("df-l-no2"));
        assert_eq!(row.spec_attributes.get("grade").unwrap(), "No.2");
    }
}
