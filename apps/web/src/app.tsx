/**
 * The app shell: a toolbar, a two-pane split (PLAN / 3D), and a status bar. These are the rails for
 * the drawing UX (docs/plans/drawing-ux-mvp.md). The viewports are empty placeholders for now — the
 * imperative plan/3D surfaces mount into them in later phases (ADR 0005). The tool buttons are
 * non-functional placeholders; no geometry, commands, or rendering are wired yet.
 */

const TOOLS = ["Footprint", "Push/Pull"] as const;

export function App() {
  return (
    <div className="app">
      <header className="toolbar">
        <span className="toolbar__title">jose — parametric framing</span>
        <nav aria-label="Drawing tools" className="toolbar__tools">
          {TOOLS.map((tool) => (
            <button className="toolbar__tool" disabled key={tool} type="button">
              {tool}
            </button>
          ))}
        </nav>
      </header>

      <main className="viewports">
        <section aria-label="Plan viewport" className="viewport">
          <span className="viewport__label">PLAN</span>
        </section>
        <section aria-label="3D viewport" className="viewport">
          <span className="viewport__label">3D</span>
        </section>
      </main>

      <footer className="statusbar">Ready</footer>
    </div>
  );
}
