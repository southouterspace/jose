//! # drawings-export
//!
//! The **Drawings Export** bounded context — the `drawings-export` layer of the domain MODEL and
//! the terminal pipeline stage. Projection + hidden-line removal over the BREP solid (heavy
//! geometry, kept here next to the kernel) hand clean linework to sheet composition (line weights,
//! dimensions, title block, SVG/PDF — the JS side). It is a parallel downstream consumer to
//! estimating: one-way, it never feeds back.
//!
//! ## The two stages
//!
//! - **Geometry (here, Rust).** [`Projection`] flattens [`geometry_kernel::Volume`]s to 2D
//!   plane-local linework along a [`ViewType`] camera; [`HiddenLineRemoval`] collapses coincident /
//!   occluded edges. The output is a [`DrawingView`].
//! - **Composition (the consumer).** [`DrawingView`]s + [`Dimension`]s + a [`TitleBlock`] place onto
//!   a [`Sheet`], and ordered sheets form the deliverable [`DrawingSet`] per the National CAD
//!   Standard.
//!
//! Linework stays plane-local integer ticks; the only reals are derived sheet quantities (a
//! [`Dimension`]'s inch value, the paper scale), converted from ticks exactly once — the same
//! base-unit discipline the engine keeps everywhere.

mod application;
mod domain;
mod keys;

pub use application::projection::{HiddenLineRemoval, Projection};
pub use domain::annotation::{Dimension, TitleBlock};
pub use domain::sheet::{DrawingSet, PaperSize, Sheet};
pub use domain::view::{DrawingView, ViewType};
pub use keys::{DrawingSetId, ProjectRef, SheetId};
