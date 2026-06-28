//! # design-standard
//!
//! The **Structural — DesignStandard Strategy Seam** bounded context. One material-blind
//! structural-check stage that calls *through* a single [`DesignStandard`] interface; N material
//! leaves supply behind it. The shared core ([`BeamStatics`], [`SizingArbiter`], [`LimitStateCheck`]
//! records) never moves when material changes; each leaf supplies exactly six things plus its
//! tables: design values, modification factors, a limit-state set, a connection-capacity method,
//! the governing load combination, and the section basis.
//!
//! **Adding a material is adding a leaf, never editing the core.** [`NdsWood`] is the fully
//! populated reference; [`AisiCfs`], [`AiscSteel`], [`AciConcrete`] and [`TmsMasonry`] are
//! structurally complete stubs. The arbiter never branches on material — it branches only on
//! `stockForm` (routing) and `origin` (check aggregation).
//!
//! ## Hexagonal shape
//!
//! This context is where ports/adapters become real: the [`DesignStandard`] trait is the **port**
//! the application core ([`SizingArbiter`]) depends on; the material leaves are the **adapters**
//! that implement it. The domain value objects the seam passes across the interface live in
//! `domain/`.
//!
//! ## Cycle break
//!
//! The schema couples this layer and the loads layer in both directions. The pipeline order
//! (loads → structural) makes loads the upstream crate: this crate depends on `loads-analysis`
//! (consuming `MemberDemand` / `LoadCombination`), while loads-analysis references this layer's
//! concepts only by opaque key — so the crate graph is acyclic.

mod adapters;
mod application;
mod domain;
mod keys;
mod ports;

pub use adapters::nds_wood::NdsWood;
pub use adapters::stubs::{AciConcrete, AiscSteel, AisiCfs, TmsMasonry};
pub use application::beam_statics::BeamStatics;
pub use application::sizing_arbiter::SizingArbiter;
pub use domain::connection::{Connection, ConnectionCapacity, ConnectionGraph, ConnectionMethod};
pub use domain::factors::{
    FactorContext, FactorId, FactorKind, ModificationFactor, MoistureCondition,
};
pub use domain::limit_state::{CheckOrigin, LimitStateCheck, LimitStateId};
pub use domain::philosophy::{
    DesignCode, DesignPhilosophy, FactorSide, MaterialKind, PhilosophyMode, SectionBasisKind,
};
pub use domain::section::SectionBasis;
pub use domain::sizing::{Escape, SizingMethod, SizingQuery, SizingResult};
pub use keys::{ConnectionGraphId, DesignStandardId, MechanicalPropertiesKey};
pub use ports::design_standard::DesignStandard;

#[cfg(test)]
mod tests {
    //! Seam-level integration: a wood beam sized through the interface, and the proof that a steel
    //! member runs through the identical arbiter with only the injected leaf swapped.
    use super::*;
    use loads_analysis::{MemberDemand, MemberRole};
    use materials::{Axis, SectionProperties, SpecKey};

    fn beam_query(moment: f64, shear: f64) -> SizingQuery {
        SizingQuery {
            member_id: building::MemberPlacementId(1),
            demand: MemberDemand {
                member_ref: building::MemberPlacementId(1),
                member_role: MemberRole::Beam,
                governing_combo: "LRFD-2".to_owned(),
                axial: 0.0,
                moment,
                shear,
                deflection: None,
                deflection_limit: 0.4,
                deflection_ratio: Some(360),
                uniform_load: Some(40.0),
                span: geometry_kernel::Tick(4608),
                unbraced_length: None,
                reaction: Some(300.0),
                applied_at: None,
            },
            candidate_spec: SpecKey::from("SPF-2x8"),
            span_in: 144.0,
            // 2x8: 1.5in × 7.25in.
            section: SectionProperties::rectangular(1.5, 7.25, Axis::Strong, 1),
            context: FactorContext::default(),
        }
    }

    #[test]
    fn wood_beam_sizes_through_the_seam() {
        let leaf = NdsWood::df_l_no2();
        let arbiter = SizingArbiter::new();
        // A light moment passes; the bending/shear/deflection core checks are all present.
        // A 2x8 DF-L No.2 carries ~11,800 lb·in in bending at CD=1.0, so 8,000 lb·in is modest.
        let result = arbiter.size(&beam_query(8_000.0, 600.0), &leaf);
        assert!(
            result
                .checks
                .iter()
                .any(|c| c.id == LimitStateId::bending())
        );
        assert!(result.checks.iter().any(|c| c.id == LimitStateId::shear()));
        assert!(
            result
                .checks
                .iter()
                .any(|c| c.id == LimitStateId::deflection())
        );
        assert_eq!(result.philosophy_used, DesignPhilosophy::ASD);
        assert!(
            result.pass,
            "a modest moment should pass: {:?}",
            result.governing_check
        );

        // An overwhelming moment fails on bending.
        let overloaded = arbiter.size(&beam_query(5_000_000.0, 600.0), &leaf);
        assert!(!overloaded.pass);
        assert_eq!(overloaded.governing_check, LimitStateId::bending());
        assert!(overloaded.utilization > 1.0);
    }

    #[test]
    fn the_same_arbiter_runs_a_steel_leaf_unchanged() {
        let arbiter = SizingArbiter::new();
        // Identical query + identical arbiter; only the injected strategy differs (the hybrid
        // end state the seam exists for). Steel sizes on Fy under LRFD.
        let result = arbiter.size(&beam_query(50_000.0, 600.0), &AiscSteel::default());
        assert_eq!(result.philosophy_used, DesignPhilosophy::LRFD);
        assert!(
            result
                .checks
                .iter()
                .any(|c| c.id == LimitStateId::bending())
        );
    }
}
