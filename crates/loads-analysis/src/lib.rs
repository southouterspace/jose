//! # loads-analysis
//!
//! The **Loads & Analysis** bounded context — the new pipeline stage between Placement and the
//! Structural Check. It converts a placed framing model into per-member structural **demand**,
//! consolidating the four scattered load fragments into one coherent concern: load **sources**
//! ([`DeadLoad`], [`LiveLoad`], [`SnowLoad`], [`WindLoad`], [`SeismicLoad`]) quantified per
//! ASCE 7 / IRC; [`TributaryArea`] attributing area to each member; [`LoadPath`] walking the
//! connection graph; [`LoadRollup`] accumulating demand down it; [`LoadCombination`] factoring the
//! source set; and [`MemberDemand`] — the layer's only downstream product.
//!
//! The layer is deliberately **material-blind**: it produces axial/moment/shear/deflection demand
//! plus the serviceability deflection *limit* in real engineering units; *which* combination
//! governs and the `CD`/deflection-capacity decisions are delegated to the `DesignStandard`
//! strategy, so cold-formed steel / concrete / masonry plug in later without touching this core.
//!
//! ## Dependency direction
//!
//! Although the schema has this layer referencing a few design-standard concepts (the connection
//! graph it walks, the ASD/LRFD philosophy a combination points at, the strategy the solver
//! delegates to), the crate stays **upstream**: those are opaque [`ConnectionGraphRef`] /
//! [`DesignPhilosophyRef`] / [`DesignStandardRef`] handles, resolved downstream, never imports.

mod application;
mod domain;
mod keys;

pub use application::load_path::{LoadEdge, LoadPath, PathTerminal, ShareRule};
pub use application::load_rollup::{LoadRollup, RollupMethod};
pub use application::load_solver::{LoadSolver, RecomputeMode};
pub use domain::combination::{CombinationTerm, DurationClass, LoadCombination};
pub use domain::demand::{AccumulatedDemand, MemberDemand, MemberRole};
pub use domain::sources::{
    DeadLoad, Effect, LiveLoad, LiveOccupancy, LoadSource, LoadSourcePayload, SeismicLoad,
    SnowLoad, SourceKind, WindExposure, WindLoad, WindMethod,
};
pub use domain::tributary::{TributaryArea, TributaryShape};
pub use keys::{ConnectionGraphRef, DesignPhilosophyRef, DesignStandardRef};
