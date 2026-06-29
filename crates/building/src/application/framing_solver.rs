//! The [`FramingSolver`] service: wall + openings + junctions → an ordered, **stable** set of
//! [`MemberPlacement`]s (plates, studs, opening framing) at the wall's OC spacing.
//!
//! Stability is the whole game: a 2in nudge must not reshuffle every stud. The layout grid is
//! *anchored* at the wall start and derived from the spacing module, so interior stud positions
//! are invariant under a length change — only the end stud moves.

use crate::application::junction_detail::{DetailedPost, detail_junction, plate_lap};
use crate::application::junction_detector::detect_junctions;
use crate::domain::placement::{
    BracedBy, BracingAxis, BracingRef, EndCondition, Fixity, MemberEnd, MemberPlacement,
    Orientation,
};
use crate::domain::role::FramingRole;
use crate::domain::wall::{Junction, JunctionMethod, JunctionType, OpeningType, Wall};
use crate::keys::MemberPlacementId;
use geometry_kernel::{Tick, TickVec2, TickVec3, Transform};
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
    /// set. `neighbors` carries the other walls a junction couples this one to, so the junction
    /// detailer can build the corner geometry; pass an empty slice when there are no junctions.
    /// Only `assemblyKind = Wall` is modeled here; Floor/Roof/Sheathing route their own rule packs
    /// through this same entry point and currently emit nothing (extension stubs).
    pub fn frame(
        &mut self,
        kind: AssemblyKind,
        wall: &Wall,
        junctions: &[Junction],
        neighbors: &[Wall],
        rules: &RuleSet,
    ) -> Vec<MemberPlacement> {
        match kind {
            AssemblyKind::Wall => self.frame_wall(wall, junctions, neighbors, rules),
            // Extension stubs: same promotion + placement seam, rule packs land later.
            AssemblyKind::Floor | AssemblyKind::Roof | AssemblyKind::Sheathing => Vec::new(),
        }
    }

    fn frame_wall(
        &mut self,
        wall: &Wall,
        junctions: &[Junction],
        neighbors: &[Wall],
        rules: &RuleSet,
    ) -> Vec<MemberPlacement> {
        let mut out = Vec::new();
        let plate_stack = (rules.bottom_plates + rules.top_plates) as i32 * PLATE_THICKNESS;
        let stud_len = Tick((wall.height.raw() - plate_stack).max(0));
        let bottom_z = rules.bottom_plates as i32 * PLATE_THICKNESS;

        // Bottom plate(s) span the full wall length.
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
        // Lapped double top plate: each course is extended into / trimmed back from each end's
        // corner so exactly one wall covers the corner cell per course, staggered between courses
        // (ADR 0009 §5). `course 0` = the lower top plate, `course 1` = the cap plate.
        for i in 0..rules.top_plates {
            let z = wall.height.raw() - (i as i32 + 1) * PLATE_THICKNESS;
            let (start_x, length) = self.lapped_top_plate_extent(wall, junctions, neighbors, i);
            out.push(self.member(
                rules.plate_spec.clone(),
                FramingRole::Plate,
                TickVec3::new(Tick(start_x), Tick(0), Tick(z)),
                Tick(length),
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

        // Shared corner posts — only the owner wall frames them (a shared post is counted exactly
        // once). The junction detailer turns each owned corner into real, world-placed vertical
        // members (the S1/S2/S3 footprints), not just a count.
        for j in junctions.iter().filter(|j| j.is_owner(wall.id)) {
            let participants: Vec<&Wall> = std::iter::once(wall)
                .chain(neighbors.iter().filter(|n| j.walls.contains(&n.id)))
                .collect();
            let detail = detail_junction(j, &participants);
            for post in detail.posts {
                out.push(self.corner_post(rules.stud_spec.clone(), post, bottom_z, stud_len, step));
            }
        }

        out
    }

    /// The lapped top-plate extent for `course` of `wall`: its world-local start x and length in
    /// ticks. A corner the wall runs *through* on this course extends the plate one wall-thickness
    /// past that baseline end into the corner cell; a corner it *butts* trims the plate back one
    /// wall-thickness so it stops short. Staggered between courses, so exactly one of the two walls
    /// at a corner covers the corner cell per course (ADR 0009 §5).
    fn lapped_top_plate_extent(
        &self,
        wall: &Wall,
        junctions: &[Junction],
        neighbors: &[Wall],
        course: u32,
    ) -> (i32, i32) {
        let thickness = wall.thickness.raw();
        let mut start = 0;
        let mut end = wall.length.raw();
        for j in junctions
            .iter()
            .filter(|j| j.junction_type == JunctionType::Corner)
        {
            let Some(lap) = plate_lap(j, wall.id, course) else {
                continue;
            };
            let delta = if lap.runs_through {
                thickness
            } else {
                -thickness
            };
            // Which baseline end this corner sits at: the shared vertex matches one of this wall's
            // endpoints. Find it via the neighbour wall the junction couples us to.
            if corner_at_wall_start(wall, j, neighbors) {
                start -= delta;
            } else {
                end += delta;
            }
        }
        (start, (end - start).max(0))
    }

    /// A corner post placed at its **world** plan min corner (the detailer already resolved the
    /// junction-local recipe to world), standing up the wall. Same install context as a field stud
    /// (bears at the bottom, pinned at the top, weak-axis braced by sheathing at the OC step).
    fn corner_post(
        &mut self,
        spec: SpecKey,
        post: DetailedPost,
        bottom_z: i32,
        length: Tick,
        step: i32,
    ) -> MemberPlacement {
        MemberPlacement {
            id: self.fresh_id(),
            spec_ref: spec,
            role: FramingRole::Post,
            transform: Transform::at(TickVec3::new(post.min.u, post.min.v, Tick(bottom_z))),
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

/// Frame a **set** of walls end-to-end: detect their junctions, then frame each wall with the
/// junctions it participates in and the neighbour walls those junctions couple it to (ADR 0009 §6
/// — wholesale recompute). Returns every member across all walls, with stable ids minted in wall
/// order. This is the building-context entry the session composes; the single-wall `frame` stays
/// for the one-wall draw path.
pub fn frame_walls(
    solver: &mut FramingSolver,
    kind: AssemblyKind,
    walls: &[Wall],
    rules: &RuleSet,
) -> Vec<MemberPlacement> {
    let junctions = detect_junctions(walls);
    let mut out = Vec::new();
    for wall in walls {
        let mine: Vec<Junction> = junctions
            .iter()
            .filter(|j| j.walls.contains(&wall.id))
            .cloned()
            .collect();
        let neighbors: Vec<Wall> = walls.iter().filter(|w| w.id != wall.id).cloned().collect();
        out.extend(solver.frame(kind, wall, &mine, &neighbors, rules));
    }
    out
}

/// Whether `junction`'s shared corner sits at `wall`'s baseline **start** (endpoint `a`, x=0 in
/// wall-local) rather than its end (endpoint `b`, x=length). The shared vertex is the endpoint
/// `wall` has in common with its neighbour at this junction; comparing it to `a` gives the end.
fn corner_at_wall_start(wall: &Wall, junction: &Junction, neighbors: &[Wall]) -> bool {
    let a = TickVec2::new(wall.baseline.a.x, wall.baseline.a.y);
    let Some(other) = neighbors
        .iter()
        .find(|n| n.id != wall.id && junction.walls.contains(&n.id))
    else {
        return false;
    };
    let oa = TickVec2::new(other.baseline.a.x, other.baseline.a.y);
    let ob = TickVec2::new(other.baseline.b.x, other.baseline.b.y);
    // The shared vertex is whichever of this wall's endpoints also belongs to the neighbour. If it
    // is endpoint `a`, the corner sits at this wall's start (x = 0); otherwise it is at the end.
    a == oa || a == ob
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
        let members = fs.frame(AssemblyKind::Wall, &wall(3840), &[], &[], &rules());
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
            .frame(AssemblyKind::Wall, &wall(3840), &[], &[], &rules())
            .iter()
            .filter(|m| m.role == FramingRole::Stud)
            .map(|m| m.transform.origin.x.raw())
            .collect();
        // Nudge the wall 2in (64 ticks) longer; interior grid positions must be unchanged.
        let mut fs2 = FramingSolver::new();
        let after: Vec<i32> = fs2
            .frame(AssemblyKind::Wall, &wall(3840 + 64), &[], &[], &rules())
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
        let m = fs.frame(AssemblyKind::Wall, &w, &[], &[], &rules());
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

    /// An L-corner: owner wall id 1 runs into the origin along +x; other wall id 2 leaves along
    /// +y. Vertex at the world origin so the golden footprints land where the detailer puts them.
    fn l_corner_walls() -> (Wall, Wall) {
        let owner = {
            let baseline = Segment::new(
                TickVec3::new(Tick(10 * 384), Tick(0), Tick(0)),
                TickVec3::ZERO,
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
        };
        let other = {
            let baseline = Segment::new(
                TickVec3::ZERO,
                TickVec3::new(Tick(0), Tick(10 * 384), Tick(0)),
            );
            Wall::promote(
                WallId(2),
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
        };
        (owner, other)
    }

    #[test]
    fn only_owner_frames_junction_posts() {
        let (owner_wall, other_wall) = l_corner_walls();
        let j = Junction {
            junction_type: crate::domain::wall::JunctionType::Corner,
            walls: vec![WallId(1), WallId(2)],
            owner_wall: WallId(1),
            method: JunctionMethod::California,
            sense: Some(crate::domain::wall::CornerSense::Outside),
        };
        let mut fs = FramingSolver::new();
        let owner = fs.frame(
            AssemblyKind::Wall,
            &owner_wall,
            std::slice::from_ref(&j),
            std::slice::from_ref(&other_wall),
            &rules(),
        );
        // California → three real corner posts (not just a count).
        let posts: Vec<_> = owner
            .iter()
            .filter(|m| m.role == FramingRole::Post)
            .collect();
        assert_eq!(posts.len(), 3);
        // Every post is vertical (extends up in +z), anchored on the bottom plate.
        assert!(posts.iter().all(|p| p.role.is_vertical()));
        // The S1 corner post sits at the world vertex (origin) min corner.
        assert!(
            posts
                .iter()
                .any(|p| p.transform.origin.x.raw() == 0 && p.transform.origin.y.raw() == 0)
        );

        // A non-owner wall sees the same junction but frames no posts.
        let mut fs2 = FramingSolver::new();
        let other = fs2.frame(
            AssemblyKind::Wall,
            &other_wall,
            std::slice::from_ref(&j),
            std::slice::from_ref(&owner_wall),
            &rules(),
        );
        assert_eq!(
            other.iter().filter(|m| m.role == FramingRole::Post).count(),
            0
        );
    }

    fn wall_between(id: u128, ax: i32, ay: i32, bx: i32, by: i32) -> Wall {
        let baseline = Segment::new(
            TickVec3::new(Tick(ax), Tick(ay), Tick(0)),
            TickVec3::new(Tick(bx), Tick(by), Tick(0)),
        );
        Wall::promote(
            WallId(id),
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
    fn rectangle_frames_four_corners_owner_only_no_doubling() {
        // CCW 10ft square: four walls, four outside corners. Each corner is California (3 posts),
        // owned once → exactly 4 × 3 = 12 posts across the whole set, never doubled.
        let ft = 384;
        let walls = [
            wall_between(1, 0, 0, 10 * ft, 0),
            wall_between(2, 10 * ft, 0, 10 * ft, 10 * ft),
            wall_between(3, 10 * ft, 10 * ft, 0, 10 * ft),
            wall_between(4, 0, 10 * ft, 0, 0),
        ];
        let mut fs = FramingSolver::new();
        let members = frame_walls(&mut fs, AssemblyKind::Wall, &walls, &rules());
        let posts = members
            .iter()
            .filter(|m| m.role == FramingRole::Post)
            .count();
        assert_eq!(
            posts,
            4 * JunctionMethod::California.post_count() as usize,
            "four corners × the California post count, owner-only (no doubling)"
        );
    }

    #[test]
    fn inside_corner_frames_without_error() {
        // The reentrant elbow of an L-shaped room: detected Inside → ThreeStud, frames cleanly.
        let ft = 384;
        let a = wall_between(1, 10 * ft, 6 * ft, 6 * ft, 6 * ft);
        let b = wall_between(2, 6 * ft, 6 * ft, 6 * ft, 10 * ft);
        let mut fs = FramingSolver::new();
        let members = frame_walls(&mut fs, AssemblyKind::Wall, &[a, b], &rules());
        // Inside default ThreeStud → 3 posts at the one corner, owner-only.
        assert_eq!(
            members
                .iter()
                .filter(|m| m.role == FramingRole::Post)
                .count(),
            JunctionMethod::ThreeStud.post_count() as usize
        );
    }

    #[test]
    fn lapped_top_plate_staggers_and_covers_the_corner_cell() {
        // The golden L: owner (id 1) runs into the origin along +x; other (id 2) leaves along +y.
        // The corner sits at the owner's baseline END (x = length) and the other's START (x = 0).
        let (owner_wall, other_wall) = l_corner_walls();
        let j = Junction {
            junction_type: JunctionType::Corner,
            walls: vec![WallId(1), WallId(2)],
            owner_wall: WallId(1),
            method: JunctionMethod::California,
            sense: Some(crate::domain::wall::CornerSense::Outside),
        };
        let thickness = 112;
        let len = owner_wall.length.raw();

        // Owner: course 0 runs through (extends past the end by +thickness), course 1 butts (-).
        let fs = FramingSolver::new();
        let o0 = fs.lapped_top_plate_extent(
            &owner_wall,
            std::slice::from_ref(&j),
            std::slice::from_ref(&other_wall),
            0,
        );
        let o1 = fs.lapped_top_plate_extent(
            &owner_wall,
            std::slice::from_ref(&j),
            std::slice::from_ref(&other_wall),
            1,
        );
        // Owner's corner is at its END, so the through-course lengthens, the butt-course shortens.
        assert_eq!(
            o0,
            (0, len + thickness),
            "course 0: owner runs through (extended)"
        );
        assert_eq!(o1, (0, len - thickness), "course 1: owner butts (trimmed)");

        // Other: corner at its START (x=0). Course 0 it butts (start moves +thickness, shortening),
        // course 1 it runs through (start moves -thickness, lengthening). Staggered vs the owner.
        let x0 = fs.lapped_top_plate_extent(
            &other_wall,
            std::slice::from_ref(&j),
            std::slice::from_ref(&owner_wall),
            0,
        );
        let x1 = fs.lapped_top_plate_extent(
            &other_wall,
            std::slice::from_ref(&j),
            std::slice::from_ref(&owner_wall),
            1,
        );
        let olen = other_wall.length.raw();
        assert_eq!(x0, (thickness, olen - thickness), "course 0: other butts");
        assert_eq!(
            x1,
            (-thickness, olen + thickness),
            "course 1: other runs through"
        );

        // Per course exactly one wall covers the corner cell: course 0 owner-through XOR other-through.
        assert!(
            o0.1 > len && x0.0 > 0,
            "course 0: owner covers, other recedes"
        );
        assert!(
            o1.1 < len && x1.0 < 0,
            "course 1: other covers, owner recedes"
        );
    }
}
