// FIXTURE (after) — one acceptable result, not the only one. Score rule-correctness, not similarity.
// Fixes: feet not ticks (rule/display-feet-not-ticks), canonical noun "Push/Pull" / "footprint" /
// "mass" (rule/canonical-noun), and a stable control label with progress in the status bar
// (rule/stable-control-label).

const TICKS_PER_FOOT = 384;

interface Store {
  readonly activeTool: "footprint" | "pushpull";
  readonly volume: { readonly height: number } | null; // height in ticks
  readonly drawing: boolean;
}

export function statusText(store: Store): string {
  if (store.activeTool === "pushpull") {
    return "Push/Pull active — drag the top cap in 3D to set the mass height";
  }
  if (store.volume) {
    const feet = (store.volume.height / TICKS_PER_FOOT).toFixed(1);
    return `Footprint placed · mass ${feet}ft tall`;
  }
  if (store.drawing) {
    return "Drawing footprint — click the first point to close";
  }
  return "Ready — Footprint tool active; click to place vertices";
}

// The control label is stable; progress is reported through statusText above.
export function footprintToolLabel(): string {
  return "Footprint";
}
