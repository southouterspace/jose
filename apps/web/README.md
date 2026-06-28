# web

The browser app — the first end-to-end **draw → recompute → render** slice.

```
pointer picks ─▶ @jose/tool-runner ─▶ (Channel A: drawWall) ─▶ engine-worker
                                                                    │
                                                            bim-wasm (Rust)
                                                                    │
   canvas elevation ◀─ @jose/render-mirror ◀─ (Channel B: SoA bytes) ◀┘
```

- **Main thread** (`src/main.ts`, the "hands & eyes") captures pointer picks, runs the
  `ToolRunner` to build a `DrawWall` intent, and renders the read-only mirror to a canvas.
- **Worker** (`src/engine-worker.ts`, the Rust "brain") owns the `bim-wasm` `Engine`, recomputes on
  each command, and ships back the canonical SoA buffer snapshot.
- **The keystone:** both sides read the *same* generated `BufferLayout`; the worker reports its
  `layoutHash` at startup and the main thread `assertLayout`s it, so the writer and reader cannot
  drift.

## Run it

The Rust engine compiles to wasm via [`wasm-pack`](https://rustwasm.github.io/wasm-pack/) — install
it first (`cargo install wasm-pack`). Then:

```bash
bun install
bun run --filter web build:wasm   # crates/bim-wasm → src/wasm/pkg/ (wasm-pack)
bun run --filter web build        # bundle src/main.ts → dist/
bun run --filter web dev          # build wasm + bundle + serve on :5173
```

Open the served page and click two points to draw a wall; the framed elevation appears.

> The wasm artifact (`src/wasm/pkg/`) is a build output and is git-ignored; `src/wasm/engine.d.ts`
> is the hand-kept ambient contract that lets the app typecheck before the artifact exists. The
> per-recompute snapshot is a copy; the zero-copy `SharedArrayBuffer` path is gated on cross-origin
> isolation and is a Phase 5 concern.
