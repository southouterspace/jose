//! The BREP push/pull kernel: extruded solids editable face-by-face.
//!
//! Phase 2 implements the **extrusion (prism) model** end-to-end — a closed profile pushed
//! along an axis, with derived volume and a top-face push/pull that changes height. General
//! face tessellation of side walls and non-prism edits land with the drawing workspace
//! (Phase 4); [`GeometryKernel::apply_push_pull`] returns `None` for cases it does not yet
//! model rather than fabricating geometry.

use crate::curve::Path2D;
use crate::plane::Plane;
use crate::tick::Tick;
use crate::transform::BoundingBox;
use crate::vector::{TickVec3, UnitVec3};

/// An opaque stable identifier for a domain entity (e.g. a [`Volume`]).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct EntityId(pub u128);

/// A bounded planar region of a [`Volume`]: a stable index, an outward normal, the [`Plane`]
/// it lies in, its boundary loop, and its derived area.
#[derive(Clone, PartialEq, Debug)]
pub struct Face {
    pub index: u32,
    pub normal: UnitVec3,
    pub plane: Plane,
    pub boundary: Path2D,
    pub area_in2: f64,
    pub inferred_role: Option<FaceRole>,
}

impl Face {
    /// Build a face, deriving its area from the boundary.
    pub fn new(index: u32, normal: UnitVec3, plane: Plane, boundary: Path2D) -> Face {
        let area_in2 = boundary.area_in2();
        Face {
            index,
            normal,
            plane,
            boundary,
            area_in2,
            inferred_role: None,
        }
    }

    /// This face's boundary lifted into world space.
    pub fn world_points(&self) -> impl Iterator<Item = TickVec3> + '_ {
        self.boundary.vertices().iter().map(|&v| self.plane.lift(v))
    }
}

/// A coarse semantic role inferred for a face. Material-blind; the building context refines it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FaceRole {
    Wall,
    Floor,
    Ceiling,
    Roof,
    Other,
}

/// Whether a push/pull adds material (extrude) or removes it (inset).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PushPullMode {
    Extrude,
    Inset,
}

/// An immutable record of one push/pull edit: move a target face along its normal by a
/// signed tick distance. Stored in [`Volume::edits`] as a replayable history.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PushPullOp {
    pub target_face_volume: EntityId,
    pub target_face_index: u32,
    pub distance: Tick,
    pub mode: PushPullMode,
}

/// The immutable record of the geometric change a push/pull applied.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct VolumeDelta {
    pub added_faces: Vec<u32>,
    pub removed_faces: Vec<u32>,
    pub moved_face_delta: Option<Tick>,
    pub dirty_range: Option<BoundingBox>,
}

/// Face index of the base (bottom) cap of an extruded volume.
pub const BASE_FACE: u32 = 0;
/// Face index of the top cap of an extruded volume.
pub const TOP_FACE: u32 = 1;

/// An extruded BREP solid: a closed profile pushed along an axis, editable face-by-face.
#[derive(Clone, PartialEq, Debug)]
pub struct Volume {
    pub id: EntityId,
    pub profile: Path2D,
    pub base_plane: Plane,
    pub extrude_axis: UnitVec3,
    pub height: Tick,
    pub faces: Vec<Face>,
    pub edits: Vec<PushPullOp>,
}

impl Volume {
    /// Base profile area in in².
    pub fn base_area_in2(&self) -> f64 {
        self.profile.area_in2()
    }

    /// Enclosed volume in in³ (base area × height) — a derived real.
    pub fn volume_in3(&self) -> f64 {
        self.base_area_in2() * self.height.to_inches().abs()
    }

    /// Tight world-space AABB over every face boundary.
    pub fn world_bounds(&self) -> Option<BoundingBox> {
        BoundingBox::from_points(self.faces.iter().flat_map(|f| f.world_points()))
    }
}

/// The kernel mutation verb: stateless BREP operations on [`Volume`]s.
#[derive(Clone, Copy, Debug, Default)]
pub struct GeometryKernel;

impl GeometryKernel {
    pub const fn new() -> GeometryKernel {
        GeometryKernel
    }

    /// Extrude a closed profile along `axis` by `height`, producing a prism with a base and
    /// top cap face. Returns `None` unless the profile is a closed polygon with positive
    /// area and `height > 0`.
    pub fn extrude(
        &self,
        id: EntityId,
        profile: Path2D,
        base_plane: Plane,
        axis: UnitVec3,
        height: Tick,
    ) -> Option<Volume> {
        if !profile.is_closed() || profile.vertex_count() < 3 || profile.area_in2() <= 0.0 {
            return None;
        }
        if height.raw() <= 0 {
            return None;
        }

        let base = Face::new(BASE_FACE, axis.flipped(), base_plane, profile.clone());
        let top_plane = base_plane.translated(TickVec3::ZERO.offset(axis, height));
        let top = Face::new(TOP_FACE, axis, top_plane, profile.clone());

        Some(Volume {
            id,
            profile,
            base_plane,
            extrude_axis: axis,
            height,
            faces: vec![base, top],
            edits: Vec::new(),
        })
    }

    /// Apply a push/pull to the top cap of an extruded volume, changing its height. Extrude
    /// grows the solid; inset shrinks it. Returns `None` (and leaves the volume untouched)
    /// if the op targets a different volume, a face other than the top cap, or would drive
    /// the height to zero or below — cases the prism model does not represent.
    pub fn apply_push_pull(&self, volume: &mut Volume, op: PushPullOp) -> Option<VolumeDelta> {
        if op.target_face_volume != volume.id || op.target_face_index != TOP_FACE {
            return None;
        }
        let signed = match op.mode {
            PushPullMode::Extrude => op.distance.raw(),
            PushPullMode::Inset => -op.distance.raw(),
        };
        let new_height = Tick(volume.height.raw() + signed);
        if new_height.raw() <= 0 {
            return None;
        }

        volume.height = new_height;
        let top_plane = volume
            .base_plane
            .translated(TickVec3::ZERO.offset(volume.extrude_axis, new_height));
        if let Some(top) = volume.faces.iter_mut().find(|f| f.index == TOP_FACE) {
            top.plane = top_plane;
        }
        volume.edits.push(op);

        Some(VolumeDelta {
            moved_face_delta: Some(Tick(signed)),
            dirty_range: volume.world_bounds(),
            ..VolumeDelta::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::TickVec2;

    fn unit_square_prism() -> (GeometryKernel, Volume) {
        let ft = Tick(384);
        let profile = Path2D::closed(vec![
            TickVec2::new(Tick(0), Tick(0)),
            TickVec2::new(ft, Tick(0)),
            TickVec2::new(ft, ft),
            TickVec2::new(Tick(0), ft),
        ]);
        let k = GeometryKernel::new();
        let v = k
            .extrude(
                EntityId(1),
                profile,
                Plane::xy(TickVec3::ZERO),
                UnitVec3::Z,
                ft, // 1ft tall
            )
            .unwrap();
        (k, v)
    }

    #[test]
    fn extrude_builds_a_prism_with_caps() {
        let (_, v) = unit_square_prism();
        assert_eq!(v.faces.len(), 2);
        assert!((v.base_area_in2() - 144.0).abs() < 1e-9);
        // 1ft cube = 12in^3 * 12 * 12 = 1728 in^3.
        assert!((v.volume_in3() - 1728.0).abs() < 1e-6);
        // top cap sits one foot up.
        let top = v.faces.iter().find(|f| f.index == TOP_FACE).unwrap();
        assert_eq!(
            top.plane.origin(),
            TickVec3::new(Tick(0), Tick(0), Tick(384))
        );
    }

    #[test]
    fn extrude_rejects_degenerate_input() {
        let k = GeometryKernel::new();
        let open = Path2D::open(vec![TickVec2::ZERO, TickVec2::new(Tick(10), Tick(0))]);
        assert!(
            k.extrude(
                EntityId(1),
                open,
                Plane::xy(TickVec3::ZERO),
                UnitVec3::Z,
                Tick(10)
            )
            .is_none()
        );
    }

    #[test]
    fn push_pull_extrude_grows_height() {
        let (k, mut v) = unit_square_prism();
        let delta = k
            .apply_push_pull(
                &mut v,
                PushPullOp {
                    target_face_volume: EntityId(1),
                    target_face_index: TOP_FACE,
                    distance: Tick(384),
                    mode: PushPullMode::Extrude,
                },
            )
            .unwrap();
        assert_eq!(v.height, Tick(768)); // now 2ft
        assert_eq!(delta.moved_face_delta, Some(Tick(384)));
        assert_eq!(v.edits.len(), 1);
        assert!(delta.dirty_range.is_some());
    }

    #[test]
    fn push_pull_inset_below_zero_is_rejected() {
        let (k, mut v) = unit_square_prism();
        let before = v.height;
        let res = k.apply_push_pull(
            &mut v,
            PushPullOp {
                target_face_volume: EntityId(1),
                target_face_index: TOP_FACE,
                distance: Tick(999),
                mode: PushPullMode::Inset,
            },
        );
        assert!(res.is_none());
        assert_eq!(v.height, before); // untouched
    }

    #[test]
    fn push_pull_wrong_target_is_rejected() {
        let (k, mut v) = unit_square_prism();
        let res = k.apply_push_pull(
            &mut v,
            PushPullOp {
                target_face_volume: EntityId(999),
                target_face_index: TOP_FACE,
                distance: Tick(10),
                mode: PushPullMode::Extrude,
            },
        );
        assert!(res.is_none());
    }
}
