//! [`LoadSolver`] — orchestrates the layer end to end: quantify sources → distribute
//! (tributary + path + rollup) → apply the combination set → emit `MemberDemand[]`.
//!
//! The material-sensitive pick of *which* combination governs is, per the schema, the strategy's
//! to make. To stay upstream of the design-standard crate, this solver applies a generic,
//! material-blind governing rule (the combination that maximizes the role's controlling effect)
//! and records which combination it chose; a downstream strategy may override the pick.

use crate::application::load_path::{LoadEdge, LoadPath};
use crate::application::load_rollup::LoadRollup;
use crate::domain::combination::LoadCombination;
use crate::domain::demand::{MemberDemand, MemberRole};
use crate::domain::sources::LoadSource;
use crate::domain::tributary::TributaryArea;
use crate::keys::{ConnectionGraphRef, DesignStandardRef};
use building::{FramingRole, MemberPlacement, MemberPlacementId};
use std::collections::BTreeMap;

/// How much of the model to re-derive.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RecomputeMode {
    /// Only members touched by a changed input re-derive (the default, dirty-range driven).
    Incremental,
    /// Re-derive everything.
    Full,
}

/// The layer's single entry point: holds the active strategy handle and recompute policy, and
/// turns placed members + sources into per-member factored demand.
#[derive(Clone, PartialEq, Debug)]
pub struct LoadSolver {
    /// The active strategy the solver would delegate the governing-combo pick to (by key).
    pub standard: DesignStandardRef,
    /// Recompute policy.
    pub recompute: RecomputeMode,
}

impl LoadSolver {
    /// A solver bound to a strategy, defaulting to incremental recompute.
    pub fn new(standard: DesignStandardRef) -> LoadSolver {
        LoadSolver {
            standard,
            recompute: RecomputeMode::Incremental,
        }
    }

    /// Solve for per-member demand. `edges` is the connection-graph load-transfer list (ordered by
    /// the load path); `combos` is the candidate ASCE 7 set. Members without an attributed
    /// tributary produce no demand.
    pub fn solve(
        &self,
        graph: ConnectionGraphRef,
        members: &[MemberPlacement],
        edges: &[LoadEdge],
        sources: &[LoadSource],
        tributaries: &[TributaryArea],
        combos: &[LoadCombination],
    ) -> Vec<MemberDemand> {
        // 1. Order the path; fall back to the given member order if there are no edges.
        let mut path = LoadPath::gravity(graph);
        let walked = path.walk(edges).to_vec();
        let order: Vec<MemberPlacementId> = if walked.is_empty() {
            members.iter().map(|m| m.id).collect()
        } else {
            walked
        };

        // 2. Roll unfactored per-source demand down the path.
        let accumulated = LoadRollup::default().roll(&order, sources, tributaries);
        let mut by_member: BTreeMap<MemberPlacementId, Vec<_>> = BTreeMap::new();
        for a in accumulated {
            by_member.entry(a.member_ref).or_default().push(a);
        }

        let trib_by: BTreeMap<MemberPlacementId, &TributaryArea> =
            tributaries.iter().map(|t| (t.member_ref, t)).collect();
        let role_by: BTreeMap<MemberPlacementId, MemberRole> =
            members.iter().map(|m| (m.id, role_of(m.role))).collect();

        // 3. Factor each member under every combo and keep the governing one.
        let mut out = Vec::new();
        for member in order {
            let (Some(acc), Some(trib)) = (by_member.get(&member), trib_by.get(&member)) else {
                continue;
            };
            let role = role_by.get(&member).copied().unwrap_or(MemberRole::Beam);
            let span_ft = trib.span.to_feet();

            let Some(best) = combos
                .iter()
                .map(|c| factor_combo(c, acc, span_ft, role))
                .reduce(|a, b| if b.controlling >= a.controlling { b } else { a })
            else {
                continue;
            };

            let ratio = 360; // L/360 live-load serviceability default
            out.push(MemberDemand {
                member_ref: member,
                member_role: role,
                governing_combo: best.combo_id,
                axial: best.axial,
                moment: best.moment,
                shear: best.shear,
                deflection: None, // E-dependent — completed by the strategy
                deflection_limit: MemberDemand::deflection_limit_for(trib.span, ratio),
                deflection_ratio: Some(ratio),
                uniform_load: Some(best.uniform_load),
                span: trib.span,
                unbraced_length: None,
                reaction: Some(best.shear),
                applied_at: trib.load_centroid,
            });
        }
        out
    }
}

/// One combo's factored result for a member.
struct FactoredDemand {
    combo_id: String,
    moment: f64,
    shear: f64,
    axial: f64,
    uniform_load: f64,
    /// The effect that decides governing for this role (axial for columns, moment for beams).
    controlling: f64,
}

/// Apply a combination's factors to a member's per-source accumulated demand.
fn factor_combo(
    combo: &LoadCombination,
    acc: &[crate::domain::demand::AccumulatedDemand],
    span_ft: f64,
    role: MemberRole,
) -> FactoredDemand {
    let mut moment = 0.0;
    let mut shear = 0.0;
    let mut uniform_load = 0.0;
    for a in acc {
        let f = combo.factor_for(a.source_kind);
        moment += a.moment * f;
        shear += a.shear * f;
        uniform_load += a.line_load * f;
    }
    // A column carries its tributary distributed load as axial; a beam bends under it.
    let axial = match role {
        MemberRole::Column | MemberRole::TensionTie | MemberRole::Bearing => uniform_load * span_ft,
        MemberRole::Beam | MemberRole::Brace => 0.0,
    };
    let controlling = match role {
        MemberRole::Column | MemberRole::TensionTie | MemberRole::Bearing => axial,
        MemberRole::Beam | MemberRole::Brace => moment,
    };
    FactoredDemand {
        combo_id: combo.combo_id.clone(),
        moment,
        shear,
        axial,
        uniform_load,
        controlling,
    }
}

/// Map a placement's [`FramingRole`] to the structural [`MemberRole`] the seam keys limit states on.
/// Exhaustive by construction: adding a framing role makes this fail to compile until it is
/// classified, so no member silently falls through to a default limit-state set.
fn role_of(role: FramingRole) -> MemberRole {
    match role {
        FramingRole::Stud
        | FramingRole::King
        | FramingRole::Jack
        | FramingRole::Cripple
        | FramingRole::Post => MemberRole::Column,
        FramingRole::Plate | FramingRole::Sill => MemberRole::Bearing,
        FramingRole::Header | FramingRole::Joist | FramingRole::Rafter | FramingRole::Chord => {
            MemberRole::Beam
        }
        FramingRole::Block | FramingRole::Panel => MemberRole::Brace,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::sources::{DeadLoad, LiveLoad, LiveOccupancy};
    use building::{
        BracedBy, BracingAxis, BracingRef, EndCondition, Fixity, MemberEnd, Orientation,
    };
    use geometry_kernel::{Tick, Transform, UnitVec3};
    use materials::SpecKey;

    fn header(id: u128) -> MemberPlacement {
        MemberPlacement {
            id: MemberPlacementId(id),
            spec_ref: SpecKey::from("DF-HEADER"),
            role: FramingRole::Header,
            transform: Transform::IDENTITY,
            length: Tick(4608),
            orientation: Orientation::flat(),
            bracing: vec![BracingRef {
                axis: BracingAxis::Weak,
                braced_by: BracedBy::Sheathing,
                spacing: Tick(512),
            }],
            ends: [
                EndCondition {
                    end: MemberEnd::Start,
                    fixity: Fixity::Pinned,
                    connection_ref: None,
                },
                EndCondition {
                    end: MemberEnd::Finish,
                    fixity: Fixity::Pinned,
                    connection_ref: None,
                },
            ],
            connections: vec![],
            demand_ref: None,
        }
    }

    #[test]
    fn solves_governing_demand_for_a_beam() {
        let members = [header(1)];
        let tributaries = [TributaryArea::strip(
            MemberPlacementId(1),
            Tick(4608),
            Tick(512),
        )];
        let sources = vec![
            LoadSource::dead(DeadLoad {
                self_weight_plf: 3.0,
                assembly_dead_psf: 10.0,
                direction: UnitVec3::Y.flipped(),
                source_ref: None,
            }),
            LoadSource::live(LiveLoad {
                occupancy: LiveOccupancy::LivingArea,
                base_psf: 40.0,
                is_roof_live: false,
                element_factor_kll: None,
                reduction_factor: None,
                reduced_psf: None,
                source_ref: None,
            }),
        ];
        let solver = LoadSolver::new(DesignStandardRef::from("NDS-2018"));
        let demands = solver.solve(
            ConnectionGraphRef::from("g"),
            &members,
            &[],
            &sources,
            &tributaries,
            &LoadCombination::lrfd_set(),
        );
        assert_eq!(demands.len(), 1);
        let d = &demands[0];
        assert_eq!(d.member_role, MemberRole::Beam);
        // Gravity combo (1.2D + 1.6L) governs over dead-only for a downward-loaded beam.
        assert_eq!(d.governing_combo, "LRFD-2");
        assert!(d.moment > 0.0 && d.shear > 0.0);
        // L/360 of a 144in span = 0.4in.
        assert!((d.deflection_limit - 0.4).abs() < 1e-9);
    }

    #[test]
    fn member_without_tributary_yields_no_demand() {
        let members = [header(1)];
        let solver = LoadSolver::new(DesignStandardRef::from("NDS-2018"));
        let demands = solver.solve(
            ConnectionGraphRef::from("g"),
            &members,
            &[],
            &[],
            &[],
            &LoadCombination::lrfd_set(),
        );
        assert!(demands.is_empty());
    }
}
