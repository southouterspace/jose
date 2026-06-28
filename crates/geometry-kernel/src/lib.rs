//! # geometry-kernel
//!
//! The single canonical home for shared geometry — the `geometry-kernel` layer of the
//! domain MODEL (`schema/model/unified-model.json`). Every other context references these
//! types rather than redefining them, which is what resolved the cross-schema `Plane` /
//! `Transform` / `BoundingBox` collisions in the original audit.
//!
//! As a **shared kernel** this crate is pure domain: primitives with invariants and
//! behavior, no ports/adapters, no dependencies. Two invariants run through everything:
//!
//! - **Linear geometry is integer [`Tick`]s** (1/32 inch). World points never drift on
//!   imperial fractions because they are never stored as floats.
//! - **Directions are unit-length** ([`UnitVec3`]) and **plane bases are orthonormal**
//!   ([`Plane`]). These are enforced at construction, not assumed.
//!
//! Area / volume / length are *derived* as real numbers (inches), never stored as ticks².

mod brep;
mod curve;
mod plane;
mod rotation;
mod tick;
mod transform;
mod vector;

pub use brep::{
    BASE_FACE, EntityId, Face, FaceRole, GeometryKernel, PushPullMode, PushPullOp, TOP_FACE,
    Volume, VolumeDelta,
};
pub use curve::{Path2D, Segment};
pub use plane::{CoordinateFrame, Plane, PlaneSource};
pub use rotation::Quat;
pub use tick::{TICKS_PER_FOOT, TICKS_PER_INCH, Tick};
pub use transform::{BoundingBox, Transform};
pub use vector::{TickVec2, TickVec3, UnitVec3};

/// Epsilon for unit-length / orthonormality checks on `f32` direction data.
pub const GEOM_EPSILON: f32 = 1e-4;
