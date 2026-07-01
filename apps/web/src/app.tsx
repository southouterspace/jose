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

/** Whether focus is in a text field that should keep its own keystrokes. */
function isTypingTarget(el: HTMLElement | null): boolean {
  return !!el && (TYPING_TAGS.has(el.tagName) || el.isContentEditable);
}

/** Resolve an undo/redo keyboard chord (Cmd/Ctrl+Z, Shift for redo; Ctrl+Y on Windows), or `null`
 *  when the event isn't one. */
function historyChord(event: KeyboardEvent): "redo" | "undo" | null {
  if (event.altKey || !(event.metaKey || event.ctrlKey)) {
    return null;
  }
  const key = event.key.toLowerCase();
  if (key === "y") {
    return "redo";
  }
  if (key === "z") {
    return event.shiftKey ? "redo" : "undo";
  }
  return null;
}

/** The tool key a bare single-key press activates given the live state, or `null` — no modifiers, and
 *  the tool must be enabled. Keeps the key handler's branching (and complexity) low. */
function shortcutToolKey(
  event: KeyboardEvent,
  state: ChromeState
): string | null {
  if (event.metaKey || event.ctrlKey || event.altKey) {
    return null;
  }
  const tool = TOOL_CHROME.find(
    (candidate) => candidate.shortcut === event.key.toLowerCase()
  );
  return tool?.enabled(state) ? tool.key : null;
}

/** The single action a keydown triggers. */
type KeyAction =
  | { readonly type: "clear" | "redo" | "undo" }
  | { readonly type: "activate"; readonly key: string };

/** Classify a keydown into the one action it triggers (or `null`). Pure, so the listener stays a thin
 *  dispatch: chords first, then Escape-clears, then a single-key tool shortcut. */
function keyAction(
  event: KeyboardEvent,
  typing: boolean,
  state: ChromeState
): KeyAction | null {
  const chord = historyChord(event);
  if (chord) {
    return typing ? null : { type: chord };
  }
  if (event.key === "Escape") {
    return typing ? null : { type: "clear" };
  }
  if (typing) {
    return null;
  }
  const key = shortcutToolKey(event, state);
  return key ? { type: "activate", key } : null;
}

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
    selectedKind: store.selection?.kind ?? null,
  };

  // Keep the latest chrome state in a ref so the (once-mounted) key listener reads it without
  // re-subscribing every render. `activate`/`undo`/`redo` are stable (useCallback in the store).
  const stateRef = useRef(chromeState);
  stateRef.current = chromeState;
  const { activate, undo, redo, dismissRejection, clearSelection } = store;
  useEffect(() => {
    const onKey = (event: KeyboardEvent): void => {
      const typing = isTypingTarget(event.target as HTMLElement | null);
      const action = keyAction(event, typing, stateRef.current);
      if (!action) {
        return;
      }
      switch (action.type) {
        case "undo":
          event.preventDefault();
          undo();
          break;
        case "redo":
          event.preventDefault();
          redo();
          break;
        case "clear":
          clearSelection();
          break;
        default:
          activate(action.key);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [activate, undo, redo, clearSelection]);

  // Auto-dismiss the rejection toast a few seconds after it appears; re-armed per rejection via the
  // nonce (an identical repeat still resets the timer). Manual dismiss + successful commands also
  // clear it (in the store).
  const rejectionNonce = store.rejection?.nonce;
  useEffect(() => {
    if (rejectionNonce === undefined) {
      return;
    }
    const id = setTimeout(dismissRejection, 5000);
    return () => clearTimeout(id);
  }, [rejectionNonce, dismissRejection]);

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

        <nav aria-label="History" className="toolbar__actions">
          <button
            aria-keyshortcuts="Control+Z Meta+Z"
            className="toolbar__tool"
            disabled={!store.canUndo}
            onClick={undo}
            title="Undo (Ctrl/⌘+Z)"
            type="button"
          >
            Undo
          </button>
          <button
            aria-keyshortcuts="Control+Shift+Z Meta+Shift+Z Control+Y"
            className="toolbar__tool"
            disabled={!store.canRedo}
            onClick={redo}
            title="Redo (Ctrl/⌘+Shift+Z)"
            type="button"
          >
            Redo
          </button>
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

      {store.rejection && (
        <div className="toast" role="alert">
          <span className="toast__message">{store.rejection.message}</span>
          <button
            aria-label="Dismiss"
            className="toast__dismiss"
            onClick={dismissRejection}
            type="button"
          >
            ×
          </button>
        </div>
      )}

      <footer aria-live="polite" className="statusbar">
        {store.ready
          ? (toolChrome(store.activeTool)?.status(chromeState) ?? "Ready")
          : "Loading engine…"}
      </footer>
    </div>
  );
}
