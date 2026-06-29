// FIXTURE (before) — intentionally flawed UI for the product-design eval. Do not "fix" in place;
// the flaws are the test. Violations: raw ticks shown to the user (rule/display-feet-not-ticks),
// a non-canonical noun "Extrude" (rule/canonical-noun), and a mutating control label
// (rule/stable-control-label).

interface Store {
  readonly activeTool: "footprint" | "pushpull";
  readonly volume: { readonly height: number } | null; // height in ticks (1ft = 384 ticks)
  readonly drawing: boolean;
}

export function statusText(store: Store): string {
  if (store.activeTool === "pushpull") {
    return "Extrude active — drag the top of the box to set its size";
  }
  if (store.volume) {
    return `Outline placed · box ${store.volume.height} ticks tall`;
  }
  return "Ready";
}

// The tool button relabels itself to show progress.
export function footprintToolLabel(store: Store): string {
  return store.drawing ? "Drawing…" : "Footprint";
}
