//! 1D and 2D curve primitives: world segments and in-plane profiles.

use crate::tick::TICKS_PER_INCH;
use crate::vector::{TickVec2, TickVec3};

/// A straight line segment between two committed world points — the atomic 1D world
/// primitive (a wall baseline, an edge).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Segment {
    pub a: TickVec3,
    pub b: TickVec3,
}

impl Segment {
    #[inline]
    pub const fn new(a: TickVec3, b: TickVec3) -> Segment {
        Segment { a, b }
    }

    /// Length in inches (a derived real — never stored as ticks).
    pub fn length_inches(&self) -> f64 {
        let dx = (self.b.x - self.a.x).to_inches();
        let dy = (self.b.y - self.a.y).to_inches();
        let dz = (self.b.z - self.a.z).to_inches();
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// An ordered list of 2D vertices in a plane's local `(u, v)` system, optionally closed —
/// the profile a [`crate::Volume`] is extruded from.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Path2D {
    vertices: Vec<TickVec2>,
    closed: bool,
}

impl Path2D {
    /// An open polyline through `vertices`.
    pub fn open(vertices: Vec<TickVec2>) -> Path2D {
        Path2D {
            vertices,
            closed: false,
        }
    }

    /// A closed polygon through `vertices` (the closing edge from last back to first is
    /// implicit — do not repeat the first vertex).
    pub fn closed(vertices: Vec<TickVec2>) -> Path2D {
        Path2D {
            vertices,
            closed: true,
        }
    }

    #[inline]
    pub fn vertices(&self) -> &[TickVec2] {
        &self.vertices
    }
    #[inline]
    pub fn is_closed(&self) -> bool {
        self.closed
    }
    #[inline]
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Signed area in in² via the shoelace formula (positive = counter-clockwise). Returns
    /// `0.0` for an open path or one with fewer than three vertices. The result is a
    /// derived real in inches, not ticks².
    pub fn signed_area_in2(&self) -> f64 {
        if !self.closed || self.vertices.len() < 3 {
            return 0.0;
        }
        let scale = 1.0 / TICKS_PER_INCH as f64;
        let mut acc: i128 = 0;
        let n = self.vertices.len();
        for i in 0..n {
            let p = self.vertices[i];
            let q = self.vertices[(i + 1) % n];
            acc += p.u.raw() as i128 * q.v.raw() as i128 - q.u.raw() as i128 * p.v.raw() as i128;
        }
        // acc is in ticks²; convert to in² (scale²) and halve.
        (acc as f64) * 0.5 * scale * scale
    }

    /// Unsigned area in in².
    pub fn area_in2(&self) -> f64 {
        self.signed_area_in2().abs()
    }

    /// Whether a **closed** ring is *simple*: no two non-adjacent edges touch or cross. A
    /// self-intersecting outline (a bowtie, a figure-eight) is not a well-formed footprint — the
    /// extrusion kernel and the framer both assume a simple boundary. Open paths and rings with
    /// fewer than three vertices are trivially simple (there is nothing to cross).
    ///
    /// Exact integer arithmetic: the orientation test is a cross product of tick coordinates, widened
    /// to `i128` so it can never overflow, so the answer is a decision, not a tolerance.
    pub fn is_simple(&self) -> bool {
        let n = self.vertices.len();
        if !self.closed || n < 3 {
            return true;
        }
        for i in 0..n {
            let a1 = self.vertices[i];
            let a2 = self.vertices[(i + 1) % n];
            // Only test each unordered edge pair once; skip adjacent edges (they legitimately share a
            // vertex). Edge i touches edge i+1, and edge 0 wraps to touch edge n-1.
            for j in (i + 2)..n {
                if i == 0 && j == n - 1 {
                    continue;
                }
                let b1 = self.vertices[j];
                let b2 = self.vertices[(j + 1) % n];
                if segments_intersect(a1, a2, b1, b2) {
                    return false;
                }
            }
        }
        true
    }
}

/// Twice the signed area of triangle `abc` (the 2D cross product `ab × ac`), in `i128` so the
/// tick-coordinate products never overflow. `> 0` = counter-clockwise, `< 0` = clockwise, `0` =
/// collinear.
fn orient(a: TickVec2, b: TickVec2, c: TickVec2) -> i128 {
    let abx = i128::from(b.u.raw()) - i128::from(a.u.raw());
    let aby = i128::from(b.v.raw()) - i128::from(a.v.raw());
    let acx = i128::from(c.u.raw()) - i128::from(a.u.raw());
    let acy = i128::from(c.v.raw()) - i128::from(a.v.raw());
    abx * acy - aby * acx
}

/// Whether point `p`, already known to be collinear with segment `ab`, lies within its bounding box
/// (i.e. actually on the segment, not on its extension).
fn on_segment(a: TickVec2, b: TickVec2, p: TickVec2) -> bool {
    let (ax, ay, bx, by, px, py) = (
        a.u.raw(),
        a.v.raw(),
        b.u.raw(),
        b.v.raw(),
        p.u.raw(),
        p.v.raw(),
    );
    px >= ax.min(bx) && px <= ax.max(bx) && py >= ay.min(by) && py <= ay.max(by)
}

/// Whether segments `p1p2` and `p3p4` intersect — the classic orientation test, including the
/// collinear-overlap and shared-endpoint (touching) cases, so a ring that merely grazes itself is
/// still reported as non-simple.
fn segments_intersect(p1: TickVec2, p2: TickVec2, p3: TickVec2, p4: TickVec2) -> bool {
    let d1 = orient(p3, p4, p1);
    let d2 = orient(p3, p4, p2);
    let d3 = orient(p1, p2, p3);
    let d4 = orient(p1, p2, p4);
    if ((d1 > 0 && d2 < 0) || (d1 < 0 && d2 > 0)) && ((d3 > 0 && d4 < 0) || (d3 < 0 && d4 > 0)) {
        return true;
    }
    (d1 == 0 && on_segment(p3, p4, p1))
        || (d2 == 0 && on_segment(p3, p4, p2))
        || (d3 == 0 && on_segment(p1, p2, p3))
        || (d4 == 0 && on_segment(p1, p2, p4))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tick::Tick;

    #[test]
    fn segment_length_is_pythagorean() {
        let s = Segment::new(
            TickVec3::ZERO,
            TickVec3::new(Tick(96), Tick(128), Tick(0)), // 3in, 4in -> 5in
        );
        assert!((s.length_inches() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn signed_area_of_unit_square() {
        // 1ft x 1ft square = 144 in². 1ft = 384 ticks.
        let ft = Tick(384);
        let square = Path2D::closed(vec![
            TickVec2::new(Tick(0), Tick(0)),
            TickVec2::new(ft, Tick(0)),
            TickVec2::new(ft, ft),
            TickVec2::new(Tick(0), ft),
        ]);
        assert!((square.signed_area_in2() - 144.0).abs() < 1e-9);
    }

    #[test]
    fn clockwise_area_is_negative() {
        let ft = Tick(384);
        let cw = Path2D::closed(vec![
            TickVec2::new(Tick(0), Tick(0)),
            TickVec2::new(Tick(0), ft),
            TickVec2::new(ft, ft),
            TickVec2::new(ft, Tick(0)),
        ]);
        assert!(cw.signed_area_in2() < 0.0);
        assert!((cw.area_in2() - 144.0).abs() < 1e-9);
    }

    #[test]
    fn open_or_degenerate_path_has_zero_area() {
        assert_eq!(Path2D::open(vec![TickVec2::ZERO]).signed_area_in2(), 0.0);
        assert_eq!(
            Path2D::closed(vec![TickVec2::ZERO, TickVec2::new(Tick(1), Tick(0))]).signed_area_in2(),
            0.0
        );
    }

    #[test]
    fn convex_ring_is_simple() {
        let ft = Tick(384);
        let square = Path2D::closed(vec![
            TickVec2::new(Tick(0), Tick(0)),
            TickVec2::new(ft, Tick(0)),
            TickVec2::new(ft, ft),
            TickVec2::new(Tick(0), ft),
        ]);
        assert!(square.is_simple());
    }

    #[test]
    fn concave_ring_is_still_simple() {
        // An L-shape: non-convex but non-self-intersecting.
        let ft = 384;
        let l = Path2D::closed(vec![
            TickVec2::new(Tick(0), Tick(0)),
            TickVec2::new(Tick(2 * ft), Tick(0)),
            TickVec2::new(Tick(2 * ft), Tick(ft)),
            TickVec2::new(Tick(ft), Tick(ft)),
            TickVec2::new(Tick(ft), Tick(2 * ft)),
            TickVec2::new(Tick(0), Tick(2 * ft)),
        ]);
        assert!(l.is_simple());
    }

    #[test]
    fn bowtie_ring_is_not_simple() {
        // The classic self-crossing quad: edges (v0->v1) and (v2->v3) cross in the middle.
        let ft = 384;
        let bowtie = Path2D::closed(vec![
            TickVec2::new(Tick(0), Tick(0)),
            TickVec2::new(Tick(ft), Tick(ft)),
            TickVec2::new(Tick(ft), Tick(0)),
            TickVec2::new(Tick(0), Tick(ft)),
        ]);
        assert!(!bowtie.is_simple());
    }

    #[test]
    fn ring_touching_itself_is_not_simple() {
        // A non-adjacent vertex landing on another edge counts as a self-touch.
        let ft = 384;
        let touching = Path2D::closed(vec![
            TickVec2::new(Tick(0), Tick(0)),
            TickVec2::new(Tick(2 * ft), Tick(0)),
            TickVec2::new(Tick(2 * ft), Tick(2 * ft)),
            TickVec2::new(Tick(ft), Tick(0)), // sits on the first edge
        ]);
        assert!(!touching.is_simple());
    }

    #[test]
    fn open_path_is_trivially_simple() {
        let open = Path2D::open(vec![
            TickVec2::new(Tick(0), Tick(0)),
            TickVec2::new(Tick(384), Tick(384)),
            TickVec2::new(Tick(384), Tick(0)),
            TickVec2::new(Tick(0), Tick(384)),
        ]);
        assert!(open.is_simple());
    }
}
