//! The [`detect_junctions`] service: a set of [`Wall`]s → the [`Junction`]s where their baselines
//! meet, classified and owned. The missing foundation — without it `bim-core` frames with an empty
//! junction slice and corners never form (ADR 0009 §3).
//!
//! Pure integer-tick plan geometry. Detection finds shared/abutting baseline endpoints; *sense*
//! (convex vs concave) is derived from the signed turn combined with each wall's authored interior
//! face ([`Wall::interior_on_left`]) — never hand-tagged; *owner* is the lower [`WallId`] so framing
//! is stable across recomputes. v1 scope is `Corner` + `Tee`; collinear inline splices and `Cross`
//! are deferred.

use crate::domain::wall::{CornerSense, Junction, JunctionMethod, JunctionType, Wall};
use crate::keys::WallId;
use geometry_kernel::TickVec2;

/// Tolerance (in ticks) for the point-on-segment test that detects a `Tee`. A tick is 1/32in, so
/// this is a hair under 1/32in — endpoints are placed on the lattice, so exact-but-for-rounding
/// abutment still registers.
const TEE_TOLERANCE_TICKS: i64 = 1;

/// The project-default detailing method per junction class, applied parametrically to every
/// detected junction (ADR 0009 §4). A flyweight config: one table, overridable later per junction;
/// no UI in v1.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CornerRules {
    /// Convex corner method (e.g. a rectangle's outside corners).
    pub outside: JunctionMethod,
    /// Concave/reentrant corner method (the elbow of an L-shaped room).
    pub inside: JunctionMethod,
    /// Tee method. `TwoStudClip` is the closest existing variant to ladder-block backing
    /// (ADR 0009 §4 calls for ladder-block backing at tees; modeled as `TwoStudClip` until a
    /// dedicated method lands).
    pub tee: JunctionMethod,
}

impl Default for CornerRules {
    fn default() -> Self {
        CornerRules {
            outside: JunctionMethod::California,
            inside: JunctionMethod::ThreeStud,
            // Ladder-block backing has no dedicated variant yet; `TwoStudClip` is the nearest.
            tee: JunctionMethod::TwoStudClip,
        }
    }
}

/// Find and classify the junctions among `walls` using the default [`CornerRules`].
///
/// A `Corner` is two walls whose baselines share an endpoint (exact tick equality) turning a
/// non-collinear corner; a `Tee` is a wall whose endpoint lands on the interior of another wall's
/// baseline. Collinear inline splices and `Cross` junctions are ignored in v1.
pub fn detect_junctions(walls: &[Wall]) -> Vec<Junction> {
    detect_junctions_with(walls, &CornerRules::default())
}

/// [`detect_junctions`] with an explicit rules table (the override seam).
pub fn detect_junctions_with(walls: &[Wall], rules: &CornerRules) -> Vec<Junction> {
    let mut out = Vec::new();

    // Corners: every unordered pair of walls sharing a baseline endpoint, non-collinear.
    for i in 0..walls.len() {
        for j in (i + 1)..walls.len() {
            if let Some(junction) = classify_corner(&walls[i], &walls[j], rules) {
                out.push(junction);
            }
        }
    }

    // Tees: a wall endpoint landing on the interior of another wall's baseline.
    for (i, stem) in walls.iter().enumerate() {
        for (j, run) in walls.iter().enumerate() {
            if i == j {
                continue;
            }
            if let Some(junction) = classify_tee(stem, run, rules) {
                out.push(junction);
            }
        }
    }

    out
}

/// Plan-projection (drop z) of a wall baseline's endpoints.
fn baseline_2d(w: &Wall) -> (TickVec2, TickVec2) {
    (
        TickVec2::new(w.baseline.a.x, w.baseline.a.y),
        TickVec2::new(w.baseline.b.x, w.baseline.b.y),
    )
}

/// `p - q` as an i64 pair (ticks), promoted so cross/dot can't overflow on long walls.
fn delta(p: TickVec2, q: TickVec2) -> (i64, i64) {
    (
        (p.u.raw() - q.u.raw()) as i64,
        (p.v.raw() - q.v.raw()) as i64,
    )
}

/// 2D cross product (signed parallelogram area). Zero ⇒ parallel/collinear.
fn cross(a: (i64, i64), b: (i64, i64)) -> i64 {
    a.0 * b.1 - a.1 * b.0
}

/// 2D dot product.
fn dot(a: (i64, i64), b: (i64, i64)) -> i64 {
    a.0 * b.0 + a.1 * b.1
}

/// The wall's interior-face normal in plan: the left-hand (+90°) side of the baseline direction
/// `a→b` when [`Wall::interior_on_left`], else the right-hand side. Rotate (x,y) → (-y,x) is +90°.
fn interior_normal(w: &Wall) -> (i64, i64) {
    let (a, b) = baseline_2d(w);
    let d = delta(b, a); // a→b direction
    let left = (-d.1, d.0);
    if w.interior_on_left {
        left
    } else {
        (-left.0, -left.1)
    }
}

/// Classify the corner formed by two walls sharing a baseline endpoint, if any. `None` when they
/// do not share an endpoint or are collinear (an inline splice — ignored in v1).
fn classify_corner(w1: &Wall, w2: &Wall, rules: &CornerRules) -> Option<Junction> {
    let (a1, b1) = baseline_2d(w1);
    let (a2, b2) = baseline_2d(w2);

    // The shared vertex (exact tick equality), and each wall's *outgoing* direction from it.
    let vertex = shared_endpoint((a1, b1), (a2, b2))?;
    let d1 = outgoing(vertex, (a1, b1));
    let d2 = outgoing(vertex, (a2, b2));

    // Collinear → inline splice (or doubling back); not a corner.
    if cross(d1, d2) == 0 {
        return None;
    }

    // Owner = lower id (deterministic, stable framing); sense derived from the owner's interior
    // relative to the *other* wall's outgoing direction.
    let (owner, other_outgoing) = if w1.id <= w2.id { (w1, d2) } else { (w2, d1) };
    let sense = corner_sense(owner, other_outgoing);

    let method = match sense {
        CornerSense::Outside => rules.outside,
        CornerSense::Inside => rules.inside,
    };

    Some(Junction {
        junction_type: JunctionType::Corner,
        walls: ordered_walls(w1.id, w2.id),
        owner_wall: owner.id,
        method,
        sense: Some(sense),
    })
}

/// Convex (`Outside`) vs concave (`Inside`): does the *other* wall's outgoing direction point
/// toward the owner's interior side? Toward ⇒ the corner wraps the interior ⇒ convex.
fn corner_sense(owner: &Wall, other_outgoing: (i64, i64)) -> CornerSense {
    if dot(other_outgoing, interior_normal(owner)) > 0 {
        CornerSense::Outside
    } else {
        CornerSense::Inside
    }
}

/// Classify a tee: `stem`'s endpoint landing on the *interior* (strictly between the endpoints) of
/// `run`'s baseline. `None` otherwise. Owner = lower id.
fn classify_tee(stem: &Wall, run: &Wall, rules: &CornerRules) -> Option<Junction> {
    let (sa, sb) = baseline_2d(stem);
    let (ra, rb) = baseline_2d(run);

    let lands = point_on_segment_interior(sa, ra, rb) || point_on_segment_interior(sb, ra, rb);
    if !lands {
        return None;
    }

    Some(Junction {
        junction_type: JunctionType::Tee,
        walls: ordered_walls(stem.id, run.id),
        owner_wall: stem.id.min(run.id),
        method: rules.tee,
        sense: None,
    })
}

/// The exactly-equal endpoint shared by two segments, if one exists.
fn shared_endpoint(s1: (TickVec2, TickVec2), s2: (TickVec2, TickVec2)) -> Option<TickVec2> {
    let (a1, b1) = s1;
    let (a2, b2) = s2;
    [a1, b1].into_iter().find(|&v| v == a2 || v == b2)
}

/// The wall's direction pointing *away* from the shared `vertex`: if `vertex` is endpoint `a`, that
/// is `a→b`; if it is `b`, that is `b→a`.
fn outgoing(vertex: TickVec2, seg: (TickVec2, TickVec2)) -> (i64, i64) {
    let (a, b) = seg;
    if vertex == a {
        delta(b, a)
    } else {
        delta(a, b)
    }
}

/// Walls in ascending id order so a junction's `walls` list is deterministic.
fn ordered_walls(x: WallId, y: WallId) -> Vec<WallId> {
    if x <= y { vec![x, y] } else { vec![y, x] }
}

/// Whether `p` lies on segment `a→b` strictly between the endpoints (a tee abuts mid-span, not at a
/// shared corner). Collinear within [`TEE_TOLERANCE_TICKS`] and the projection parameter in `(0, 1)`.
fn point_on_segment_interior(p: TickVec2, a: TickVec2, b: TickVec2) -> bool {
    let ab = delta(b, a);
    let ap = delta(p, a);
    let len_sq = dot(ab, ab);
    if len_sq == 0 {
        return false; // degenerate run
    }
    // Off the line? (perpendicular distance via cross, compared to tolerance × |ab|).
    let area = cross(ab, ap).abs();
    if area * area > (TEE_TOLERANCE_TICKS * TEE_TOLERANCE_TICKS) * len_sq {
        return false;
    }
    // Strictly interior: 0 < (ap·ab) < |ab|².
    let proj = dot(ap, ab);
    proj > 0 && proj < len_sq
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::spacing::SpacingModule;
    use crate::domain::wall::WallRole;
    use crate::keys::{FaceRef, WallId};
    use geometry_kernel::{EntityId, Segment, Tick, TickVec3};

    /// A wall from plan endpoints in ticks; `interior_on_left` matches the CCW footprint default
    /// unless overridden by the caller after construction.
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
            Tick(112),
            WallRole::Bearing,
            SpacingModule::inches(16),
        )
    }

    const FT: i32 = 384;

    #[test]
    fn l_corner_is_one_outside_corner_owned_by_lower_id() {
        // Two walls of a CCW footprint meeting at (10ft, 0): interior on the left of each.
        // wall 1: (0,0)→(10ft,0); wall 2: (10ft,0)→(10ft,10ft).
        let w1 = wall(1, 0, 0, 10 * FT, 0);
        let w2 = wall(2, 10 * FT, 0, 10 * FT, 10 * FT);

        let js = detect_junctions(&[w1, w2]);
        assert_eq!(js.len(), 1);
        let j = &js[0];
        assert_eq!(j.junction_type, JunctionType::Corner);
        assert_eq!(j.sense, Some(CornerSense::Outside));
        assert_eq!(j.owner_wall, WallId(1)); // lower id
        assert_eq!(j.method, JunctionMethod::California);
        assert_eq!(j.walls, vec![WallId(1), WallId(2)]);
    }

    #[test]
    fn closed_rectangle_has_four_outside_corners_each_owned_once() {
        // CCW square 10ft × 10ft, four walls, interior on the left of each edge.
        let w1 = wall(1, 0, 0, 10 * FT, 0);
        let w2 = wall(2, 10 * FT, 0, 10 * FT, 10 * FT);
        let w3 = wall(3, 10 * FT, 10 * FT, 0, 10 * FT);
        let w4 = wall(4, 0, 10 * FT, 0, 0);

        let js = detect_junctions(&[w1, w2, w3, w4]);
        let corners: Vec<_> = js
            .iter()
            .filter(|j| j.junction_type == JunctionType::Corner)
            .collect();
        assert_eq!(corners.len(), 4, "exactly four corners");
        assert!(
            corners
                .iter()
                .all(|j| j.sense == Some(CornerSense::Outside)),
            "every corner of a convex room is Outside"
        );
        assert!(
            corners
                .iter()
                .all(|j| j.method == JunctionMethod::California),
            "outside default is California"
        );
        // No junction double-counted: each adjacent wall-pair appears once, owner is the lower id.
        let mut pairs: Vec<Vec<WallId>> = corners.iter().map(|j| j.walls.clone()).collect();
        pairs.sort();
        pairs.dedup();
        assert_eq!(pairs.len(), 4, "no corner counted twice");
        for j in &corners {
            assert_eq!(j.owner_wall, *j.walls.iter().min().unwrap());
        }
    }

    #[test]
    fn l_shaped_room_inner_corner_is_inside() {
        // CCW L-shaped footprint with a reentrant elbow at (6ft, 6ft):
        // (0,0)→(10ft,0)→(10ft,6ft)→(6ft,6ft)→(6ft,10ft)→(0,10ft)→close.
        // The reentrant corner is between the (10,6)→(6,6) edge and the (6,6)→(6,10) edge.
        let a = wall(1, 10 * FT, 6 * FT, 6 * FT, 6 * FT); // runs into the elbow
        let b = wall(2, 6 * FT, 6 * FT, 6 * FT, 10 * FT); // leaves the elbow

        let js = detect_junctions(&[a, b]);
        assert_eq!(js.len(), 1);
        let j = &js[0];
        assert_eq!(j.junction_type, JunctionType::Corner);
        assert_eq!(
            j.sense,
            Some(CornerSense::Inside),
            "reentrant elbow is Inside"
        );
        assert_eq!(j.method, JunctionMethod::ThreeStud, "inside default");
    }

    #[test]
    fn partition_tee_on_mid_span() {
        // A run wall (0,0)→(20ft,0) and a partition ending on its mid-span at (10ft,0).
        let run = wall(1, 0, 0, 20 * FT, 0);
        let partition = wall(2, 10 * FT, 0, 10 * FT, 8 * FT);

        let js = detect_junctions(&[run, partition]);
        let tees: Vec<_> = js
            .iter()
            .filter(|j| j.junction_type == JunctionType::Tee)
            .collect();
        assert_eq!(tees.len(), 1, "exactly one tee");
        let t = tees[0];
        assert_eq!(t.sense, None, "tee carries no sense in v1");
        assert_eq!(t.method, JunctionMethod::TwoStudClip);
        assert_eq!(t.owner_wall, WallId(1));
        // The partition's free endpoint is not on the run, so no spurious corner forms either.
        assert!(js.iter().all(|j| j.junction_type != JunctionType::Corner));
    }

    #[test]
    fn collinear_inline_splice_is_not_a_junction() {
        // Two colinear walls sharing the endpoint (10ft,0): an inline splice, ignored.
        let w1 = wall(1, 0, 0, 10 * FT, 0);
        let w2 = wall(2, 10 * FT, 0, 20 * FT, 0);

        let js = detect_junctions(&[w1, w2]);
        assert!(js.is_empty(), "collinear splice yields no junction");
    }
}
