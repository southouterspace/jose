//! [`DesignStandard`] — THE seam. The single Strategy interface (a port) every Tier-1 core type
//! calls through to stay material-blind. Each material leaf (an adapter) implements it.
//!
//! The interface declares a finite, enumerable supply surface, so adding a material is a closed
//! checklist, not a redesign: design values, modification factors, the extra limit-state set, a
//! connection-capacity method, the governing combination, and the section basis — plus an optional
//! prescriptive table. The boundary test is mechanical: a field belongs behind the seam iff it
//! reads Fb-or-Fy / ASD-or-LRFD / gross-or-effective / cut-or-cast.

use crate::domain::connection::{Connection, ConnectionCapacity};
use crate::domain::factors::{FactorContext, ModificationFactor};
use crate::domain::limit_state::LimitStateCheck;
use crate::domain::limit_state::LimitStateId;
use crate::domain::philosophy::{DesignCode, DesignPhilosophy, MaterialKind, SectionBasisKind};
use crate::domain::section::SectionBasis;
use crate::domain::sizing::SizingQuery;
use crate::keys::DesignStandardId;
use loads_analysis::LoadCombination;
use reference_data::PrescriptiveLookup;

/// The Strategy interface. N material leaves supply behind it; the shared core (BeamStatics,
/// SizingArbiter, LimitStateCheck) never moves when material changes.
pub trait DesignStandard {
    /// The leaf-selecting key, e.g. `NDS-2018`.
    fn id(&self) -> DesignStandardId;
    /// Which standard this leaf implements.
    fn code(&self) -> DesignCode;
    /// The material this leaf sizes (must match the `Stock.material` it sizes).
    fn material(&self) -> MaterialKind;
    /// Which basis `SectionProperties` are computed on.
    fn section_basis(&self) -> SectionBasisKind;
    /// The leaf's default ASD/LRFD philosophy.
    fn philosophy(&self) -> DesignPhilosophy;

    /// ① Resolve a query's candidate spec to a [`SectionBasis`] (strengths + stiffness on the
    /// declared basis). For wood this references the single `MechanicalProperties` flyweight.
    fn design_values(&self, query: &SizingQuery) -> SectionBasis;

    /// ② The applied (contextual) factor stack for a given placement/use condition.
    fn modification_factors(&self, ctx: &FactorContext) -> Vec<ModificationFactor>;

    /// ③ The **extra** (strategy-origin) failure modes this leaf checks beyond core
    /// bending/shear/deflection. The arbiter enumerates these and asks [`strategy_check`] for each.
    ///
    /// [`strategy_check`]: DesignStandard::strategy_check
    fn extra_limit_states(&self) -> Vec<LimitStateId>;

    /// Compute one strategy-supplied check (e.g. wood column buckling). Returns `None` for a
    /// declared-but-unpopulated mode (a stub leaf: structure complete, table empty).
    fn strategy_check(
        &self,
        mode: &LimitStateId,
        query: &SizingQuery,
        section: &SectionBasis,
        factors: &[ModificationFactor],
    ) -> Option<LimitStateCheck> {
        let _ = (mode, query, section, factors);
        None
    }

    /// ④ Capacity of a typed connection edge.
    fn connection_capacity(&self, conn: &Connection) -> ConnectionCapacity;

    /// ⑤ Pick which ASCE 7 combination governs (wood: `min(load/CD)`; steel/concrete: max
    /// factored). The combination *set* is owned by the loads layer; only the **pick** is here.
    fn governing_combination(&self, combos: &[LoadCombination]) -> Option<LoadCombination>;

    /// ⑥ The leaf's non-engineered prescriptive table, if any.
    fn prescriptive_table(&self) -> Option<&PrescriptiveLookup> {
        None
    }
}
