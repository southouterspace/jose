//! # building
//!
//! The **Building Model + Placement** bounded context — the `building-placement` layer of the
//! domain MODEL. It performs semantic promotion (kernel geometry → [`Wall`] / [`Opening`] /
//! [`Junction`]) and then captures *how each member is installed* ([`MemberPlacement`] + its
//! install context). Shape becomes meaning, then meaning becomes placed members.
//!
//! [`MemberPlacement`] is the canonical intrinsic/contextual seam: a shared material-agnostic
//! `StockSpec` (referenced by key from the materials layer) plus per-instance install context
//! ([`Orientation`], [`BracingRef`], [`EndCondition`]). This layer **originates** the contextual
//! adjustment-factor *inputs* as neutral physical facts but never models the factor stack, any
//! design values, or the load traversal — those are the strategy seam and the loads layer.
//!
//! Stud and member counts are always **derived** by the [`FramingSolver`], never stored: edit the
//! wall and the framer re-runs. The framer anchors its OC grid so a small edit does not reshuffle
//! every stud.
//!
//! ## Dependency direction
//!
//! This context is upstream of the loads layer. The link to the per-member demand a placement
//! carries is held as an opaque [`MemberDemandRef`] handle rather than a `loads-analysis` import,
//! so loads stay single-homed downstream and this crate never depends on the load model.

mod application;
mod domain;
mod keys;

pub use application::framing_solver::{AssemblyKind, FramingSolver, RuleSet};
pub use application::junction_detector::{CornerRules, detect_junctions};
pub use domain::assemblies::{AssemblyFace, Floor, RisePerRun, Roof, Sheathing};
pub use domain::placement::{
    BracedBy, BracingAxis, BracingRef, EndCondition, Fixity, LoadFace, MemberEnd, MemberPlacement,
    Orientation,
};
pub use domain::role::FramingRole;
pub use domain::spacing::{SpacingAnchor, SpacingKey, SpacingModule};
pub use domain::wall::{
    CornerSense, Junction, JunctionMethod, JunctionType, Opening, OpeningType, Wall, WallRole,
};
pub use keys::{
    ConnectionPointRef, FaceRef, FloorId, JunctionRef, MemberDemandRef, MemberPlacementId, RoofId,
    SheathingId, WallId, WallRef,
};
