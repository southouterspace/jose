//! # bim-core
//!
//! The **composition root**: the one place the bounded-context crates are wired into a running
//! pipeline. It is *technical, not a domain* (per the repo plan's `system-architecture` mapping) —
//! it owns no new domain types; it composes the existing contexts and produces the canonical
//! Structure-of-Arrays [`MemberBuffer`] that the JS render mirror reads.
//!
//! ## The draw → recompute → render slice
//!
//! 1. A [`Command`] (the JS → Rust intent) arrives — for the first slice, [`DrawWall`].
//! 2. [`Session::apply`] promotes the baseline into a wall (the `building` context), runs the
//!    `FramingSolver`, and writes every placed member into the SoA buffer.
//! 3. The buffer bytes are handed to JS read-only; `render-mirror` cuts zero-copy typed-array
//!    views over the columns using the *same* generated [`layout`] table the writer used.
//!
//! Because both sides read the one generated [`layout`], the Rust writer and the JS reader
//! **provably cannot drift** — the keystone contract the schema promises. Loads-analysis and the
//! design-standard check are later pipeline stages that slot into [`Session`] without changing the
//! boundary or the buffer contract.

mod buffer;
mod command;
mod session;

/// The generated SoA byte-offset tables — the `BufferLayout` keystone (Rust writer side). Emitted
/// by `tooling/codegen`; regenerate with `bun run codegen`, never edit by hand.
#[path = "generated/layout.rs"]
pub mod layout;

pub use buffer::{
    FootprintBuffer, FootprintRow, MemberBuffer, MemberRow, NOMINAL_WIDTH, VolumeBuffer, VolumeRow,
};
pub use command::{Command, DrawFootprint, DrawWall, PushPull};
pub use session::Session;
