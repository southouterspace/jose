//! Oriented planes and the world coordinate frame.

use crate::tick::Tick;
use crate::vector::{TickVec2, TickVec3, UnitVec3};

/// How a [`Plane`] came to be — provenance for snapping and re-derivation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PlaneSource {
    /// A named world plane: XY, XZ, or YZ through the origin.
    NamedWorld(NamedWorldPlane),
    /// Derived from a solid's face.
    FaceDerived,
    /// Spanned by a sketch's U/V axes.
    UvSpanned,
}

/// The three named world planes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NamedWorldPlane {
    Xy,
    Xz,
    Yz,
}

/// An oriented infinite plane in 3D world space: an origin plus an orthonormal in-plane
/// basis (`basis_u`, `basis_v`) and outward `normal`. THE single shared plane primitive.
///
/// The basis is guaranteed orthonormal and right-handed (`basis_u × basis_v = normal`)
/// because the only constructors enforce it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Plane {
    origin: TickVec3,
    normal: UnitVec3,
    basis_u: UnitVec3,
    basis_v: UnitVec3,
    source: Option<PlaneSource>,
}

impl Plane {
    /// Build a plane from an origin and a `(u, v)` basis. The normal is derived as
    /// `u × v`. Returns `None` unless `u` and `v` are orthonormal (non-parallel,
    /// perpendicular within epsilon).
    pub fn from_basis(
        origin: TickVec3,
        basis_u: UnitVec3,
        basis_v: UnitVec3,
        source: Option<PlaneSource>,
    ) -> Option<Plane> {
        if !basis_u.is_orthogonal_to(basis_v) {
            return None;
        }
        let normal = basis_u.cross(basis_v)?;
        Some(Plane {
            origin,
            normal,
            basis_u,
            basis_v,
            source,
        })
    }

    /// The world XY plane through `origin` (basis X, Y; normal +Z).
    pub fn xy(origin: TickVec3) -> Plane {
        Plane {
            origin,
            normal: UnitVec3::Z,
            basis_u: UnitVec3::X,
            basis_v: UnitVec3::Y,
            source: Some(PlaneSource::NamedWorld(NamedWorldPlane::Xy)),
        }
    }

    /// The world XZ plane through `origin` (basis X, Z; normal +Y... right-handed gives -Y,
    /// so the normal is +Y with basis Z, X — we use basis X, Z and normal X×Z = -Y).
    pub fn xz(origin: TickVec3) -> Plane {
        // X × Z = -Y; keep the basis as authored and let the normal follow right-handedly.
        Plane {
            origin,
            normal: UnitVec3::Y.flipped(),
            basis_u: UnitVec3::X,
            basis_v: UnitVec3::Z,
            source: Some(PlaneSource::NamedWorld(NamedWorldPlane::Xz)),
        }
    }

    /// The world YZ plane through `origin` (basis Y, Z; normal +X).
    pub fn yz(origin: TickVec3) -> Plane {
        Plane {
            origin,
            normal: UnitVec3::X,
            basis_u: UnitVec3::Y,
            basis_v: UnitVec3::Z,
            source: Some(PlaneSource::NamedWorld(NamedWorldPlane::Yz)),
        }
    }

    #[inline]
    pub fn origin(&self) -> TickVec3 {
        self.origin
    }
    #[inline]
    pub fn normal(&self) -> UnitVec3 {
        self.normal
    }
    #[inline]
    pub fn basis_u(&self) -> UnitVec3 {
        self.basis_u
    }
    #[inline]
    pub fn basis_v(&self) -> UnitVec3 {
        self.basis_v
    }
    #[inline]
    pub fn source(&self) -> Option<PlaneSource> {
        self.source
    }

    /// Lift a local `(u, v)` tick point onto this plane, returning the world point. The
    /// in-plane offset is computed in real space and rounded back onto the tick lattice.
    pub fn lift(&self, local: TickVec2) -> TickVec3 {
        self.origin
            .offset(self.basis_u, local.u)
            .offset(self.basis_v, local.v)
    }

    /// Translate the plane's origin along its normal by `distance` ticks (a parallel
    /// offset — the basis and normal are unchanged).
    pub fn offset_along_normal(&self, distance: Tick) -> Plane {
        Plane {
            origin: self.origin.offset(self.normal, distance),
            ..*self
        }
    }

    /// Translate the plane by a world delta (basis and normal unchanged).
    pub fn translated(&self, delta: TickVec3) -> Plane {
        Plane {
            origin: self.origin + delta,
            ..*self
        }
    }
}

/// The global world datum: origin `(0,0,0)` plus the orthonormal X/Y/Z basis everything
/// ultimately resolves axis-locks against. There is one canonical frame, [`CoordinateFrame::WORLD`].
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CoordinateFrame {
    pub origin: TickVec3,
    pub axis_x: UnitVec3,
    pub axis_y: UnitVec3,
    pub axis_z: UnitVec3,
}

impl CoordinateFrame {
    /// The canonical world frame.
    pub const WORLD: CoordinateFrame = CoordinateFrame {
        origin: TickVec3::ZERO,
        axis_x: UnitVec3::X,
        axis_y: UnitVec3::Y,
        axis_z: UnitVec3::Z,
    };
}

impl Default for CoordinateFrame {
    fn default() -> Self {
        CoordinateFrame::WORLD
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_planes_have_consistent_normals() {
        let o = TickVec3::ZERO;
        assert!(Plane::xy(o).normal().dot(UnitVec3::Z) > 0.999);
        assert!(Plane::yz(o).normal().dot(UnitVec3::X) > 0.999);
        // basis_u x basis_v must equal the stored normal for every named plane.
        for p in [Plane::xy(o), Plane::xz(o), Plane::yz(o)] {
            let derived = p.basis_u().cross(p.basis_v()).unwrap();
            assert!(derived.dot(p.normal()) > 0.999, "{p:?}");
        }
    }

    #[test]
    fn from_basis_rejects_non_orthogonal() {
        let skew = UnitVec3::new(1.0, 1.0, 0.0).unwrap();
        assert!(Plane::from_basis(TickVec3::ZERO, UnitVec3::X, skew, None).is_none());
    }

    #[test]
    fn lift_maps_local_to_world_on_xy() {
        let p = Plane::xy(TickVec3::new(Tick(32), Tick(0), Tick(0)));
        let world = p.lift(TickVec2::new(Tick(64), Tick(96)));
        assert_eq!(world, TickVec3::new(Tick(96), Tick(96), Tick(0)));
    }

    #[test]
    fn offset_along_normal_moves_origin() {
        let p = Plane::xy(TickVec3::ZERO).offset_along_normal(Tick(48));
        assert_eq!(p.origin(), TickVec3::new(Tick(0), Tick(0), Tick(48)));
    }
}
