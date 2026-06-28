//! [`Command`] — the immutable intents that cross the JS → Rust command channel (Channel A).
//!
//! One per user action. The wasm boundary translates a JS call into a `Command`; the
//! [`Session`](crate::Session) applies it and rewrites the canonical SoA buffer. Keeping the intent
//! a plain owned value (no geometry-kernel types in the signature) keeps the boundary trivial to
//! marshal.

/// An immutable intent message sent JS → Rust over the command channel.
#[derive(Clone, PartialEq, Debug)]
pub enum Command {
    /// Draw (or redraw) the wall from its plan baseline at a height, framed at an OC module.
    DrawWall(DrawWall),
}

/// Place a wall from two plan-baseline endpoints. Linear inputs are integer ticks (1/32in); the
/// on-center module is a real inch value (19.2in is a valid, non-tick module), exactly as the
/// `SpacingModule` parameter in the building context requires.
#[derive(Clone, PartialEq, Debug)]
pub struct DrawWall {
    /// Baseline start X in plan, ticks.
    pub x0: i32,
    /// Baseline start Y in plan, ticks.
    pub y0: i32,
    /// Baseline end X in plan, ticks.
    pub x1: i32,
    /// Baseline end Y in plan, ticks.
    pub y1: i32,
    /// Wall height, ticks.
    pub height: i32,
    /// On-center layout module, real inches (e.g. 16.0 or 19.2).
    pub spacing_inches: f64,
}
