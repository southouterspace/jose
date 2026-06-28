//! ASCE 7 load combinations — **single-homed here** (pulled out of the design-standard core and
//! the solver). [`LoadCombination`] carries the full factored set but does **not** decide which
//! governs; that pick is delegated to the strategy seam downstream.

use crate::domain::sources::SourceKind;
use crate::keys::DesignPhilosophyRef;
use reference_data::CitationKey;

/// The load-duration class an NDS leaf maps to `CD`. Surfaced here as the *cause*; `CD` itself is
/// wood-only and computed by the strategy, never folded into any flyweight.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DurationClass {
    Permanent,
    TenYear,
    TwoMonth,
    SevenDay,
    TenMinute,
    Impact,
}

/// One factored term inside a [`LoadCombination`]: a load-source kind and its ASCE 7 factor.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CombinationTerm {
    /// The source this factor applies to.
    pub source_kind: SourceKind,
    /// ASCE 7 load factor (e.g. 1.2 for D under LRFD, 1.6 for L).
    pub factor: f64,
    /// Reduced factor when this source is a companion (e.g. 0.5L when wind/seismic leads).
    pub companion_factor: Option<f64>,
}

impl CombinationTerm {
    /// A primary (non-companion) factored term.
    pub fn new(source_kind: SourceKind, factor: f64) -> CombinationTerm {
        CombinationTerm {
            source_kind,
            factor,
            companion_factor: None,
        }
    }
}

/// An ASCE 7 load combination — a named, factored recipe over the source set. Carries the recipe
/// and its provenance but never selects the governing combination (the strategy does that).
#[derive(Clone, PartialEq, Debug)]
pub struct LoadCombination {
    /// Stable combo identifier, e.g. `LRFD-2` or `ASD-7`.
    pub combo_id: String,
    /// The ASD/LRFD philosophy this combo is built under — referenced, single-homed downstream.
    pub philosophy_ref: DesignPhilosophyRef,
    /// Ordered factored terms.
    pub terms: Vec<CombinationTerm>,
    /// The load-duration class (the cause of `CD`), if relevant.
    pub duration_class: Option<DurationClass>,
    /// True for net-uplift combos where wind reverses the dead-load sign.
    pub includes_uplift: Option<bool>,
    /// Provenance (ASCE 7 §2.3 LRFD / §2.4 ASD).
    pub source_ref: Option<CitationKey>,
}

impl LoadCombination {
    /// The factor applied to a given source in this combination (0.0 if the source is absent).
    pub fn factor_for(&self, kind: SourceKind) -> f64 {
        self.terms
            .iter()
            .find(|t| t.source_kind == kind)
            .map(|t| t.factor)
            .unwrap_or(0.0)
    }

    /// `1.2D + 1.6L + 0.5Lr` — the canonical LRFD gravity strength combination (ASCE 7 2.3.1-2).
    pub fn lrfd_gravity() -> LoadCombination {
        LoadCombination {
            combo_id: "LRFD-2".to_owned(),
            philosophy_ref: DesignPhilosophyRef::from("LRFD"),
            terms: vec![
                CombinationTerm::new(SourceKind::Dead, 1.2),
                CombinationTerm::new(SourceKind::Live, 1.6),
                CombinationTerm::new(SourceKind::RoofLive, 0.5),
            ],
            duration_class: None,
            includes_uplift: Some(false),
            source_ref: Some(CitationKey::book("ASCE 7").at("2.3.1")),
        }
    }

    /// `1.4D` — the dead-only LRFD combination (ASCE 7 2.3.1-1).
    pub fn lrfd_dead_only() -> LoadCombination {
        LoadCombination {
            combo_id: "LRFD-1".to_owned(),
            philosophy_ref: DesignPhilosophyRef::from("LRFD"),
            terms: vec![CombinationTerm::new(SourceKind::Dead, 1.4)],
            duration_class: Some(DurationClass::Permanent),
            includes_uplift: Some(false),
            source_ref: Some(CitationKey::book("ASCE 7").at("2.3.1")),
        }
    }

    /// `0.9D + 1.0W` — the LRFD net-uplift combination where wind opposes dead (ASCE 7 2.3.1-6).
    pub fn lrfd_uplift() -> LoadCombination {
        LoadCombination {
            combo_id: "LRFD-6".to_owned(),
            philosophy_ref: DesignPhilosophyRef::from("LRFD"),
            terms: vec![
                CombinationTerm::new(SourceKind::Dead, 0.9),
                CombinationTerm::new(SourceKind::Wind, 1.0),
            ],
            duration_class: Some(DurationClass::TenMinute),
            includes_uplift: Some(true),
            source_ref: Some(CitationKey::book("ASCE 7").at("2.3.1")),
        }
    }

    /// The standard ASCE 7 LRFD set used as the default candidate pool.
    pub fn lrfd_set() -> Vec<LoadCombination> {
        vec![
            Self::lrfd_dead_only(),
            Self::lrfd_gravity(),
            Self::lrfd_uplift(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gravity_combo_factors_each_source() {
        let c = LoadCombination::lrfd_gravity();
        assert_eq!(c.factor_for(SourceKind::Dead), 1.2);
        assert_eq!(c.factor_for(SourceKind::Live), 1.6);
        assert_eq!(c.factor_for(SourceKind::RoofLive), 0.5);
        assert_eq!(c.factor_for(SourceKind::Snow), 0.0); // absent
    }

    #[test]
    fn uplift_combo_is_flagged() {
        let c = LoadCombination::lrfd_uplift();
        assert_eq!(c.includes_uplift, Some(true));
        assert_eq!(c.factor_for(SourceKind::Dead), 0.9);
    }
}
