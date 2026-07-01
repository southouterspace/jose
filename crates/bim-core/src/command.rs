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
    /// Draw (or redraw) a space footprint: a closed world-XY ring, extruded to a default height.
    DrawFootprint(DrawFootprint),
    /// Edit the current space's footprint ring in place, re-extruding at its current height.
    EditFootprint(EditFootprint),
    /// Push/pull the current volume's top cap by a signed tick distance.
    PushPull(PushPull),
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

/// Draw (or redraw) a space footprint from a **closed** ring of world-XY vertices in ticks
/// (1/32in). The closing edge from the last vertex back to the first is implicit — do not repeat
/// the first vertex. The engine extrudes the ring to a default height to form the space's mass.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DrawFootprint {
    /// The closed ring's vertices, world-XY, ticks. `(x, y)` per vertex.
    pub vertices: Vec<(i32, i32)>,
}

/// Edit the current space's footprint from a **closed** ring of world-XY vertices in ticks — the
/// mutated ring after a vertex move / insert / delete, computed client-side against the render mirror.
/// Same shape as [`DrawFootprint`], but a different intent: unlike a fresh draw (which starts flat),
/// an edit **re-extrudes the ring at the current mass height**, so reshaping an already-pushed
/// footprint changes the mass's plan without flattening it (ADR 0015). The closing edge is implicit.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct EditFootprint {
    /// The edited closed ring's vertices, world-XY, ticks. `(x, y)` per vertex.
    pub vertices: Vec<(i32, i32)>,
}

/// Push/pull a face of the current volume by a signed tick distance (positive = extrude taller,
/// negative = inset shorter). The engine validates `face_index == TOP_FACE` and rejects otherwise.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PushPull {
    /// The target volume's id.
    pub volume_id: u32,
    /// The picked face index — must be the kernel's `TOP_FACE` in this slice.
    pub face_index: u32,
    /// Signed move distance along the face normal, ticks.
    pub distance: i32,
}

/// Why the engine refused a command. Each variant is a *user-correctable* condition (a degenerate
/// footprint, an out-of-model push/pull) — not an internal error. The [`Session`](crate::Session)
/// leaves canonical state untouched on a rejection, and the boundary surfaces the reason so the UI
/// can explain it instead of the command silently doing nothing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RejectReason {
    /// A footprint ring with fewer than three vertices — not a polygon.
    TooFewVertices,
    /// A footprint ring whose vertices enclose no area (collinear or coincident).
    ZeroArea,
    /// A footprint ring that crosses or touches itself (a bowtie) — not a simple boundary.
    SelfIntersecting,
    /// A push/pull aimed at a face other than the top cap.
    NotTopFace,
    /// A push/pull that would drive the mass height to zero or below.
    NonPositiveHeight,
    /// A push/pull with nothing to act on (no footprint drawn yet).
    NoTarget,
}

impl RejectReason {
    /// A stable machine code the wasm boundary hands to JS (the UI maps it to human copy). Kept in
    /// lock-step with the `rejectionMessage` table on the TS side.
    pub const fn code(self) -> &'static str {
        match self {
            RejectReason::TooFewVertices => "too_few_vertices",
            RejectReason::ZeroArea => "zero_area",
            RejectReason::SelfIntersecting => "self_intersecting",
            RejectReason::NotTopFace => "not_top_face",
            RejectReason::NonPositiveHeight => "non_positive_height",
            RejectReason::NoTarget => "no_target",
        }
    }
}

/// The result of applying a [`Command`]: either it changed canonical state (carrying the new live
/// member count) or it was refused with a [`RejectReason`] and state is unchanged.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[must_use]
pub enum CommandOutcome {
    /// The command mutated canonical state; `member_count` is the new live member row count.
    Accepted { member_count: usize },
    /// The command was refused; canonical state is unchanged.
    Rejected { reason: RejectReason },
}

impl CommandOutcome {
    /// Whether the command changed canonical state.
    pub const fn is_accepted(self) -> bool {
        matches!(self, CommandOutcome::Accepted { .. })
    }

    /// The rejection reason, or `None` when the command was accepted.
    pub const fn reason(self) -> Option<RejectReason> {
        match self {
            CommandOutcome::Rejected { reason } => Some(reason),
            CommandOutcome::Accepted { .. } => None,
        }
    }

    /// The live member count after an accepted command, or `0` for a rejection.
    pub const fn member_count(self) -> usize {
        match self {
            CommandOutcome::Accepted { member_count } => member_count,
            CommandOutcome::Rejected { .. } => 0,
        }
    }
}
