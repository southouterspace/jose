//! The [`FramingSolver`] service: wall + openings + junctions → an ordered, **stable** set of
//! [`MemberPlacement`]s (plates, studs, opening framing) at the wall's OC spacing.
//!
//! Stability is the whole game: a 2in nudge must not reshuffle every stud. The layout grid is
//! *anchored* at the wall start and derived from the spacing module, so interior stud positions
//! are invariant under a length change — only the end stud moves.

use crate::domain::placement::{
    BracedBy, BracingAxis, BracingRef, EndCondition, Fixity, MemberEnd, MemberPlacement,
    Orientation,
};
use crate::domain::role::FramingRole;
use crate::domain::wall::{Junction, JunctionMethod, OpeningType, Wall};
use crate::keys::MemberPlacementId;
use geometry_kernel::{Tick, TickVec3, Transform};
use materials::SpecKey;

/// Nominal dressed plate/stud thickness, 1.5in = 48 ticks.
const PLATE_THICKNESS: i32 = 48;

/// Which rule pack the solver runs. Wall is fully modeled; Floor/Roof/Sheathing register their
/// own packs without new placement entities (the extension seam).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AssemblyKind {
    Wall,
    Floor,
    Roof,
    Sheathing,
}

/// The flyweight rule pack looked up by `assemblyKind` (OC layout, plate stack, specs), not
/// copied per wall. Modeled in-memory at this phase.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RuleSet {
    /// Spec for studs / kings / jacks / cripples.
    pub stud_spec: SpecKey,
    /// Spec for top/bottom plates and sills.
    pub plate_spec: SpecKey,
    /// Spec for headers (typically a deeper built-up section).
    pub header_spec: SpecKey,
    /// Bottom-plate count (usually 1).
    pub bottom_plates: u32,
    /// Top-plate count (usually 2 — a double top plate).
    pub top_plates: u32,
}

impl RuleSet {
    /// A conventional light-frame wall pack: single bottom plate, double top plate.
    pub fn light_frame_wall(
        stud_spec: SpecKey,
        plate_spec: SpecKey,
        header_spec: SpecKey,
    ) -> RuleSet {
        RuleSet {
            stud_spec,
            plate_spec,
            header_spec,
            bottom_plates: 1,
            top_plates: 2,
        }
    }
}

impl JunctionMethod {
    /// How many shared-post members the owner wall frames for this junction method.
    pub fn post_count(self) -> u32 {
        match self {
            JunctionMethod::ThreeStud | JunctionMethod::California => 3,
            JunctionMethod::TwoStudClip => 2,
        }
    }
}

/// Stateful framer that hands out stable [`MemberPlacementId`]s as it emits members. Derives all
/// counts — nothing is stored on the wall.
#[derive(Clone, Debug)]
pub struct FramingSolver {
    next_id: u128,
}

impl Default for FramingSolver {
    fn default() -> Self {
        FramingSolver::new()
    }
}

impl FramingSolver {
    /// A fresh framer.
    pub fn new() -> FramingSolver {
        FramingSolver { next_id: 1 }
    }

    fn fresh_id(&mut self) -> MemberPlacementId {
        let id = MemberPlacementId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Frame one wall (with the junctions it participates in) into a derived, ordered placement
    /// set. Only `assemblyKind = Wall` is modeled here; Floor/Roof/Sheathing route their own rule
    /// packs through this same entry point and currently emit nothing (extension stubs).
    pub fn frame(
        &mut self,
        kind: AssemblyKind,
        wall: &Wall,
        junctions: &[Junction],
        rules: &RuleSet,
    ) -> Vec<MemberPlacement> {
        match kind {
            AssemblyKind::Wall => self.frame_wall(wall, junctions, rules),
            // Extension stubs: same promotion + placement seam, rule packs land later.
            AssemblyKind::Floor | AssemblyKind::Roof | AssemblyKind::Sheathing => Vec::new(),
        }
    }

    fn frame_wall(
        &mut self,
        wall: &Wall,
        junctions: &[Junction],
        rules: &RuleSet,
    ) -> Vec<MemberPlacement> {
        let mut out = Vec::new();
        let plate_stack = (rules.bottom_plates + rules.top_plates) as i32 * PLATE_THICKNESS;
        let stud_len = Tick((wall.height.raw() - plate_stack).max(0));
        let bottom_z = rules.bottom_plates as i32 * PLATE_THICKNESS;

        // Plates span the full wall length.
        for i in 0..rules.bottom_plates {
            let z = i as i32 * PLATE_THICKNESS;
            out.push(self.member(
                rules.plate_spec.clone(),
                FramingRole::Plate,
                TickVec3::new(Tick(0), Tick(0), Tick(z)),
                wall.length,
                Orientation::flat(),
            ));
        }
        for i in 0..rules.top_plates {
            let z = wall.height.raw() - (i as i32 + 1) * PLATE_THICKNESS;
            out.push(self.member(
                rules.plate_spec.clone(),
                FramingRole::Plate,
                TickVec3::new(Tick(0), Tick(0), Tick(z)),
                wall.length,
                Orientation::flat(),
            ));
        }

        // Anchored OC stud grid: positions 0, step, 2·step, … plus an end stud. Skipped inside
        // openings; opening edges get explicit king studs.
        let step = wall.spacing.step_ticks().raw().max(1);
        let l = wall.length.raw();
        let mut xs: Vec<i32> = (0..).map(|n| n * step).take_while(|&x| x < l).collect();
        xs.push(l);
        xs.sort_unstable();
        xs.dedup();

        for x in xs {
            if wall.openings.iter().any(|o| {
                let (a, b) = (o.position.raw(), o.position.raw() + o.width.raw());
                x >= a && x <= b
            }) {
                continue; // interior of an opening — framed by opening members instead
            }
            out.push(self.stud(
                rules.stud_spec.clone(),
                FramingRole::Stud,
                x,
                bottom_z,
                stud_len,
                step,
            ));
        }

        // Opening framing: king studs at the edges, a jack supporting a header, and a sill +
        // cripples for windows.
        for o in &wall.openings {
            let a = o.position.raw();
            let b = a + o.width.raw();
            let header_underside = o.sill_height.raw() + o.height.raw();

            for &edge in &[a, b] {
                out.push(self.stud(
                    rules.stud_spec.clone(),
                    FramingRole::King,
                    edge,
                    bottom_z,
                    stud_len,
                    step,
                ));
            }
            // Jacks sit inside the kings, carrying the header.
            for &jack in &[a + PLATE_THICKNESS, b - PLATE_THICKNESS] {
                out.push(self.stud(
                    rules.stud_spec.clone(),
                    FramingRole::Jack,
                    jack,
                    bottom_z,
                    Tick((header_underside - bottom_z).max(0)),
                    step,
                ));
            }
            // Header across the opening (bearing onto the jacks each side).
            out.push(self.member(
                rules.header_spec.clone(),
                FramingRole::Header,
                TickVec3::new(Tick(a), Tick(0), Tick(header_underside)),
                Tick(o.width.raw() + 2 * PLATE_THICKNESS),
                Orientation::flat(),
            ));
            // Windows: a sill plus cripples below it.
            if o.opening_type == OpeningType::Window && o.sill_height.raw() > bottom_z {
                out.push(self.member(
                    rules.plate_spec.clone(),
                    FramingRole::Sill,
                    TickVec3::new(Tick(a), Tick(0), o.sill_height),
                    o.width,
                    Orientation::flat(),
                ));
                let mut cx = a + step;
                while cx < b {
                    out.push(self.stud(
                        rules.stud_spec.clone(),
                        FramingRole::Cripple,
                        cx,
                        bottom_z,
                        Tick((o.sill_height.raw() - bottom_z).max(0)),
                        step,
                    ));
                    cx += step;
                }
            }
        }

        // Shared corner/intersection posts — only the owner wall frames them, so a shared post is
        // counted exactly once.
        for j in junctions.iter().filter(|j| j.is_owner(wall.id)) {
            for _ in 0..j.method.post_count() {
                out.push(self.stud(
                    rules.stud_spec.clone(),
                    FramingRole::Post,
                    0,
                    bottom_z,
                    stud_len,
                    step,
                ));
            }
        }

        out
    }

    /// A vertical member (stud/king/jack/cripple/post) at wall-local `x`, sitting on the bottom
    /// plate, braced weak-axis by sheathing at the OC step, bearing at the bottom and pinned top.
    #[allow(clippy::too_many_arguments)]
    fn stud(
        &mut self,
        spec: SpecKey,
        role: FramingRole,
        x: i32,
        bottom_z: i32,
        length: Tick,
        step: i32,
    ) -> MemberPlacement {
        MemberPlacement {
            id: self.fresh_id(),
            spec_ref: spec,
            role,
            transform: Transform::at(TickVec3::new(Tick(x), Tick(0), Tick(bottom_z))),
            length,
            orientation: Orientation::vertical_stud(),
            bracing: vec![BracingRef {
                axis: BracingAxis::Weak,
                braced_by: BracedBy::Sheathing,
                spacing: Tick(step),
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
            connections: Vec::new(),
            demand_ref: None,
        }
    }

    /// A horizontal member (plate/header/sill) with a flat orientation, pinned both ends.
    fn member(
        &mut self,
        spec: SpecKey,
        role: FramingRole,
        origin: TickVec3,
        length: Tick,
        orientation: Orientation,
    ) -> MemberPlacement {
        MemberPlacement {
            id: self.fresh_id(),
            spec_ref: spec,
            role,
            transform: Transform::at(origin),
            length,
            orientation,
            bracing: Vec::new(),
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
            connections: Vec::new(),
            demand_ref: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::spacing::SpacingModule;
    use crate::domain::wall::{Opening, WallRole};
    use crate::keys::{FaceRef, WallId};
    use geometry_kernel::{EntityId, Segment};

    fn rules() -> RuleSet {
        RuleSet::light_frame_wall(
            SpecKey::from("SPF-STUD"),
            SpecKey::from("SPF-PLATE"),
            SpecKey::from("DF-HEADER"),
        )
    }

    fn wall(len_ticks: i32) -> Wall {
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
            Tick(96 * 32), // 8ft
            Tick(112),
            WallRole::Bearing,
            SpacingModule::inches(16),
        )
    }

    #[test]
    fn frames_plates_and_studs() {
        let mut fs = FramingSolver::new();
        let members = fs.frame(AssemblyKind::Wall, &wall(3840), &[], &rules());
        let plates = members
            .iter()
            .filter(|m| m.role == FramingRole::Plate)
            .count();
        let studs = members
            .iter()
            .filter(|m| m.role == FramingRole::Stud)
            .count();
        assert_eq!(plates, 3); // 1 bottom + 2 top
        // 10ft wall @ 16in OC: studs at 0,512,…,4608 (<3840 gives 0..3584 = 8) + end stud.
        assert!(studs >= 8);
        // stud length = 96in - 3 plates*1.5in = 96 - 4.5 = 91.5in = 2928 ticks.
        let a_stud = members
            .iter()
            .find(|m| m.role == FramingRole::Stud)
            .unwrap();
        assert_eq!(a_stud.length, Tick(96 * 32 - 3 * 48));
    }

    #[test]
    fn grid_is_stable_under_a_small_nudge() {
        let mut fs = FramingSolver::new();
        let before: Vec<i32> = fs
            .frame(AssemblyKind::Wall, &wall(3840), &[], &rules())
            .iter()
            .filter(|m| m.role == FramingRole::Stud)
            .map(|m| m.transform.origin.x.raw())
            .collect();
        // Nudge the wall 2in (64 ticks) longer; interior grid positions must be unchanged.
        let mut fs2 = FramingSolver::new();
        let after: Vec<i32> = fs2
            .frame(AssemblyKind::Wall, &wall(3840 + 64), &[], &rules())
            .iter()
            .filter(|m| m.role == FramingRole::Stud)
            .map(|m| m.transform.origin.x.raw())
            .collect();
        // Every anchored interior stud (all but the moved end stud) is shared.
        let common: Vec<i32> = before
            .iter()
            .copied()
            .filter(|x| after.contains(x))
            .collect();
        assert!(common.len() >= before.len() - 1);
    }

    #[test]
    fn opening_gets_kings_jacks_and_header() {
        let mut w = wall(3840);
        w.openings.push(Opening {
            opening_type: OpeningType::Window,
            width: Tick(32 * 36),  // 36in
            height: Tick(32 * 36), // 36in
            sill_height: Tick(32 * 36),
            position: Tick(32 * 48), // 4ft in
        });
        let mut fs = FramingSolver::new();
        let m = fs.frame(AssemblyKind::Wall, &w, &[], &rules());
        assert_eq!(m.iter().filter(|x| x.role == FramingRole::King).count(), 2);
        assert_eq!(
            m.iter().filter(|x| x.role == FramingRole::Header).count(),
            1
        );
        assert_eq!(m.iter().filter(|x| x.role == FramingRole::Sill).count(), 1);
        assert!(m.iter().any(|x| x.role == FramingRole::Cripple));
        // No regular stud lands inside the opening span.
        let inside = m.iter().filter(|x| x.role == FramingRole::Stud).any(|x| {
            let xx = x.transform.origin.x.raw();
            xx > 32 * 48 && xx < 32 * 48 + 32 * 36
        });
        assert!(!inside);
    }

    #[test]
    fn only_owner_frames_junction_posts() {
        let j = Junction {
            junction_type: crate::domain::wall::JunctionType::Corner,
            walls: vec![WallId(1), WallId(2)],
            owner_wall: WallId(1),
            method: JunctionMethod::California,
        };
        let mut fs = FramingSolver::new();
        let owner = fs.frame(
            AssemblyKind::Wall,
            &wall(3840),
            std::slice::from_ref(&j),
            &rules(),
        );
        assert_eq!(
            owner.iter().filter(|m| m.role == FramingRole::Post).count(),
            3
        );

        // A non-owner wall sees the same junction but frames no posts.
        let mut w2 = wall(3840);
        w2.id = WallId(2);
        let mut fs2 = FramingSolver::new();
        let other = fs2.frame(AssemblyKind::Wall, &w2, std::slice::from_ref(&j), &rules());
        assert_eq!(
            other.iter().filter(|m| m.role == FramingRole::Post).count(),
            0
        );
    }
}
