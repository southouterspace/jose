//! # bim-wasm
//!
//! The **wasm-bindgen boundary** — the Seam, the one place the two machines touch. The Rust worker
//! (the "brain") owns canonical state; the JS main thread (the "hands & eyes") drives it. This
//! crate fixes the protocol between them:
//!
//! - **Channel A — commands.** Low-volume intents JS → Rust ([`Engine::draw_wall`]), returning the
//!   new member count. Compile-time type safety via wasm-bindgen.
//! - **Channel B — state.** The canonical SoA bytes ([`Engine::snapshot`]) the JS render mirror
//!   views read-only. Both sides interpret the bytes through the *same* generated `BufferLayout`
//!   table ([`Engine::layout_hash`] lets JS assert they match), so they cannot drift.
//!
//! All domain logic lives in [`bim_core`] and the context crates; this crate is a thin marshaling
//! shell — the only crate in the workspace that contains FFI `unsafe` (via the wasm-bindgen macro).

use bim_core::{
    Command, CommandOutcome, DrawFootprint, DrawWall, EditFootprint, PushPull, Session,
};
use wasm_bindgen::prelude::*;

/// The engine handle exposed to JavaScript. Wraps the canonical [`Session`]; its methods are the
/// two channels — intents in, SoA bytes out.
#[wasm_bindgen]
#[derive(Debug)]
pub struct Engine {
    session: Session,
}

impl Default for Engine {
    fn default() -> Self {
        Engine::new()
    }
}

#[wasm_bindgen]
impl Engine {
    /// Construct a fresh engine with an empty model.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Engine {
        Engine {
            session: Session::new(),
        }
    }

    /// Channel A: draw (or redraw) the wall from its plan baseline (ticks) at a height (ticks),
    /// framed at an on-center module (real inches). Returns the new live member count.
    #[wasm_bindgen(js_name = drawWall)]
    pub fn draw_wall(
        &mut self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        height: i32,
        spacing_inches: f64,
    ) -> u32 {
        // DrawWall never rejects; fall back to the live count if that ever changes.
        match self.session.apply(Command::DrawWall(DrawWall {
            x0,
            y0,
            x1,
            y1,
            height,
            spacing_inches,
        })) {
            CommandOutcome::Accepted { member_count } => member_count as u32,
            CommandOutcome::Rejected { .. } => self.session.member_count() as u32,
        }
    }

    /// Channel A: draw (or redraw) the current space's footprint from a closed ring of world-XY
    /// vertices in ticks. `xs` and `ys` are parallel columns (one entry per vertex); the closing
    /// edge is implicit. Returns the empty string when accepted, or a stable rejection code (see
    /// [`bim_core::RejectReason::code`]) when the ring is degenerate and canonical state is unchanged.
    #[wasm_bindgen(js_name = drawFootprint)]
    pub fn draw_footprint(&mut self, xs: &[i32], ys: &[i32]) -> String {
        let vertices = xs.iter().zip(ys.iter()).map(|(&x, &y)| (x, y)).collect();
        outcome_code(
            self.session
                .apply(Command::DrawFootprint(DrawFootprint { vertices })),
        )
    }

    /// Channel A: edit the current space's footprint from a closed ring of world-XY vertices in ticks
    /// — the mutated ring after a vertex move / insert / delete. Same ABI as [`Engine::draw_footprint`]
    /// (`xs`/`ys` parallel columns, the closing edge implicit), but the intent is an **edit**: the ring
    /// is re-extruded at the current mass height instead of flattening it (ADR 0015). Returns the empty
    /// string when accepted, or a stable rejection code when the edit is degenerate (state unchanged).
    #[wasm_bindgen(js_name = editFootprint)]
    pub fn edit_footprint(&mut self, xs: &[i32], ys: &[i32]) -> String {
        let vertices = xs.iter().zip(ys.iter()).map(|(&x, &y)| (x, y)).collect();
        outcome_code(
            self.session
                .apply(Command::EditFootprint(EditFootprint { vertices })),
        )
    }

    /// Channel A: push/pull a face of the current volume by a signed tick distance (positive =
    /// extrude, negative = inset). The engine validates `face_index` is the top cap. Returns the
    /// empty string when accepted, or a stable rejection code when the move is refused.
    #[wasm_bindgen(js_name = pushPull)]
    pub fn push_pull(&mut self, volume_id: u32, face_index: u32, distance: i32) -> String {
        outcome_code(self.session.apply(Command::PushPull(PushPull {
            volume_id,
            face_index,
            distance,
        })))
    }

    /// Channel A: step back to the previous space state. Returns `true` when the model changed (so
    /// the caller reships the snapshot), `false` when there was nothing to undo.
    #[wasm_bindgen(js_name = undo)]
    pub fn undo(&mut self) -> bool {
        self.session.undo()
    }

    /// Channel A: reinstate the most recently undone space state. Returns `true` when it changed.
    #[wasm_bindgen(js_name = redo)]
    pub fn redo(&mut self) -> bool {
        self.session.redo()
    }

    /// Whether there is a prior state to undo to — for the toolbar's Undo enablement.
    #[wasm_bindgen(js_name = canUndo)]
    pub fn can_undo(&self) -> bool {
        self.session.can_undo()
    }

    /// Whether there is an undone state to redo — for the toolbar's Redo enablement.
    #[wasm_bindgen(js_name = canRedo)]
    pub fn can_redo(&self) -> bool {
        self.session.can_redo()
    }

    /// The live member count.
    #[wasm_bindgen(js_name = memberCount)]
    pub fn member_count(&self) -> u32 {
        self.session.member_count() as u32
    }

    /// The live footprint vertex count.
    #[wasm_bindgen(js_name = footprintCount)]
    pub fn footprint_count(&self) -> u32 {
        self.session.footprint_count() as u32
    }

    /// The live volume (mass) count.
    #[wasm_bindgen(js_name = volumeCount)]
    pub fn volume_count(&self) -> u32 {
        self.session.volume_count() as u32
    }

    /// Channel B: a copy of the canonical SoA buffer bytes for the render mirror to view.
    ///
    /// The zero-copy path — exposing the wasm linear memory directly as a `SharedArrayBuffer` — is
    /// gated on cross-origin isolation; a per-recompute snapshot copy is the honest first slice and
    /// the reader code is identical either way.
    pub fn snapshot(&self) -> Vec<u8> {
        self.session.buffer_bytes().to_vec()
    }

    /// Channel B: a copy of the canonical footprint SoA bytes for the plan view to read.
    #[wasm_bindgen(js_name = footprintSnapshot)]
    pub fn footprint_snapshot(&self) -> Vec<u8> {
        self.session.footprint_bytes().to_vec()
    }

    /// Channel B: a copy of the canonical volume SoA bytes for the 3D view to read.
    #[wasm_bindgen(js_name = volumeSnapshot)]
    pub fn volume_snapshot(&self) -> Vec<u8> {
        self.session.volume_bytes().to_vec()
    }

    /// The generated layout digest. JS compares this against its own generated `LAYOUT_HASH` at
    /// startup so a schema/codegen mismatch fails loudly instead of corrupting reads.
    #[wasm_bindgen(js_name = layoutHash)]
    pub fn layout_hash(&self) -> String {
        bim_core::layout::LAYOUT_HASH.to_owned()
    }

    /// Bytes per logical element — capacity math for the JS side.
    #[wasm_bindgen(js_name = elementStride)]
    pub fn element_stride(&self) -> u32 {
        bim_core::layout::member_placement::ELEMENT_STRIDE as u32
    }
}

/// Marshal a [`CommandOutcome`] into the boundary's string convention: `""` when accepted, or the
/// [`RejectReason`](bim_core::RejectReason) code when refused. Keeps the wasm signatures a single
/// `String` (no boxed result class to `free()` per command).
fn outcome_code(outcome: CommandOutcome) -> String {
    match outcome.reason() {
        Some(reason) => reason.code().to_owned(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_draws_and_reports_a_consistent_snapshot() {
        let mut engine = Engine::new();
        assert_eq!(engine.member_count(), 0);

        // 10ft x 8ft wall @ 16in OC (ticks: 1ft = 384).
        let count = engine.draw_wall(0, 0, 10 * 384, 0, 8 * 384, 16.0);
        assert!(count > 0);
        assert_eq!(engine.member_count(), count);

        let bytes = engine.snapshot();
        assert_eq!(
            bytes.len(),
            bim_core::layout::member_placement::BUFFER_BYTES
        );
        assert_eq!(engine.element_stride(), 32);
        assert_eq!(engine.layout_hash(), bim_core::layout::LAYOUT_HASH);
    }

    #[test]
    fn footprint_and_pushpull_report_rejection_codes() {
        let ft = 384;
        let mut engine = Engine::new();

        // A self-crossing ring is refused with its code; state stays empty.
        let code = engine.draw_footprint(&[0, ft, ft, 0], &[0, ft, 0, ft]);
        assert_eq!(code, "self_intersecting");
        assert_eq!(engine.footprint_count(), 0);

        // A valid square is accepted (empty code).
        let ok = engine.draw_footprint(&[0, ft, ft, 0], &[0, 0, ft, ft]);
        assert_eq!(ok, "");
        assert_eq!(engine.footprint_count(), 4);

        // Push/pull the base face is refused; the top face extrudes.
        assert_eq!(engine.push_pull(1, 0, ft), "not_top_face");
        assert_eq!(engine.push_pull(1, 1, ft), "");
        assert_eq!(engine.volume_count(), 1);
    }

    #[test]
    fn edit_footprint_reshapes_and_reports_rejection_codes() {
        let ft = 384;
        let mut engine = Engine::new();

        // An edit with nothing drawn is refused.
        assert_eq!(
            engine.edit_footprint(&[0, ft, ft], &[0, 0, ft]),
            "no_target"
        );

        // Draw a square, push it into a mass, then edit the ring — the mass survives (still 1 volume).
        engine.draw_footprint(&[0, ft, ft, 0], &[0, 0, ft, ft]);
        assert_eq!(engine.push_pull(1, 1, 3 * ft), "");
        assert_eq!(engine.volume_count(), 1);
        assert_eq!(
            engine.edit_footprint(&[0, 2 * ft, ft, 0], &[0, 0, ft, ft]),
            ""
        );
        assert_eq!(engine.footprint_count(), 4);
        assert_eq!(engine.volume_count(), 1);

        // A degenerate edit (down to two vertices) is refused with its code; state stays.
        assert_eq!(engine.edit_footprint(&[0, ft], &[0, 0]), "too_few_vertices");
        assert_eq!(engine.footprint_count(), 4);
    }

    #[test]
    fn undo_redo_track_availability() {
        let ft = 384;
        let mut engine = Engine::new();
        assert!(!engine.can_undo() && !engine.can_redo());

        engine.draw_footprint(&[0, ft, ft, 0], &[0, 0, ft, ft]);
        assert!(engine.can_undo());
        assert!(engine.undo());
        assert_eq!(engine.footprint_count(), 0);
        assert!(engine.can_redo());
        assert!(engine.redo());
        assert_eq!(engine.footprint_count(), 4);

        // Nothing left to redo.
        assert!(!engine.can_redo());
        assert!(!engine.redo());
    }
}
