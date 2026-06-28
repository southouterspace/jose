//! Tick-space points and unit directions.

use crate::GEOM_EPSILON;
use crate::tick::Tick;
use core::ops::{Add, Sub};

/// A 2D point in a plane's local `(u, v)` coordinate system, in integer ticks.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct TickVec2 {
    pub u: Tick,
    pub v: Tick,
}

impl TickVec2 {
    pub const ZERO: TickVec2 = TickVec2 {
        u: Tick::ZERO,
        v: Tick::ZERO,
    };

    #[inline]
    pub const fn new(u: Tick, v: Tick) -> Self {
        Self { u, v }
    }
}

impl Add for TickVec2 {
    type Output = TickVec2;
    #[inline]
    fn add(self, rhs: TickVec2) -> TickVec2 {
        TickVec2::new(self.u + rhs.u, self.v + rhs.v)
    }
}
impl Sub for TickVec2 {
    type Output = TickVec2;
    #[inline]
    fn sub(self, rhs: TickVec2) -> TickVec2 {
        TickVec2::new(self.u - rhs.u, self.v - rhs.v)
    }
}

/// A position in 3D world space, stored as three integer tick coordinates. THE canonical
/// type for any committed world point.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct TickVec3 {
    pub x: Tick,
    pub y: Tick,
    pub z: Tick,
}

impl TickVec3 {
    pub const ZERO: TickVec3 = TickVec3 {
        x: Tick::ZERO,
        y: Tick::ZERO,
        z: Tick::ZERO,
    };

    #[inline]
    pub const fn new(x: Tick, y: Tick, z: Tick) -> Self {
        Self { x, y, z }
    }

    /// Component-wise minimum (for AABB construction).
    #[inline]
    pub fn min(self, other: TickVec3) -> TickVec3 {
        TickVec3::new(
            self.x.min(other.x),
            self.y.min(other.y),
            self.z.min(other.z),
        )
    }

    /// Component-wise maximum.
    #[inline]
    pub fn max(self, other: TickVec3) -> TickVec3 {
        TickVec3::new(
            self.x.max(other.x),
            self.y.max(other.y),
            self.z.max(other.z),
        )
    }

    /// Translate this point by a unit direction scaled to `distance` ticks. The offset is
    /// computed in real space and rounded back onto the tick lattice.
    #[inline]
    pub fn offset(self, dir: UnitVec3, distance: Tick) -> TickVec3 {
        let d = distance.raw() as f32;
        TickVec3::new(
            self.x + Tick((dir.x() * d).round() as i32),
            self.y + Tick((dir.y() * d).round() as i32),
            self.z + Tick((dir.z() * d).round() as i32),
        )
    }
}

impl Add for TickVec3 {
    type Output = TickVec3;
    #[inline]
    fn add(self, rhs: TickVec3) -> TickVec3 {
        TickVec3::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}
impl Sub for TickVec3 {
    type Output = TickVec3;
    #[inline]
    fn sub(self, rhs: TickVec3) -> TickVec3 {
        TickVec3::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

/// A unitless direction or normal: a real 3-vector constrained to unit length (`|v| = 1`).
///
/// Construction is the only way to make one, and it normalizes (or rejects a zero vector),
/// so a `UnitVec3` is *always* unit length — orientation only, never a position.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct UnitVec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl UnitVec3 {
    /// World +X / +Y / +Z.
    pub const X: UnitVec3 = UnitVec3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    pub const Y: UnitVec3 = UnitVec3 {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    pub const Z: UnitVec3 = UnitVec3 {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };

    /// Normalize a raw vector to unit length. Returns `None` for a (near-)zero vector,
    /// which has no defined direction.
    pub fn new(x: f32, y: f32, z: f32) -> Option<UnitVec3> {
        let len = (x * x + y * y + z * z).sqrt();
        if len < GEOM_EPSILON {
            return None;
        }
        Some(UnitVec3 {
            x: x / len,
            y: y / len,
            z: z / len,
        })
    }

    #[inline]
    pub fn x(self) -> f32 {
        self.x
    }
    #[inline]
    pub fn y(self) -> f32 {
        self.y
    }
    #[inline]
    pub fn z(self) -> f32 {
        self.z
    }

    /// Dot product. For two unit vectors this is the cosine of the angle between them.
    #[inline]
    pub fn dot(self, other: UnitVec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Cross product, renormalized. Returns `None` if the inputs are parallel (degenerate).
    pub fn cross(self, other: UnitVec3) -> Option<UnitVec3> {
        UnitVec3::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    /// The opposite direction.
    #[inline]
    pub fn flipped(self) -> UnitVec3 {
        UnitVec3 {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }

    /// Whether two directions are perpendicular within [`GEOM_EPSILON`].
    #[inline]
    pub fn is_orthogonal_to(self, other: UnitVec3) -> bool {
        self.dot(other).abs() < GEOM_EPSILON
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tickvec3_arithmetic() {
        let a = TickVec3::new(Tick(32), Tick(0), Tick(0));
        let b = TickVec3::new(Tick(0), Tick(64), Tick(0));
        assert_eq!(a + b, TickVec3::new(Tick(32), Tick(64), Tick(0)));
        assert_eq!((a + b).min(a), a);
        assert_eq!((a + b).max(a), a + b);
    }

    #[test]
    fn unitvec3_is_always_normalized() {
        let v = UnitVec3::new(3.0, 0.0, 4.0).unwrap();
        let len = (v.x() * v.x() + v.y() * v.y() + v.z() * v.z()).sqrt();
        assert!((len - 1.0).abs() < GEOM_EPSILON);
    }

    #[test]
    fn zero_vector_has_no_direction() {
        assert!(UnitVec3::new(0.0, 0.0, 0.0).is_none());
    }

    #[test]
    fn cross_of_axes_is_third_axis() {
        let z = UnitVec3::X.cross(UnitVec3::Y).unwrap();
        assert!(z.dot(UnitVec3::Z) > 1.0 - GEOM_EPSILON);
        assert!(UnitVec3::X.cross(UnitVec3::X).is_none()); // parallel
    }

    #[test]
    fn offset_moves_along_direction() {
        let p = TickVec3::ZERO.offset(UnitVec3::X, Tick(32));
        assert_eq!(p, TickVec3::new(Tick(32), Tick(0), Tick(0)));
    }
}
