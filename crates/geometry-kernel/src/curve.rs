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
}
