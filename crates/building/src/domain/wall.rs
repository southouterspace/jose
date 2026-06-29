//! Semantic promotion: [`Wall`], [`Opening`], [`Junction`]. Shape becomes meaning. The hard
//! layer — stud counts are **derived** by the [`FramingSolver`](crate::FramingSolver), never
//! stored: edit the wall, the framer re-runs.

use crate::domain::spacing::SpacingModule;
use crate::keys::{FaceRef, JunctionRef, WallId, WallRef};
use geometry_kernel::{Segment, Tick};

/// What a wall is for — selects load participation and default detailing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WallRole {
    Exterior,
    Interior,
    Bearing,
    Partition,
}

/// A door or window void in a wall.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OpeningType {
    Door,
    Window,
}

/// A door/window void in a wall, driving king/jack/header/cripple/sill framing around it. An
/// immutable value object owned by its [`Wall`]. All four linear fields are integer ticks.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Opening {
    /// Door or window.
    pub opening_type: OpeningType,
    /// Rough-opening width in ticks (not the door/window unit width).
    pub width: Tick,
    /// Rough-opening height in ticks.
    pub height: Tick,
    /// Height of the rough sill above the wall base in ticks; 0 for a door.
    pub sill_height: Tick,
    /// Offset of the opening start along the wall baseline from wall start, in ticks.
    pub position: Tick,
}

/// Corner / tee / cross condition where walls meet.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JunctionType {
    Corner,
    Tee,
    Cross,
}

/// Which way a detected corner turns, derived from the drawing (never hand-tagged): **Outside**
/// is a convex corner (the interior faces are on the inside of the turn — a rectangle's four
/// corners), **Inside** is a concave/reentrant corner (the elbow of an L-shaped room). Selects
/// the default detailing method. A `Tee` carries no sense in v1.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CornerSense {
    /// Convex corner (interior on the inside of the turn).
    Outside,
    /// Concave/reentrant corner (interior on the outside of the turn).
    Inside,
}

/// How the owner wall frames a shared post at a junction — selects how many members it emits.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JunctionMethod {
    /// Three-stud corner.
    ThreeStud,
    /// Two studs plus a drywall clip.
    TwoStudClip,
    /// California corner.
    California,
}

/// Where two or more walls meet. Names the condition **and** the single owner so a shared post is
/// counted exactly once — the owner-wall pattern that resolves corner double-counting
/// deterministically. A value object identified structurally by its participating walls.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Junction {
    /// The condition.
    pub junction_type: JunctionType,
    /// All walls meeting at this junction.
    pub walls: Vec<WallRef>,
    /// The one wall that claims and frames the shared post.
    pub owner_wall: WallRef,
    /// The framing method — selects member count.
    pub method: JunctionMethod,
    /// For a `Corner`, whether it turns convex (`Outside`) or concave (`Inside`); `None` for a
    /// `Tee` (no sense in v1). Derived from the drawing, never authored.
    pub sense: Option<CornerSense>,
}

impl Junction {
    /// Whether `wall` is the designated owner that frames the shared post (the others see the
    /// corner but do not frame it — this is what keeps the count exact).
    pub fn is_owner(&self, wall: WallRef) -> bool {
        self.owner_wall == wall
    }
}

/// A bearing or partition wall semantically promoted from a vertical kernel face. Carries the
/// parameters the framer needs; stud count is **derived**, never stored.
#[derive(Clone, PartialEq, Debug)]
pub struct Wall {
    /// Stable identity.
    pub id: WallId,
    /// The geometry-kernel face this wall was promoted from (re-promotion source on face edit).
    pub source_face: FaceRef,
    /// Baseline in plan (two tick endpoints). Length is derived from it.
    pub baseline: Segment,
    /// Derived wall length in ticks — a fast read for OC layout, recomputed on edit, not authored.
    pub length: Tick,
    /// Wall height in ticks.
    pub height: Tick,
    /// Nominal framed thickness in ticks (a 2x4 wall = 3.5in = 112t).
    pub thickness: Tick,
    /// Load participation / detailing.
    pub role: WallRole,
    /// On-center layout module (a parameter, not ticks).
    pub spacing: SpacingModule,
    /// Voids owned by this wall; drive opening framing.
    pub openings: Vec<Opening>,
    /// Back-references to junctions this wall participates in.
    pub junction_refs: Vec<JunctionRef>,
    /// Which side of the baseline (looking from `a` toward `b`) the wall's **interior face**
    /// lies on — `true` = left (the +90° side of the baseline direction in plan). The outward-
    /// framing rule says the drawn footprint is the interior face; corner *sense* (convex vs
    /// concave) cannot be recovered from two bare segments, so it needs this. Plain Rust, not in
    /// the SoA buffer. Defaults to `true` in [`Wall::promote`] (a CCW-drawn footprint has its
    /// interior on the left of every edge).
    pub interior_on_left: bool,
}

impl Wall {
    /// Promote a wall from a baseline + face, deriving `length` from the baseline. Height,
    /// thickness, role and spacing are the authored parameters.
    pub fn promote(
        id: WallId,
        source_face: FaceRef,
        baseline: Segment,
        height: Tick,
        thickness: Tick,
        role: WallRole,
        spacing: SpacingModule,
    ) -> Wall {
        let length = Tick(
            (baseline.length_inches() * geometry_kernel::TICKS_PER_INCH as f64).round() as i32,
        );
        Wall {
            id,
            source_face,
            baseline,
            length,
            height,
            thickness,
            role,
            spacing,
            openings: Vec::new(),
            junction_refs: Vec::new(),
            // CCW footprint convention: interior on the left of each baseline. Adjust per wall
            // when a footprint edge is drawn the other way.
            interior_on_left: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::spacing::SpacingModule;
    use geometry_kernel::{EntityId, TickVec3};

    fn straight_wall(len_ticks: i32) -> Wall {
        let baseline = Segment::new(
            TickVec3::ZERO,
            TickVec3::new(Tick(len_ticks), Tick(0), Tick(0)),
        );
        Wall::promote(
            WallId(1),
            FaceRef {
                volume: EntityId(1),
                face_index: 0,
            },
            baseline,
            Tick(96 * 32),
            Tick(112),
            WallRole::Bearing,
            SpacingModule::inches(16),
        )
    }

    #[test]
    fn promote_derives_length_from_baseline() {
        let w = straight_wall(3840); // 10ft
        assert_eq!(w.length, Tick(3840));
        assert!(w.openings.is_empty());
    }

    #[test]
    fn junction_owner_frames_exactly_once() {
        let j = Junction {
            junction_type: JunctionType::Corner,
            walls: vec![WallId(1), WallId(2)],
            owner_wall: WallId(1),
            method: JunctionMethod::California,
            sense: Some(CornerSense::Outside),
        };
        assert!(j.is_owner(WallId(1)));
        assert!(!j.is_owner(WallId(2)));
    }
}
