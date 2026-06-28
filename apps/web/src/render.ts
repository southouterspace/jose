/**
 * The render mirror's pixels: draw a [`MemberMirror`]'s framing as a wall elevation on a 2D canvas.
 *
 * Reads only — it walks the zero-copy column views and strokes one line per member (studs vertical,
 * plates/headers horizontal), coloring by role. The elevation is fit to the canvas with a margin.
 */
import type { MemberMirror } from "@jose/render-mirror";

const ROLE_COLORS: Record<string, string> = {
  plate: "#b5651d",
  stud: "#deb887",
  king: "#8b5a2b",
  jack: "#a0522d",
  cripple: "#cd853f",
  header: "#5b3a1a",
  sill: "#c19a6b",
  post: "#6b4423",
};

export function renderMembers(ctx: CanvasRenderingContext2D, mirror: MemberMirror): void {
  const { canvas } = ctx;
  ctx.clearRect(0, 0, canvas.width, canvas.height);

  const rows = mirror.rows();
  if (rows.length === 0) return;

  // Fit the wall-local elevation (x along the baseline, z up) into the canvas.
  let maxX = 1;
  let maxZ = 1;
  for (const r of rows) {
    maxX = Math.max(maxX, r.x0, r.x1);
    maxZ = Math.max(maxZ, r.z0, r.z1);
  }
  const margin = 48;
  const scale = Math.min((canvas.width - 2 * margin) / maxX, (canvas.height - 2 * margin) / maxZ);
  const toX = (x: number): number => margin + x * scale;
  const toY = (z: number): number => canvas.height - margin - z * scale; // z up

  ctx.lineCap = "round";
  for (const r of rows) {
    ctx.strokeStyle = ROLE_COLORS[r.role] ?? "#888888";
    ctx.lineWidth = Math.max(1, r.width * scale);
    ctx.beginPath();
    ctx.moveTo(toX(r.x0), toY(r.z0));
    ctx.lineTo(toX(r.x1), toY(r.z1));
    ctx.stroke();
  }
}
