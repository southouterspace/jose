//! Rigid placement and axis-aligned bounds.

use crate::rotation::Quat;
use crate::vector::TickVec3;

/// Rigid placement in 3D world space: a translation (`origin`, in ticks) plus a `rotation`
/// (unit quaternion). Decoupled from geometry so one shared mesh/spec serves thousands of
/// placements — the Flyweight split (intrinsic geometry, contextual transform).
///
/// > Note: the MODEL card for `Transform` lists a spurious `axisOrigin : f64 (USD)` field —
/// > a data glitch (a money unit on a geometry type). The type's own description defines it
/// > as *origin + rotation*, which is what this implements. Flagged for a schema fix.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Transform {
    pub origin: TickVec3,
    pub rotation: Quat,
}

impl Transform {
    /// The identity placement: at the world origin, unrotated.
    pub const IDENTITY: Transform = Transform {
        origin: TickVec3::ZERO,
        rotation: Quat::IDENTITY,
    };

    /// A pure translation to `origin`, unrotated.
    #[inline]
    pub fn at(origin: TickVec3) -> Transform {
        Transform {
            origin,
            rotation: Quat::IDENTITY,
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Transform::IDENTITY
    }
}

/// An axis-aligned bounding box (AABB) in world ticks: `min` and `max` corners. Used for
/// culling, snapping, broad-phase clash/clearance, and as a geometry digest.
///
/// The invariant `min <= max` component-wise holds for every constructed box.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BoundingBox {
    min: TickVec3,
    max: TickVec3,
}

impl BoundingBox {
    /// Build a box from two opposite corners in any order; components are sorted so the
    /// `min <= max` invariant holds.
    pub fn new(a: TickVec3, b: TickVec3) -> BoundingBox {
        BoundingBox {
            min: a.min(b),
            max: a.max(b),
        }
    }

    /// The tight box around a set of points. Returns `None` for an empty set.
    pub fn from_points(points: impl IntoIterator<Item = TickVec3>) -> Option<BoundingBox> {
        let mut it = points.into_iter();
        let first = it.next()?;
        let mut bb = BoundingBox {
            min: first,
            max: first,
        };
        for p in it {
            bb.min = bb.min.min(p);
            bb.max = bb.max.max(p);
        }
        Some(bb)
    }

    #[inline]
    pub fn min(&self) -> TickVec3 {
        self.min
    }
    #[inline]
    pub fn max(&self) -> TickVec3 {
        self.max
    }

    /// Whether a point lies inside or on the box.
    pub fn contains(&self, p: TickVec3) -> bool {
        p.x >= self.min.x
            && p.x <= self.max.x
            && p.y >= self.min.y
            && p.y <= self.max.y
            && p.z >= self.min.z
            && p.z <= self.max.z
    }

    /// The smallest box containing both inputs.
    pub fn union(&self, other: &BoundingBox) -> BoundingBox {
        BoundingBox {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// The eight corners, for projection / clash tests.
    pub fn corners(&self) -> [TickVec3; 8] {
        let (lo, hi) = (self.min, self.max);
        [
            TickVec3::new(lo.x, lo.y, lo.z),
            TickVec3::new(hi.x, lo.y, lo.z),
            TickVec3::new(lo.x, hi.y, lo.z),
            TickVec3::new(hi.x, hi.y, lo.z),
            TickVec3::new(lo.x, lo.y, hi.z),
            TickVec3::new(hi.x, lo.y, hi.z),
            TickVec3::new(lo.x, hi.y, hi.z),
            TickVec3::new(hi.x, hi.y, hi.z),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tick::Tick;

    #[test]
    fn box_sorts_corners() {
        let bb = BoundingBox::new(
            TickVec3::new(Tick(10), Tick(0), Tick(5)),
            TickVec3::new(Tick(0), Tick(8), Tick(-3)),
        );
        assert_eq!(bb.min(), TickVec3::new(Tick(0), Tick(0), Tick(-3)));
        assert_eq!(bb.max(), TickVec3::new(Tick(10), Tick(8), Tick(5)));
    }

    #[test]
    fn from_points_and_contains() {
        let pts = [
            TickVec3::ZERO,
            TickVec3::new(Tick(32), Tick(32), Tick(32)),
            TickVec3::new(Tick(-16), Tick(8), Tick(0)),
        ];
        let bb = BoundingBox::from_points(pts).unwrap();
        assert!(bb.contains(TickVec3::new(Tick(0), Tick(8), Tick(16))));
        assert!(!bb.contains(TickVec3::new(Tick(64), Tick(0), Tick(0))));
        assert!(BoundingBox::from_points(std::iter::empty()).is_none());
    }

    #[test]
    fn union_grows_to_cover_both() {
        let a = BoundingBox::new(TickVec3::ZERO, TickVec3::new(Tick(10), Tick(10), Tick(10)));
        let b = BoundingBox::new(
            TickVec3::new(Tick(5), Tick(5), Tick(5)),
            TickVec3::new(Tick(20), Tick(20), Tick(20)),
        );
        let u = a.union(&b);
        assert_eq!(u.max(), TickVec3::new(Tick(20), Tick(20), Tick(20)));
        assert_eq!(u.corners().len(), 8);
    }
}
