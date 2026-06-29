//! The [`detail_junction`] service: a classified [`Junction`] + its participating [`Wall`]s →
//! the concrete **corner posts** (vertical [`MemberPlacement`]s in world coordinates) and the
//! **lapped double top-plate** decisions. The geometric half of ADR 0009 §1/§5 — detection
//! (`junction_detector`) finds and classifies; this turns a classification into real members.
//!
//! Pure, deterministic, junction-local → world. The detailer lays out the corner studs in a
//! **junction-local frame** (the shared outside vertex at the origin, the *through* wall running
//! local +x, the *butting* wall running local +y, interior to the NE) and places the result by
//! the junction's transform — the flyweight/per-junction-transform idiom of ADR 0009 §2. For an
//! axis-aligned L it reproduces the golden footprints exactly; the same recipe generalises to any
//! corner because the local axes *are* the two walls' outgoing directions.

use crate::domain::wall::{Junction, JunctionMethod, JunctionType, Wall};
use geometry_kernel::{Tick, TickVec2};

/// A 2×4 dressed face: 1.5in = 48 ticks (the narrow dimension of a corner stud).
const STUD_NARROW: i32 = 48;
/// A 2×4 dressed face: 3.5in = 112 ticks (the wide dimension / wall thickness).
const STUD_WIDE: i32 = 112;

/// One corner post, as a junction-local **plan footprint** (min-corner → size, in ticks) plus a
/// role. The detailer emits these in the junction-local frame; [`detail_junction`] transforms them
/// to world. `vertical` posts extend up the wall in +z by the post length.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct LocalPost {
    /// Min corner of the plan footprint in junction-local ticks.
    min: TickVec2,
    /// Footprint size (du, dv) in junction-local ticks.
    size: TickVec2,
}

impl LocalPost {
    const fn new(min_u: i32, min_v: i32, du: i32, dv: i32) -> LocalPost {
        LocalPost {
            min: TickVec2::new(Tick(min_u), Tick(min_v)),
            size: TickVec2::new(Tick(du), Tick(dv)),
        }
    }
}

/// The junction-local corner-post recipe for a method, in ticks. The golden reference
/// (ADR 0009 / the corner study): outside butt corner, shared vertex at local origin, through wall
/// +x with body v∈[0,3.5], butting wall +y with body u∈[0,3.5], interior NE.
fn local_posts(method: JunctionMethod) -> Vec<LocalPost> {
    match method {
        // S1 corner post, S2 butting-wall end stud, S3 upright backer packing the pocket.
        JunctionMethod::ThreeStud => vec![
            LocalPost::new(0, 0, STUD_NARROW, STUD_WIDE),
            LocalPost::new(0, STUD_WIDE, STUD_WIDE, STUD_NARROW),
            LocalPost::new(2 * 32, 0, STUD_NARROW, STUD_WIDE),
        ],
        // S1, S2 as above; S3 is a *flat* backer that leaves the drywall pocket open.
        JunctionMethod::California => vec![
            LocalPost::new(0, 0, STUD_NARROW, STUD_WIDE),
            LocalPost::new(0, STUD_WIDE, STUD_WIDE, STUD_NARROW),
            LocalPost::new(STUD_NARROW, 2 * 32, STUD_WIDE, STUD_NARROW),
        ],
        // Two studs plus a clip: the corner post and the butting-wall end stud, pocket left open.
        JunctionMethod::TwoStudClip => vec![
            LocalPost::new(0, 0, STUD_NARROW, STUD_WIDE),
            LocalPost::new(0, STUD_WIDE, STUD_WIDE, STUD_NARROW),
        ],
    }
}

/// Which wall runs *through* a corner cell on a given plate course, and which *butts* into it.
/// The lapped double top plate staggers the joint between courses (ADR 0009 §5): course 0 → the
/// owner runs through; course 1 → it flips. One [`PlateLap`] per (wall, course).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PlateLap {
    /// The top-plate course this decision applies to (0 = lower top plate, 1 = upper cap plate).
    pub course: u32,
    /// Whether *this wall* runs through the corner cell on this course (vs. butting into it).
    pub runs_through: bool,
}

/// The result of detailing one junction *for one participating wall*: the corner posts the wall
/// emits (only the owner emits posts; non-owners get an empty vec) and how each of the wall's two
/// top-plate courses is lapped at this corner.
#[derive(Clone, PartialEq, Debug)]
pub struct JunctionDetail {
    /// Corner posts in **world** coordinates, vertical, role `Post`. Empty unless `wall` is the
    /// junction owner.
    pub posts: Vec<DetailedPost>,
    /// Per-course lap decision for the wall's double top plate at this corner.
    pub laps: Vec<PlateLap>,
}

/// A corner post resolved to world coordinates: the plan footprint's min corner and size (ticks),
/// ready to be promoted to a vertical [`MemberPlacement`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DetailedPost {
    /// World plan min corner (x, y) in ticks.
    pub min: TickVec2,
    /// World plan footprint size (dx, dy) in ticks (always non-negative).
    pub size: TickVec2,
}

/// Detail `junction` for `wall`, given both participating walls. Returns the world-space corner
/// posts (owner-only) and the wall's top-plate lap decisions.
///
/// Pure: identical `(method, sense, walls)` → identical output, so equal corners reuse one
/// computation (the memoisable flyweight of ADR 0009 §2). Only `Corner` junctions emit posts in
/// v1; a `Tee` produces no corner posts here (its backing is field framing) and no plate lap.
pub fn detail_junction(junction: &Junction, walls: &[&Wall]) -> JunctionDetail {
    if junction.junction_type != JunctionType::Corner {
        return JunctionDetail {
            posts: Vec::new(),
            laps: Vec::new(),
        };
    }

    // The two participating walls, and which one is the owner / runs through course 0.
    let Some((owner, other)) = corner_walls(junction, walls) else {
        return JunctionDetail {
            posts: Vec::new(),
            laps: Vec::new(),
        };
    };

    // Junction-local frame: shared vertex at the origin; local +x = the *through* wall's outgoing
    // direction, local +y = the *butting* wall's outgoing direction. The owner is the through
    // wall (it runs the corner cell on course 0); interior is the local NE quadrant by
    // construction, matching the golden recipe.
    let frame = JunctionFrame::of(owner, other);

    let posts = if junction.is_owner(owner.id) {
        local_posts(junction.method)
            .into_iter()
            .map(|p| frame.place(p))
            .collect()
    } else {
        Vec::new()
    };

    JunctionDetail {
        posts,
        laps: plate_laps(junction, walls),
    }
}

/// The lapped double top-plate decisions for `wall` at `junction` (both courses). Course 0: the
/// owner runs through, the other butts. Course 1 flips. Staggered, so exactly one wall covers the
/// corner cell per course (ADR 0009 §5).
fn plate_laps(junction: &Junction, walls: &[&Wall]) -> Vec<PlateLap> {
    let Some(this) = walls.iter().find(|w| junction.walls.contains(&w.id)) else {
        return Vec::new();
    };
    [0, 1]
        .into_iter()
        .filter_map(|course| plate_lap(junction, this.id, course))
        .collect()
}

/// The lap decision for one `wall` at `junction` on a given top-plate `course` (0 = lower top
/// plate, 1 = cap plate). `None` for a non-corner junction or a wall not in the junction. Course 0
/// → the owner runs through; course 1 flips, so the two courses stagger.
pub fn plate_lap(junction: &Junction, wall: crate::keys::WallId, course: u32) -> Option<PlateLap> {
    if junction.junction_type != JunctionType::Corner || !junction.walls.contains(&wall) {
        return None;
    }
    let owner_through_course0 = junction.is_owner(wall);
    let runs_through = match course {
        0 => owner_through_course0,
        _ => !owner_through_course0,
    };
    Some(PlateLap {
        course,
        runs_through,
    })
}

/// Resolve the two walls of a `Corner` junction into `(owner, other)`. `None` if either is absent
/// from `walls` (caller passed the wrong set).
fn corner_walls<'a>(junction: &Junction, walls: &[&'a Wall]) -> Option<(&'a Wall, &'a Wall)> {
    let owner = *walls.iter().find(|w| w.id == junction.owner_wall)?;
    let other = *walls
        .iter()
        .find(|w| junction.walls.contains(&w.id) && w.id != junction.owner_wall)?;
    Some((owner, other))
}

/// The junction-local → world transform: the shared vertex and the two local basis axes (each a
/// wall's outgoing direction from the vertex, ±unit on an axis-aligned corner). A local plan point
/// `(u, v)` maps to `vertex + u·ex + v·ey`.
struct JunctionFrame {
    vertex: TickVec2,
    /// Local +x basis (the *through* wall's outgoing direction), as an integer tick unit vector.
    ex: (i64, i64),
    /// Local +y basis (the *butting* wall's outgoing direction).
    ey: (i64, i64),
}

impl JunctionFrame {
    /// Build the frame for an owner (through, local +x) and other (butting, local +y) wall. The
    /// shared vertex and outgoing directions are recovered from the baselines.
    fn of(owner: &Wall, other: &Wall) -> JunctionFrame {
        let vertex = shared_vertex(owner, other);
        let ex = unit_outgoing(vertex, owner);
        let ey = unit_outgoing(vertex, other);
        JunctionFrame { vertex, ex, ey }
    }

    /// Transform a junction-local plan footprint to a world [`DetailedPost`]. The four local
    /// corners are mapped and the world AABB taken, so a sign-flipped basis still yields a
    /// non-negative size with the correct min corner.
    fn place(&self, p: LocalPost) -> DetailedPost {
        let (u0, v0) = (p.min.u.raw() as i64, p.min.v.raw() as i64);
        let (u1, v1) = (u0 + p.size.u.raw() as i64, v0 + p.size.v.raw() as i64);
        let corners = [(u0, v0), (u1, v0), (u0, v1), (u1, v1)];
        let mut min_x = i64::MAX;
        let mut min_y = i64::MAX;
        let mut max_x = i64::MIN;
        let mut max_y = i64::MIN;
        for (u, v) in corners {
            let wx = self.vertex.u.raw() as i64 + u * self.ex.0 + v * self.ey.0;
            let wy = self.vertex.v.raw() as i64 + u * self.ex.1 + v * self.ey.1;
            min_x = min_x.min(wx);
            min_y = min_y.min(wy);
            max_x = max_x.max(wx);
            max_y = max_y.max(wy);
        }
        DetailedPost {
            min: TickVec2::new(Tick(min_x as i32), Tick(min_y as i32)),
            size: TickVec2::new(Tick((max_x - min_x) as i32), Tick((max_y - min_y) as i32)),
        }
    }
}

/// The shared baseline endpoint of two walls in plan (ticks). Falls back to one endpoint if no
/// exact match — callers only build a frame for a detected `Corner`, which always shares one.
fn shared_vertex(a: &Wall, b: &Wall) -> TickVec2 {
    let (a0, a1) = endpoints(a);
    let (b0, b1) = endpoints(b);
    for v in [a0, a1] {
        if v == b0 || v == b1 {
            return v;
        }
    }
    a1
}

/// Plan endpoints of a wall baseline (drop z).
fn endpoints(w: &Wall) -> (TickVec2, TickVec2) {
    (
        TickVec2::new(w.baseline.a.x, w.baseline.a.y),
        TickVec2::new(w.baseline.b.x, w.baseline.b.y),
    )
}

/// The wall's outgoing direction from `vertex`, reduced to an integer **unit** step on an
/// axis-aligned baseline (each component is -1, 0, or +1). Non-axis-aligned corners reduce by the
/// gcd, which keeps the local lattice consistent though it is no longer unit length.
fn unit_outgoing(vertex: TickVec2, w: &Wall) -> (i64, i64) {
    let (a, b) = endpoints(w);
    let (dx, dy) = if vertex == a {
        (
            (b.u.raw() - a.u.raw()) as i64,
            (b.v.raw() - a.v.raw()) as i64,
        )
    } else {
        (
            (a.u.raw() - b.u.raw()) as i64,
            (a.v.raw() - b.v.raw()) as i64,
        )
    };
    let g = gcd(dx.abs(), dy.abs()).max(1);
    (dx / g, dy / g)
}

/// Greatest common divisor (for reducing an outgoing direction to its unit step).
fn gcd(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::spacing::SpacingModule;
    use crate::domain::wall::{CornerSense, JunctionType, WallRole};
    use crate::keys::{FaceRef, WallId};
    use geometry_kernel::{EntityId, Segment, TickVec3};

    const FT: i32 = 384;

    fn wall(id: u128, ax: i32, ay: i32, bx: i32, by: i32) -> Wall {
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
            Tick(STUD_WIDE),
            WallRole::Bearing,
            SpacingModule::inches(16),
        )
    }

    fn corner(owner: WallId, other: WallId, method: JunctionMethod) -> Junction {
        Junction {
            junction_type: JunctionType::Corner,
            walls: vec![owner, other],
            owner_wall: owner,
            method,
            sense: Some(CornerSense::Outside),
        }
    }

    /// The golden axis-aligned L: through wall (owner, id 1) running into the vertex along world
    /// +x so its outgoing is world +x at the origin; butting wall (id 2) leaving along world +y.
    /// Vertex at the world origin so local == world and the golden footprints land verbatim.
    fn golden_l() -> (Wall, Wall) {
        // owner: (10ft,0) -> (0,0): vertex is b=(0,0), outgoing (a-b) = +x.
        let owner = wall(1, 10 * FT, 0, 0, 0);
        // other: (0,0) -> (0,10ft): vertex is a=(0,0), outgoing (b-a) = +y.
        let other = wall(2, 0, 0, 0, 10 * FT);
        (owner, other)
    }

    #[test]
    fn three_stud_posts_match_the_golden_footprints() {
        let (owner, other) = golden_l();
        let j = corner(WallId(1), WallId(2), JunctionMethod::ThreeStud);
        let detail = detail_junction(&j, &[&owner, &other]);

        assert_eq!(detail.posts.len(), 3, "three-stud emits three posts");
        // S1 (0,0)+1.5x3.5 ; S2 (0,3.5)+3.5x1.5 ; S3 (2.0,0)+1.5x3.5 — local == world here.
        let expect = [
            ((0, 0), (STUD_NARROW, STUD_WIDE)),
            ((0, STUD_WIDE), (STUD_WIDE, STUD_NARROW)),
            ((2 * 32, 0), (STUD_NARROW, STUD_WIDE)),
        ];
        for ((mx, my), (sx, sy)) in expect {
            assert!(
                detail
                    .posts
                    .iter()
                    .any(|p| p.min == TickVec2::new(Tick(mx), Tick(my))
                        && p.size == TickVec2::new(Tick(sx), Tick(sy))),
                "expected a post at min ({mx},{my}) size ({sx},{sy}); got {:?}",
                detail.posts
            );
        }
    }

    #[test]
    fn california_backer_is_flat_and_leaves_the_pocket_open() {
        let (owner, other) = golden_l();
        let j = corner(WallId(1), WallId(2), JunctionMethod::California);
        let detail = detail_junction(&j, &[&owner, &other]);

        assert_eq!(detail.posts.len(), 3);
        // S3 California flat backer at (1.5,2.0)+3.5x1.5 (local == world).
        let s3 = DetailedPost {
            min: TickVec2::new(Tick(STUD_NARROW), Tick(2 * 32)),
            size: TickVec2::new(Tick(STUD_WIDE), Tick(STUD_NARROW)),
        };
        assert!(
            detail.posts.contains(&s3),
            "California S3 must be the flat backer (1.5,2.0)+3.5x1.5; got {:?}",
            detail.posts
        );
        // And it must differ from the three-stud upright backer (pocket stays open).
        let three_stud_s3 = DetailedPost {
            min: TickVec2::new(Tick(2 * 32), Tick(0)),
            size: TickVec2::new(Tick(STUD_NARROW), Tick(STUD_WIDE)),
        };
        assert!(!detail.posts.contains(&three_stud_s3));
    }

    #[test]
    fn plate_lap_staggers_between_courses() {
        let (owner, other) = golden_l();
        let j = corner(WallId(1), WallId(2), JunctionMethod::California);

        // Owner's laps: through on course 0, butt on course 1.
        let owner_detail = detail_junction(&j, &[&owner, &other]);
        let o0 = owner_detail.laps.iter().find(|l| l.course == 0).unwrap();
        let o1 = owner_detail.laps.iter().find(|l| l.course == 1).unwrap();
        assert!(
            o0.runs_through,
            "owner runs through the corner cell on course 0"
        );
        assert!(!o1.runs_through, "owner butts on course 1 (staggered)");

        // The other wall (detailed from its own membership) is the mirror image, so exactly one
        // wall covers the corner cell per course.
        let other_detail = detail_junction(&j, &[&other, &owner]);
        let x0 = other_detail.laps.iter().find(|l| l.course == 0).unwrap();
        let x1 = other_detail.laps.iter().find(|l| l.course == 1).unwrap();
        assert_ne!(
            o0.runs_through, x0.runs_through,
            "course 0: exactly one through"
        );
        assert_ne!(
            o1.runs_through, x1.runs_through,
            "course 1: exactly one through"
        );
    }

    #[test]
    fn general_corner_transforms_local_recipe_to_world() {
        // Same corner translated to a non-origin vertex and rotated 90deg: owner outgoing world
        // -y, other outgoing world +x. The S1 footprint min must land at the transformed corner.
        let vertex = (5 * FT, 7 * FT);
        // owner: (5ft, 7ft+10ft) -> (5ft, 7ft): vertex is b, outgoing (a-b) = (0,+1)? a-b = (0,+10ft) -> +y.
        let owner = wall(1, 5 * FT, 7 * FT + 10 * FT, 5 * FT, 7 * FT);
        // other: (5ft,7ft) -> (5ft+10ft, 7ft): vertex a, outgoing +x.
        let other = wall(2, 5 * FT, 7 * FT, 5 * FT + 10 * FT, 7 * FT);
        let j = corner(WallId(1), WallId(2), JunctionMethod::ThreeStud);
        let detail = detail_junction(&j, &[&owner, &other]);
        assert_eq!(detail.posts.len(), 3);
        // local ex=+y, ey=+x. S1 local (0..48, 0..112) -> world x in [vertex.x, vertex.x+112],
        // y in [vertex.y, vertex.y+48]. min corner = vertex exactly.
        let s1 = detail.posts.iter().find(|p| {
            p.size == TickVec2::new(Tick(STUD_WIDE), Tick(STUD_NARROW))
                && p.min == TickVec2::new(Tick(vertex.0), Tick(vertex.1))
        });
        assert!(
            s1.is_some(),
            "S1 min must sit at the world vertex; got {:?}",
            detail.posts
        );
    }
}
