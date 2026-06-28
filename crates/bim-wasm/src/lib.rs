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

use bim_core::{Command, DrawWall, Session};
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
        self.session.apply(Command::DrawWall(DrawWall {
            x0,
            y0,
            x1,
            y1,
            height,
            spacing_inches,
        })) as u32
    }

    /// The live member count.
    #[wasm_bindgen(js_name = memberCount)]
    pub fn member_count(&self) -> u32 {
        self.session.member_count() as u32
    }

    /// Channel B: a copy of the canonical SoA buffer bytes for the render mirror to view.
    ///
    /// The zero-copy path — exposing the wasm linear memory directly as a `SharedArrayBuffer` — is
    /// gated on cross-origin isolation; a per-recompute snapshot copy is the honest first slice and
    /// the reader code is identical either way.
    pub fn snapshot(&self) -> Vec<u8> {
        self.session.buffer_bytes().to_vec()
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
}
