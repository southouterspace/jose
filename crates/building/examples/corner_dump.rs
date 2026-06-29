//! Plan-view visualizer for the corner framer — **real engine output**, not a re-drawing.
//!
//! Drives the actual `detect_junctions` → `detail_junction` path (the same calls `frame_walls`
//! makes) over three scenarios — the golden L corner, a rectangle, and an inside (reentrant)
//! elbow — and emits a self-contained HTML plan view to stdout. Every corner-post rectangle it
//! draws is literally the `(min, size)` the detailer returned in world ticks, so eyeballing it
//! against `scratchpad/corner-framing.html` confirms the code matches the hand-drawn geometry.
//!
//! ```bash
//! cargo run -p building --example corner_dump > scratchpad/corner-framing-engine.html
//! ```

use building::{
    CornerSense, FaceRef, Junction, SpacingModule, Wall, WallId, WallRole, detail_junction,
    detect_junctions,
};
use geometry_kernel::{EntityId, Segment, Tick, TickVec3};

const FT: i32 = 384; // 1ft = 384 ticks (1/32in)

fn wall(id: u128, ax: i32, ay: i32, bx: i32, by: i32) -> Wall {
    let baseline = Segment::new(
        TickVec3::new(Tick(ax), Tick(ay), Tick(0)),
        TickVec3::new(Tick(bx), Tick(by), Tick(0)),
    );
    Wall::promote(
        WallId(id),
        FaceRef {
            volume: EntityId(1),
            face_index: 0,
        },
        baseline,
        Tick(96 * 32), // 8ft tall
        Tick(112),     // 3.5in thick (2x4)
        WallRole::Bearing,
        SpacingModule::inches(16),
    )
}

/// The shared plan endpoint of two walls (the corner vertex), in world ticks.
fn shared_vertex(a: &Wall, b: &Wall) -> (i32, i32) {
    let ea = [
        (a.baseline.a.x.raw(), a.baseline.a.y.raw()),
        (a.baseline.b.x.raw(), a.baseline.b.y.raw()),
    ];
    let eb = [
        (b.baseline.a.x.raw(), b.baseline.a.y.raw()),
        (b.baseline.b.x.raw(), b.baseline.b.y.raw()),
    ];
    for p in ea {
        if eb.contains(&p) {
            return p;
        }
    }
    ea[1]
}

fn sense_str(s: Option<CornerSense>) -> &'static str {
    match s {
        Some(CornerSense::Outside) => "Outside",
        Some(CornerSense::Inside) => "Inside",
        None => "—",
    }
}

/// Emit one scenario's walls + classified junctions + real detailer posts as a JSON object.
fn scenario_json(name: &str, walls: &[Wall]) -> String {
    let junctions = detect_junctions(walls);

    let walls_json = walls
        .iter()
        .map(|w| {
            format!(
                "{{\"id\":{},\"ax\":{},\"ay\":{},\"bx\":{},\"by\":{},\"thickness\":{},\"interiorLeft\":{}}}",
                w.id.0,
                w.baseline.a.x.raw(),
                w.baseline.a.y.raw(),
                w.baseline.b.x.raw(),
                w.baseline.b.y.raw(),
                w.thickness.raw(),
                w.interior_on_left
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    let junctions_json = junctions
        .iter()
        .map(|j| junction_json(j, walls))
        .collect::<Vec<_>>()
        .join(",");

    format!("{{\"name\":\"{name}\",\"walls\":[{walls_json}],\"junctions\":[{junctions_json}]}}")
}

fn junction_json(j: &Junction, walls: &[Wall]) -> String {
    // Participating walls, owner first, so the real detailer runs exactly as the solver calls it.
    let owner = walls.iter().find(|w| w.id == j.owner_wall);
    let other = walls
        .iter()
        .find(|w| j.walls.contains(&w.id) && w.id != j.owner_wall);
    let (Some(owner), Some(other)) = (owner, other) else {
        return String::from("{}");
    };

    let detail = detail_junction(j, &[owner, other]);
    let (vx, vy) = shared_vertex(owner, other);

    let posts_json = detail
        .posts
        .iter()
        .enumerate()
        .map(|(i, p)| {
            format!(
                "{{\"label\":\"S{}\",\"minx\":{},\"miny\":{},\"dx\":{},\"dy\":{}}}",
                i + 1,
                p.min.u.raw(),
                p.min.v.raw(),
                p.size.u.raw(),
                p.size.v.raw()
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    let laps_json = detail
        .laps
        .iter()
        .map(|l| {
            format!(
                "{{\"course\":{},\"runsThrough\":{}}}",
                l.course, l.runs_through
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    format!(
        "{{\"type\":\"{:?}\",\"sense\":\"{}\",\"method\":\"{:?}\",\"owner\":{},\"vx\":{vx},\"vy\":{vy},\"posts\":[{posts_json}],\"laps\":[{laps_json}]}}",
        j.junction_type,
        sense_str(j.sense),
        j.method,
        j.owner_wall.0
    )
}

fn main() {
    // An outside L corner matching corner-framing.html: both walls wound so their interior
    // (left of travel) lands NE, making the shared origin the building's convex outside corner.
    let l_corner = vec![
        wall(1, 0, 0, 10 * FT, 0), // runs east from the corner; interior +y
        wall(2, 0, 10 * FT, 0, 0), // runs south into the corner; interior +x
    ];

    let rectangle = vec![
        wall(1, 0, 0, 10 * FT, 0),
        wall(2, 10 * FT, 0, 10 * FT, 10 * FT),
        wall(3, 10 * FT, 10 * FT, 0, 10 * FT),
        wall(4, 0, 10 * FT, 0, 0),
    ];

    let inside_elbow = vec![
        wall(1, 10 * FT, 6 * FT, 6 * FT, 6 * FT), // reentrant corner of an L-room
        wall(2, 6 * FT, 6 * FT, 6 * FT, 10 * FT),
    ];

    let scenarios = [
        scenario_json("L corner (outside → California)", &l_corner),
        scenario_json("Rectangle — 4 outside corners (owner-only)", &rectangle),
        scenario_json("Inside elbow (inside → 3-stud)", &inside_elbow),
    ]
    .join(",");

    let data = format!("[{scenarios}]");
    print!("{}", html(&data));
}

fn html(data: &str) -> String {
    format!(
        r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>Corner framer — engine output (plan view)</title>
<style>
  :root {{ color-scheme: dark; font-family: system-ui, sans-serif; }}
  body {{ margin: 0; background: #0e1014; color: #e8e8ea; padding: 16px 20px 48px; }}
  h1 {{ font-size: 1.05rem; margin: 0 0 2px; }}
  p.sub {{ margin: 0 0 18px; color: #9aa3b2; font-size: .82rem; max-width: 70rem; }}
  .scn {{ margin: 0 0 30px; }}
  .scn h2 {{ font-size: .95rem; margin: 0 0 6px; }}
  .row {{ display: flex; gap: 24px; flex-wrap: wrap; align-items: flex-start; }}
  svg {{ background: #0b0d12; border-radius: 6px; display: block; border: 1px solid #1c2230; }}
  .meta {{ font-size: .76rem; color: #c8cad2; }}
  .meta table {{ border-collapse: collapse; margin: 4px 0 12px; }}
  .meta th, .meta td {{ border: 1px solid #2a3140; padding: 2px 8px; text-align: left; }}
  .meta th {{ color: #aeb6c4; }}
  .tag {{ display:inline-block; padding:1px 7px; border-radius:10px; font-size:.72rem; font-weight:600; }}
  .out {{ background:#5a3a2a; color:#f0c089; }}
  .in {{ background:#2a3a5a; color:#89b0f0; }}
  .legend {{ display: flex; gap: 16px; flex-wrap: wrap; font-size: .74rem; color: #c8cad2; margin: 0 0 14px; }}
  .legend span {{ display: inline-flex; align-items: center; gap: 6px; }}
  .sw {{ width: 14px; height: 10px; border-radius: 2px; display: inline-block; border: 1px solid #0008; }}
</style>
</head>
<body>
  <h1>Corner framer — engine output, plan view (looking down)</h1>
  <p class="sub">
    Every corner-post rectangle below is the <b>real</b> output of <code>detect_junctions</code> →
    <code>detail_junction</code> (the same path <code>frame_walls</code> runs), rendered in world
    ticks (1/32&quot;; 384 = 1ft). Owner-only posts; in/out is derived from the drawing; the method
    comes from the per-class defaults (outside&nbsp;→&nbsp;California, inside&nbsp;→&nbsp;3-stud).
    Compare to <code>corner-framing.html</code>: the footprints match to the tick.
  </p>
  <div class="legend">
    <span><i class="sw" style="background:#c98b5a"></i> corner post (owner-framed)</span>
    <span><i class="sw" style="background:#3a4a6b"></i> wall body</span>
    <span><i class="sw" style="background:#8fd1ff"></i> baseline (exterior face)</span>
    <span><i class="sw" style="background:#fff"></i> plotted stud corner</span>
  </div>
  <div id="root"></div>
<script>
const DATA = {data};
const svgNS = 'http://www.w3.org/2000/svg';
const el = (t, a) => {{ const e = document.createElementNS(svgNS, t); for (const k in a) e.setAttribute(k, a[k]); return e; }};

function bounds(scn) {{
  let minx = Infinity, miny = Infinity, maxx = -Infinity, maxy = -Infinity;
  const eat = (x, y) => {{ minx = Math.min(minx, x); miny = Math.min(miny, y); maxx = Math.max(maxx, x); maxy = Math.max(maxy, y); }};
  for (const w of scn.walls) {{ eat(w.ax, w.ay); eat(w.bx, w.by); }}
  for (const j of scn.junctions) for (const p of j.posts) {{ eat(p.minx, p.miny); eat(p.minx + p.dx, p.miny + p.dy); }}
  return {{ minx, miny, maxx, maxy }};
}}

function render(scn, host) {{
  const b = bounds(scn);
  const pad = 96; // ticks of margin around content
  const MINX = b.minx - pad, MAXX = b.maxx + pad, MINY = b.miny - pad, MAXY = b.maxy + pad;
  const PX = 360 / Math.max(MAXX - MINX, MAXY - MINY); // px per tick, target ~360px
  const W = (MAXX - MINX) * PX + 8, H = (MAXY - MINY) * PX + 8;
  const px = x => 4 + (x - MINX) * PX;
  const py = y => 4 + (MAXY - y) * PX; // flip Y (north up)
  const svg = el('svg', {{ width: W, height: H, viewBox: `0 0 ${{W}} ${{H}}` }});

  // grid every 1ft
  for (let x = Math.ceil(MINX / FT2()) * FT2(); x <= MAXX; x += FT2())
    svg.appendChild(el('line', {{ x1: px(x), y1: py(MINY), x2: px(x), y2: py(MAXY), stroke: '#171c27', 'stroke-width': 1 }}));
  for (let y = Math.ceil(MINY / FT2()) * FT2(); y <= MAXY; y += FT2())
    svg.appendChild(el('line', {{ x1: px(MINX), y1: py(y), x2: px(MAXX), y2: py(y), stroke: '#171c27', 'stroke-width': 1 }}));

  // wall bodies (baseline offset to the interior side by thickness)
  for (const w of scn.walls) {{
    const dx = w.bx - w.ax, dy = w.by - w.ay, len = Math.hypot(dx, dy) || 1;
    const ux = dx / len, uy = dy / len;
    // interior normal: left = (-uy, ux); right = (uy, -ux)
    const nx = (w.interiorLeft ? -uy : uy) * w.thickness;
    const ny = (w.interiorLeft ? ux : -ux) * w.thickness;
    const pts = [[w.ax, w.ay], [w.bx, w.by], [w.bx + nx, w.by + ny], [w.ax + nx, w.ay + ny]];
    svg.appendChild(el('polygon', {{ points: pts.map(([x, y]) => `${{px(x)}},${{py(y)}}`).join(' '), fill: '#3a4a6b33', stroke: '#46506b', 'stroke-width': 1 }}));
    // baseline = exterior face
    svg.appendChild(el('line', {{ x1: px(w.ax), y1: py(w.ay), x2: px(w.bx), y2: py(w.by), stroke: '#8fd1ff', 'stroke-width': 2 }}));
  }}

  // corner posts — the real detailer footprints
  for (const j of scn.junctions) {{
    for (const p of j.posts) {{
      const x0 = p.minx, y0 = p.miny, x1 = p.minx + p.dx, y1 = p.miny + p.dy;
      svg.appendChild(el('rect', {{ x: px(x0), y: py(y1), width: p.dx * PX, height: p.dy * PX, fill: '#c98b5a', stroke: '#0a0c10', 'stroke-width': 1, opacity: .92 }}));
      for (const [vx, vy] of [[x0, y0], [x1, y0], [x1, y1], [x0, y1]])
        svg.appendChild(el('circle', {{ cx: px(vx), cy: py(vy), r: 2.4, fill: '#fff', stroke: '#0a0c10', 'stroke-width': .7 }}));
      const t = el('text', {{ x: px((x0 + x1) / 2), y: py((y0 + y1) / 2) + 3, fill: '#1a1205', 'font-size': 9, 'font-weight': 700, 'text-anchor': 'middle' }});
      t.textContent = p.label; svg.appendChild(t);
    }}
    // mark the corner vertex
    svg.appendChild(el('circle', {{ cx: px(j.vx), cy: py(j.vy), r: 3.2, fill: 'none', stroke: '#e06666', 'stroke-width': 1.4 }}));
  }}

  host.appendChild(svg);
}}

function FT2() {{ return 384; }}

function metaTable(scn) {{
  const wrap = document.createElement('div'); wrap.className = 'meta';
  const rows = scn.junctions.map(j => {{
    const tag = j.sense === 'Outside' ? '<span class="tag out">outside</span>'
      : j.sense === 'Inside' ? '<span class="tag in">inside</span>' : j.sense;
    const laps = j.laps.map(l => `c${{l.course}}:${{l.runsThrough ? 'through' : 'butt'}}`).join(', ');
    const posts = j.posts.map(p => `${{p.label}} (${{p.minx}},${{p.miny}})+${{p.dx}}×${{p.dy}}`).join('<br>');
    return `<tr><td>${{j.type}}</td><td>${{tag}}</td><td>${{j.method}}</td><td>wall ${{j.owner}}</td><td>${{posts || '—'}}</td><td>${{laps || '—'}}</td></tr>`;
  }}).join('');
  wrap.innerHTML = `<table><tr><th>type</th><th>sense</th><th>method</th><th>owner</th><th>posts (world ticks)</th><th>top-plate lap</th></tr>${{rows}}</table>`;
  return wrap;
}}

const root = document.getElementById('root');
for (const scn of DATA) {{
  const block = document.createElement('div'); block.className = 'scn';
  block.innerHTML = `<h2>${{scn.name}}</h2>`;
  const row = document.createElement('div'); row.className = 'row';
  const left = document.createElement('div');
  render(scn, left);
  row.appendChild(left);
  row.appendChild(metaTable(scn));
  block.appendChild(row);
  root.appendChild(block);
}}
</script>
</body>
</html>
"##
    )
}
