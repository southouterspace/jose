//! The drawings-export value objects + the two entities ([`Sheet`](sheet::Sheet) and
//! [`DrawingSet`](sheet::DrawingSet)).
//!
//! Linework is plane-local tick polylines; the only reals are the derived sheet quantities
//! ([`Dimension`](annotation::Dimension) inches, the paper scale), converted from ticks exactly
//! once — the same base-unit discipline the rest of the engine keeps.

pub mod annotation;
pub mod sheet;
pub mod view;
