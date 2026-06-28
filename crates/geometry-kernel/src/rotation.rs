//! Rotations as unit quaternions.

use crate::GEOM_EPSILON;
use crate::vector::UnitVec3;

/// A rotation as a unit quaternion `(x, y, z, w)`. Unitless. The canonical rotation
/// representation, replacing any `quat | euler` ambiguity.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quat {
    /// The identity rotation (no rotation).
    pub const IDENTITY: Quat = Quat {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };

    /// A rotation of `radians` about `axis`.
    pub fn from_axis_angle(axis: UnitVec3, radians: f32) -> Quat {
        let half = radians * 0.5;
        let s = half.sin();
        Quat {
            x: axis.x() * s,
            y: axis.y() * s,
            z: axis.z() * s,
            w: half.cos(),
        }
        .normalized()
    }

    /// Renormalize to unit length, guarding against accumulated drift. Falls back to
    /// identity for a degenerate (near-zero) quaternion.
    pub fn normalized(self) -> Quat {
        let len = (self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w).sqrt();
        if len < GEOM_EPSILON {
            return Quat::IDENTITY;
        }
        Quat {
            x: self.x / len,
            y: self.y / len,
            z: self.z / len,
            w: self.w / len,
        }
    }

    /// Hamilton product (composition): applying `rhs` then `self`.
    pub fn compose(self, rhs: Quat) -> Quat {
        Quat {
            w: self.w * rhs.w - self.x * rhs.x - self.y * rhs.y - self.z * rhs.z,
            x: self.w * rhs.x + self.x * rhs.w + self.y * rhs.z - self.z * rhs.y,
            y: self.w * rhs.y - self.x * rhs.z + self.y * rhs.w + self.z * rhs.x,
            z: self.w * rhs.z + self.x * rhs.y - self.y * rhs.x + self.z * rhs.w,
        }
        .normalized()
    }
}

impl Default for Quat {
    fn default() -> Self {
        Quat::IDENTITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::PI;

    #[test]
    fn identity_is_unit() {
        let q = Quat::IDENTITY;
        assert!((q.w - 1.0).abs() < GEOM_EPSILON);
    }

    #[test]
    fn axis_angle_is_normalized() {
        let q = Quat::from_axis_angle(UnitVec3::Z, PI / 2.0);
        let len = (q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w).sqrt();
        assert!((len - 1.0).abs() < GEOM_EPSILON);
    }

    #[test]
    fn identity_composition_is_noop() {
        let q = Quat::from_axis_angle(UnitVec3::X, 0.7);
        let r = q.compose(Quat::IDENTITY);
        assert!((q.x - r.x).abs() < GEOM_EPSILON && (q.w - r.w).abs() < GEOM_EPSILON);
    }
}
