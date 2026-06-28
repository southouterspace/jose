/**
 * The app shell: a toolbar, a two-pane split (PLAN / 3D), and a status bar — the rails for the
 * drawing UX (docs/plans/drawing-ux-mvp.md). This phase wires the **plan draw loop**: the toolbar's
 * Footprint button activates the footprint tool, the plan view captures the draw and (on ring close)
 * sends a `DrawFootprint` command into the engine worker, and the returned snapshot renders through a
 * read-only `FootprintMirror`. The 3D pane stays an empty placeholder (next phase, ADR 0008).
 */

import { useEngineStore } from "./engine-store";
import { PlanView } from "./plan-view";

const TOOLS = [
  { key: "footprint", label: "Footprint" },
  { key: "pushpull", label: "Push/Pull" },
] as const;

export function App() {
  const store = useEngineStore();

  return (
    <div className="app">
      <header className="toolbar">
        <span className="toolbar__title">jose — parametric framing</span>
        <nav aria-label="Drawing tools" className="toolbar__tools">
          {TOOLS.map((tool) => (
            <button
              aria-pressed={store.activeTool === tool.key}
              className="toolbar__tool"
              // Push/Pull is the next phase — only the footprint tool is wired this slice.
              disabled={tool.key !== "footprint"}
              key={tool.key}
              onClick={() => store.activate(tool.key)}
              type="button"
            >
              {tool.label}
            </button>
          ))}
        </nav>
      </header>

      <main className="viewports">
        <section aria-label="Plan viewport" className="viewport">
          <PlanView store={store} />
        </section>
        <section aria-label="3D viewport" className="viewport">
          <span className="viewport__label">3D</span>
        </section>
      </main>

      <footer className="statusbar">
        {store.ready ? statusText(store) : "Loading engine…"}
      </footer>
    </div>
  );
}

function statusText(store: ReturnType<typeof useEngineStore>): string {
  if (store.footprint && store.footprint.count >= 3) {
    return `Footprint: ${store.footprint.count} vertices`;
  }
  if (store.pendingPicks.length > 0) {
    return `Drawing footprint — ${store.pendingPicks.length} point(s); click the first to close`;
  }
  return "Ready — Footprint tool active; click to place vertices";
}
