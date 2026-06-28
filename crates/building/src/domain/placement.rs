//! [`MemberPlacement`] — one framing member, in place — and its install-context value objects
//! ([`Orientation`], [`BracingRef`], [`EndCondition`]).
//!
//! The exemplar intrinsic/contextual seam: a shared material-agnostic `StockSpec` (referenced by
//! key) plus per-instance install context. This layer *originates* the contextual
//! adjustment-factor **inputs** (bracing, end fixity, orientation, the demand linkage) as neutral
//! physical facts — it never models the factor stack or any design values (the strategy seam) and
//! never models the load traversal (the loads layer).

use crate::keys::{ConnectionPointRef, MemberDemandRef, MemberPlacementId};
use geometry_kernel::{Tick, Transform, UnitVec3};
use materials::SpecKey;

/// Which face of a member takes load — section-relative and material-agnostic, so it selects the
/// governing section modulus for any material.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LoadFace {
    /// Loaded flat on the narrow edge (weak-axis bending).
    Narrow,
    /// Loaded on the wide face (strong-axis bending — joist orientation).
    Wide,
}

/// The member's local axis frame. Decides strong vs weak axis — a ~5–6× swing in bending capacity
/// for the same stick. Directions are unitless [`UnitVec3`]s, never tick positions.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Orientation {
    /// Which face takes load.
    pub load_face: LoadFace,
    /// The strong bending axis (unitless direction).
    pub strong_axis: UnitVec3,
    /// The weak bending axis, orthogonal to `strong_axis`.
    pub weak_axis: UnitVec3,
}

/// Which member axis a brace restrains.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BracingAxis {
    Strong,
    Weak,
}

/// The source of lateral restraint on a member.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BracedBy {
    /// Sheathing — typically braces the weak axis.
    Sheathing,
    /// Discrete blocking.
    Blocking,
    /// No restraint — full-height unbraced.
    None,
}

/// What braces a given axis, and how often. Sets the effective (unbraced) length that is the
/// `CP`/`CL` adjustment-factor **input** — the factor itself is computed behind the seam.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BracingRef {
    /// The restrained axis.
    pub axis: BracingAxis,
    /// The restraint source.
    pub braced_by: BracedBy,
    /// Unbraced length along the axis, in ticks (linear) — the physical input to `CP`/`CL`.
    pub spacing: Tick,
}

/// Which end of a member an [`EndCondition`] describes (aligned to the baseline direction).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MemberEnd {
    Start,
    Finish,
}

/// Restraint condition at a member end.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fixity {
    Pinned,
    Fixed,
    Bearing,
    Free,
}

/// Fixity at one end of a member. Feeds the effective-length factor (`Ke`) and load-transfer
/// mode — both contextual factor **inputs**, not the factor itself.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EndCondition {
    /// Which end.
    pub end: MemberEnd,
    /// The restraint condition.
    pub fixity: Fixity,
    /// A concrete fastener/bearing detail at this end (materials-layer connection point).
    pub connection_ref: Option<ConnectionPointRef>,
}

/// One framing member, in place. References a shared material-agnostic `StockSpec` (by key);
/// **owns** its install context. The unit the capacity check, the cut list, and the cost takeoff
/// all read. Length is derived by the solver, never authored.
#[derive(Clone, PartialEq, Debug)]
pub struct MemberPlacement {
    /// Stable identity.
    pub id: MemberPlacementId,
    /// Pointer to the shared `StockSpec` flyweight (materials layer). Intrinsic, never copied.
    pub spec_ref: SpecKey,
    /// Open role string (`stud`|`king`|`jack`|`header`|`cripple`|`sill`|`plate`|`block`, plus
    /// `joist`|`rafter`|`chord`|`panel`). String-coded so new assemblies extend roles freely.
    pub role: String,
    /// Rigid placement (origin + rotation), a geometry-kernel primitive.
    pub transform: Transform,
    /// Cut length, **derived** by the solver from wall/opening geometry; recomputed on edit.
    pub length: Tick,
    /// Contextual: which face takes load → strong vs weak axis.
    pub orientation: Orientation,
    /// Contextual: unbraced lengths per axis → `CP`/`CL` inputs.
    pub bracing: Vec<BracingRef>,
    /// Contextual: fixity at both ends → effective-length factor + load transfer.
    pub ends: [EndCondition; 2],
    /// Per-member fastener locations (materials-layer connection points), by handle.
    pub connections: Vec<ConnectionPointRef>,
    /// Downstream link to the per-member demand this placement carries (loads layer), by handle.
    /// The placement does not compute or store loads — this is a neutral linkage.
    pub demand_ref: Option<MemberDemandRef>,
}

impl Orientation {
    /// A vertical member (stud) loaded on its wide face: strong axis horizontal across the wall,
    /// weak axis through the wall thickness.
    pub fn vertical_stud() -> Orientation {
        Orientation {
            load_face: LoadFace::Wide,
            strong_axis: UnitVec3::X,
            weak_axis: UnitVec3::Y,
        }
    }

    /// A horizontal member lying flat (plate/header/sill): strong axis along its length, weak
    /// axis vertical.
    pub fn flat() -> Orientation {
        Orientation {
            load_face: LoadFace::Wide,
            strong_axis: UnitVec3::X,
            weak_axis: UnitVec3::Z,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orientation_axes_are_orthogonal() {
        let o = Orientation::vertical_stud();
        assert!(o.strong_axis.is_orthogonal_to(o.weak_axis));
        assert_eq!(o.load_face, LoadFace::Wide);
    }

    #[test]
    fn placement_holds_context_not_loads() {
        let p = MemberPlacement {
            id: MemberPlacementId(1),
            spec_ref: SpecKey::from("SPF-STUD-SDRY"),
            role: "stud".to_owned(),
            transform: Transform::IDENTITY,
            length: Tick(3072),
            orientation: Orientation::vertical_stud(),
            bracing: vec![BracingRef {
                axis: BracingAxis::Weak,
                braced_by: BracedBy::Sheathing,
                spacing: Tick(512),
            }],
            ends: [
                EndCondition {
                    end: MemberEnd::Start,
                    fixity: Fixity::Bearing,
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
        };
        assert_eq!(p.role, "stud");
        assert_eq!(p.bracing[0].braced_by, BracedBy::Sheathing);
        assert!(p.demand_ref.is_none()); // loads are not stored here
    }
}
