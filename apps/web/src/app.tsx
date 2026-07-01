/**
 * The app shell: a toolbar, a two-pane split (PLAN / 3D), and a status bar — the rails for the
 * drawing UX (docs/plans/drawing-ux-mvp.md). The toolbar, status bar, and single-key tool shortcuts
 * all read from the **tool-chrome registry** (`tool-chrome.ts`, ADR 0012): a tool declares its label,
 * shortcut, enablement, and status copy in one place, and this shell renders whatever is registered —
 * no per-tool branches here.
 */

import { useEffect, useRef } from "react";
import { useEngineStore } from "./engine-store";
import { PlanView } from "./plan-view";
import { ThreeView } from "./three-view";
import { type ChromeState, TOOL_CHROME, toolChrome } from "./tool-chrome";

/** Tags whose focus should swallow tool shortcuts (typing a length, not switching tools). */
const TYPING_TAGS = new Set(["INPUT", "TEXTAREA", "SELECT"]);

export function App() {
  const store = useEngineStore();
  const chromeState: ChromeState = {
    hasMass: (store.volume?.count ?? 0) > 0,
    footprintVertices: store.footprint?.count ?? 0,
    pendingPicks: store.pendingPicks.length,
    heightFeet:
      store.volume && store.volume.count >= 1
        ? store.volume.row(0).height / 384
        : null,
  };

  // Keep the latest chrome state in a ref so the (once-mounted) key listener reads it without
  // re-subscribing every render. `activate` is stable (useCallback in the store).
  const stateRef = useRef(chromeState);
  stateRef.current = chromeState;
  const { activate } = store;
  useEffect(() => {
    const onKey = (event: KeyboardEvent): void => {
      if (event.metaKey || event.ctrlKey || event.altKey) {
        return; // Leave browser/OS chords alone.
      }
      const el = event.target as HTMLElement | null;
      if (el && (TYPING_TAGS.has(el.tagName) || el.isContentEditable)) {
        return; // Don't hijack keystrokes while typing a value.
      }
      const tool = TOOL_CHROME.find(
        (candidate) => candidate.shortcut === event.key.toLowerCase()
      );
      if (tool?.enabled(stateRef.current)) {
        activate(tool.key);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [activate]);

  return (
    <div className="app">
      <header className="toolbar">
        <span className="toolbar__title">jose — parametric framing</span>
        <nav aria-label="Drawing tools" className="toolbar__tools">
          {TOOL_CHROME.map((tool) => (
            <button
              aria-keyshortcuts={tool.shortcut ?? undefined}
              aria-pressed={store.activeTool === tool.key}
              className="toolbar__tool"
              disabled={!tool.enabled(chromeState)}
              key={tool.key}
              onClick={() => activate(tool.key)}
              title={
                tool.shortcut
                  ? `${tool.label} (${tool.shortcut.toUpperCase()})`
                  : tool.label
              }
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
          <ThreeView store={store} />
        </section>
      </main>

      <footer aria-live="polite" className="statusbar">
        {store.ready
          ? (toolChrome(store.activeTool)?.status(chromeState) ?? "Ready")
          : "Loading engine…"}
      </footer>
    </div>
  );
}
