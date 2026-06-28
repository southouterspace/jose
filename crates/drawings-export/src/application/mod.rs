//! The drawings-export application services — the Rust-side geometry verbs.
//!
//! [`projection::Projection`] flattens BREP volumes to 2D linework along a camera;
//! [`projection::HiddenLineRemoval`] cleans coincident/occluded edges. Both run next to the kernel
//! and hand finished linework to the JS sheet-composition stage.

pub mod projection;
