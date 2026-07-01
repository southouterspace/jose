/**
 * Rejection copy — the human-facing twin of the engine's `RejectReason` codes (`command.rs`). The
 * engine refuses a degenerate footprint or an out-of-model push/pull with a stable machine code; the
 * UI surfaces *why* nothing happened instead of the command silently doing nothing (the
 * rejected-command coverage gap). Kept pure/React-free so it is unit-testable without a scene, and
 * kept in lock-step with `RejectReason::code` on the Rust side.
 */

/** Map a `RejectReason` code to a short, user-facing sentence. Unknown codes fall back to a generic
 *  line rather than leaking the raw code. */
export function rejectionMessage(reason: string): string {
  switch (reason) {
    case "too_few_vertices":
      return "A footprint needs at least 3 corners.";
    case "zero_area":
      return "That footprint encloses no area — its corners are in a line.";
    case "self_intersecting":
      return "That footprint crosses itself. Draw a simple outline that doesn't overlap.";
    case "not_top_face":
      return "Push/Pull works on the top face only.";
    case "non_positive_height":
      return "A mass can't be pushed below the ground.";
    case "no_target":
      return "Draw a footprint before pushing it into a mass.";
    default:
      return "That action can't be applied here.";
  }
}
