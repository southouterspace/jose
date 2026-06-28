//! [`SectionBasis`] — the strategy's resolved design-value + stiffness bundle for a spec, on the
//! correct basis (gross/effective/transformed/…).
//!
//! This is the crux of single-sourcing: for wood the bundle *references* the canonical
//! `MechanicalProperties` flyweight (by key) rather than re-declaring Fb/Ft/Fc/Fv/E a third time.

use crate::domain::philosophy::SectionBasisKind;
use crate::keys::MechanicalPropertiesKey;
use materials::SectionProperties;
use std::collections::BTreeMap;

/// The resolved characteristic strengths (psi, keyed by id) plus stiffness and the section
/// geometry on the declared basis. The strategy's `designValues()` output.
#[derive(Clone, PartialEq, Debug)]
pub struct SectionBasis {
    /// Must equal the leaf's declared section basis. The single owner of the basis discriminator.
    pub basis: SectionBasisKind,
    /// For wood: the key into the `MechanicalProperties` flyweight the strengths are read from.
    /// `None` for materials whose values are code constants (e.g. steel E = 29,000 ksi).
    pub design_values_ref: Option<MechanicalPropertiesKey>,
    /// Resolved characteristic strengths on the basis, keyed by id (`Fb`/`Ft`/`Fc`/`FcPerp`/`Fv`
    /// for wood; `Fy`/`Fu` for steel; …). Canonical unit psi.
    pub strengths: BTreeMap<String, f64>,
    /// Modulus of elasticity injected into the statics engine (psi).
    pub e: f64,
    /// Stability/buckling modulus (NDS `Emin`); `None` where the material uses another formulation.
    pub e_min: Option<f64>,
    /// `A`, `S`, `I` on the declared basis (the strategy reduces gross for effective/cracked).
    pub section: SectionProperties,
}

impl SectionBasis {
    /// A resolved strength by id (e.g. `"Fb"`), if present.
    pub fn strength(&self, id: &str) -> Option<f64> {
        self.strengths.get(id).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use materials::Axis;

    #[test]
    fn strengths_are_keyed_lookups() {
        let mut strengths = BTreeMap::new();
        strengths.insert("Fb".to_owned(), 900.0);
        strengths.insert("Fv".to_owned(), 180.0);
        let basis = SectionBasis {
            basis: SectionBasisKind::Gross,
            design_values_ref: Some(MechanicalPropertiesKey::from("df-l-no2")),
            strengths,
            e: 1.6e6,
            e_min: Some(580_000.0),
            section: SectionProperties::rectangular(1.5, 3.5, Axis::Strong, 1),
        };
        assert_eq!(basis.strength("Fb"), Some(900.0));
        assert_eq!(basis.strength("Fy"), None);
        assert_eq!(basis.basis, SectionBasisKind::Gross);
    }
}
