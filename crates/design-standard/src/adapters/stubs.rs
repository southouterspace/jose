//! The four stubbed-but-structurally-complete leaves: [`AisiCfs`], [`AiscSteel`],
//! [`AciConcrete`], [`TmsMasonry`].
//!
//! Every [`DesignStandard`] method is present and typed; only the code lookup tables are
//! unpopulated (`strategy_check` returns `None`). Adding any of them requires **zero** core edits —
//! the proof of the forward extensibility requirement. The hybrid end state (wood walls + steel
//! beams + concrete footing) runs each member through its own leaf under one arbiter.

use crate::domain::connection::{Connection, ConnectionCapacity};
use crate::domain::factors::{FactorContext, ModificationFactor};
use crate::domain::limit_state::LimitStateId;
use crate::domain::philosophy::{DesignCode, DesignPhilosophy, MaterialKind, SectionBasisKind};
use crate::domain::section::SectionBasis;
use crate::domain::sizing::SizingQuery;
use crate::keys::DesignStandardId;
use crate::ports::design_standard::DesignStandard;
use loads_analysis::LoadCombination;

/// Build a section basis from `(name, psi)` strength pairs, a modulus, and the candidate section.
fn section_basis(
    basis: SectionBasisKind,
    strengths: &[(&str, f64)],
    e: f64,
    query: &SizingQuery,
) -> SectionBasis {
    SectionBasis {
        basis,
        design_values_ref: None, // code constants, not a flyweight
        strengths: strengths
            .iter()
            .map(|(k, v)| ((*k).to_owned(), *v))
            .collect(),
        e,
        e_min: None,
        section: query.section,
    }
}

/// Pick the max-factored combination (steel/concrete strength design).
fn max_factored(combos: &[LoadCombination]) -> Option<LoadCombination> {
    combos
        .iter()
        .max_by(|a, b| {
            let sa: f64 = a.terms.iter().map(|t| t.factor).sum();
            let sb: f64 = b.terms.iter().map(|t| t.factor).sum();
            sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned()
}

/// Map an open mode name into the limit-state vocabulary.
fn modes(names: &[&str]) -> Vec<LimitStateId> {
    names.iter().map(|n| LimitStateId::from(*n)).collect()
}

/// Cold-formed steel leaf (AISI S100). Effective-section design; the cheapest next material.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AisiCfs {
    pub id: DesignStandardId,
}

impl Default for AisiCfs {
    fn default() -> Self {
        AisiCfs {
            id: DesignStandardId::from("AISI-S100-16"),
        }
    }
}

impl DesignStandard for AisiCfs {
    fn id(&self) -> DesignStandardId {
        self.id.clone()
    }
    fn code(&self) -> DesignCode {
        DesignCode::Aisi
    }
    fn material(&self) -> MaterialKind {
        MaterialKind::Steel
    }
    fn section_basis(&self) -> SectionBasisKind {
        SectionBasisKind::Effective
    }
    fn philosophy(&self) -> DesignPhilosophy {
        DesignPhilosophy::LRFD
    }
    fn design_values(&self, query: &SizingQuery) -> SectionBasis {
        // Fy 50 ksi, Fu 65 ksi, E 29,000 ksi — converted ×1000 to psi at the boundary.
        section_basis(
            SectionBasisKind::Effective,
            &[("Fy", 50_000.0), ("Fu", 65_000.0)],
            29.0e6,
            query,
        )
    }
    fn modification_factors(&self, _ctx: &FactorContext) -> Vec<ModificationFactor> {
        vec![ModificationFactor::contextual("phi", 0.9)]
    }
    fn extra_limit_states(&self) -> Vec<LimitStateId> {
        modes(&[
            "localBuckling",
            "distortionalBuckling",
            "LTB",
            "webCrippling",
        ])
    }
    fn connection_capacity(&self, conn: &Connection) -> ConnectionCapacity {
        ConnectionCapacity {
            method: conn.method,
            z: 0.0,
            group_factor: None,
        }
    }
    fn governing_combination(&self, combos: &[LoadCombination]) -> Option<LoadCombination> {
        max_factored(combos)
    }
}

/// Hot-rolled structural steel leaf (AISC 360). Plastic/gross design for compact sections.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AiscSteel {
    pub id: DesignStandardId,
}

impl Default for AiscSteel {
    fn default() -> Self {
        AiscSteel {
            id: DesignStandardId::from("AISC-360-16"),
        }
    }
}

impl DesignStandard for AiscSteel {
    fn id(&self) -> DesignStandardId {
        self.id.clone()
    }
    fn code(&self) -> DesignCode {
        DesignCode::Aisc
    }
    fn material(&self) -> MaterialKind {
        MaterialKind::Steel
    }
    fn section_basis(&self) -> SectionBasisKind {
        SectionBasisKind::Plastic
    }
    fn philosophy(&self) -> DesignPhilosophy {
        DesignPhilosophy::LRFD
    }
    fn design_values(&self, query: &SizingQuery) -> SectionBasis {
        section_basis(
            SectionBasisKind::Plastic,
            &[("Fy", 50_000.0), ("Fu", 65_000.0)],
            29.0e6,
            query,
        )
    }
    fn modification_factors(&self, _ctx: &FactorContext) -> Vec<ModificationFactor> {
        vec![ModificationFactor::contextual("phi", 0.9)]
    }
    fn extra_limit_states(&self) -> Vec<LimitStateId> {
        modes(&[
            "LTB",
            "flangeLocalBuckling",
            "webLocalBuckling",
            "compactness",
        ])
    }
    fn connection_capacity(&self, conn: &Connection) -> ConnectionCapacity {
        ConnectionCapacity {
            method: conn.method,
            z: 0.0,
            group_factor: None,
        }
    }
    fn governing_combination(&self, combos: &[LoadCombination]) -> Option<LoadCombination> {
        max_factored(combos)
    }
}

/// Concrete leaf (ACI 318). Transformed/cracked section; `cast` stockForm bypasses the cut
/// optimizer entirely without the core knowing about concrete.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AciConcrete {
    pub id: DesignStandardId,
}

impl Default for AciConcrete {
    fn default() -> Self {
        AciConcrete {
            id: DesignStandardId::from("ACI-318-19"),
        }
    }
}

impl DesignStandard for AciConcrete {
    fn id(&self) -> DesignStandardId {
        self.id.clone()
    }
    fn code(&self) -> DesignCode {
        DesignCode::Aci
    }
    fn material(&self) -> MaterialKind {
        MaterialKind::Concrete
    }
    fn section_basis(&self) -> SectionBasisKind {
        SectionBasisKind::Transformed
    }
    fn philosophy(&self) -> DesignPhilosophy {
        DesignPhilosophy::LRFD
    }
    fn design_values(&self, query: &SizingQuery) -> SectionBasis {
        // f'c 4000 psi, rebar fy 60 ksi, Ec ≈ 57000·√f'c.
        let fc = 4000.0;
        section_basis(
            SectionBasisKind::Transformed,
            &[("fc_prime", fc), ("fy", 60_000.0)],
            57_000.0 * fc.sqrt(),
            query,
        )
    }
    fn modification_factors(&self, _ctx: &FactorContext) -> Vec<ModificationFactor> {
        vec![ModificationFactor::contextual("phiFlex", 0.9)]
    }
    fn extra_limit_states(&self) -> Vec<LimitStateId> {
        modes(&["crackedFlexure", "stirrupShear", "devLength", "splice"])
    }
    fn connection_capacity(&self, conn: &Connection) -> ConnectionCapacity {
        ConnectionCapacity {
            method: conn.method,
            z: 0.0,
            group_factor: None,
        }
    }
    fn governing_combination(&self, combos: &[LoadCombination]) -> Option<LoadCombination> {
        max_factored(combos)
    }
}

/// Masonry leaf (TMS 402). Net/transformed section; `unit` stockForm is the fourth output path
/// (count + grout/mortar volume), confirming routing is by stockForm not material.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TmsMasonry {
    pub id: DesignStandardId,
}

impl Default for TmsMasonry {
    fn default() -> Self {
        TmsMasonry {
            id: DesignStandardId::from("TMS-402-16"),
        }
    }
}

impl DesignStandard for TmsMasonry {
    fn id(&self) -> DesignStandardId {
        self.id.clone()
    }
    fn code(&self) -> DesignCode {
        DesignCode::Tms
    }
    fn material(&self) -> MaterialKind {
        MaterialKind::Masonry
    }
    fn section_basis(&self) -> SectionBasisKind {
        SectionBasisKind::Net
    }
    fn philosophy(&self) -> DesignPhilosophy {
        DesignPhilosophy::ASD
    }
    fn design_values(&self, query: &SizingQuery) -> SectionBasis {
        // f'm 1500 psi, rebar fy 60 ksi, Em ≈ 900·f'm.
        let fm = 1500.0;
        section_basis(
            SectionBasisKind::Net,
            &[("fm_prime", fm), ("fy", 60_000.0)],
            900.0 * fm,
            query,
        )
    }
    fn modification_factors(&self, _ctx: &FactorContext) -> Vec<ModificationFactor> {
        vec![ModificationFactor::contextual("slendernessReduction", 1.0)]
    }
    fn extra_limit_states(&self) -> Vec<LimitStateId> {
        modes(&[
            "axialSlenderness",
            "flexuralTension",
            "shearFriction",
            "anchorBolt",
        ])
    }
    fn connection_capacity(&self, conn: &Connection) -> ConnectionCapacity {
        ConnectionCapacity {
            method: conn.method,
            z: 0.0,
            group_factor: None,
        }
    }
    fn governing_combination(&self, combos: &[LoadCombination]) -> Option<LoadCombination> {
        max_factored(combos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_leaf_supplies_the_full_interface() {
        let leaves: Vec<Box<dyn DesignStandard>> = vec![
            Box::new(AisiCfs::default()),
            Box::new(AiscSteel::default()),
            Box::new(AciConcrete::default()),
            Box::new(TmsMasonry::default()),
        ];
        for leaf in &leaves {
            // Structurally complete: a basis, a factor stack, declared modes — only the tables
            // (strategy_check) are empty.
            assert!(!leaf.extra_limit_states().is_empty());
            assert!(
                leaf.governing_combination(&LoadCombination::lrfd_set())
                    .is_some()
            );
        }
    }

    #[test]
    fn leaves_declare_distinct_bases() {
        assert_eq!(
            AisiCfs::default().section_basis(),
            SectionBasisKind::Effective
        );
        assert_eq!(
            AiscSteel::default().section_basis(),
            SectionBasisKind::Plastic
        );
        assert_eq!(
            AciConcrete::default().section_basis(),
            SectionBasisKind::Transformed
        );
        assert_eq!(TmsMasonry::default().section_basis(), SectionBasisKind::Net);
    }
}
