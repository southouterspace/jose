//! [`Projection`] + [`HiddenLineRemoval`] — the Rust-side geometry stage of drawings export.
//!
//! Projection and hidden-line removal are heavy over a BREP solid, so they live next to the kernel
//! (architecture review #3): the worker hands clean 2D linework to the JS sheet-composition stage
//! (line weights, dimensions, title block, SVG/PDF), which never re-derives geometry.

use crate::domain::view::{DrawingView, ViewType};
use geometry_kernel::{Path2D, TickVec2, Volume};
use std::collections::BTreeSet;

/// Projects 3D BREP volumes to a 2D view along a camera direction. A pure verb.
#[derive(Clone, Copy, Debug, Default)]
pub struct Projection;

impl Projection {
    /// A fresh projector.
    pub fn new() -> Projection {
        Projection
    }

    /// Project every face boundary of `source` orthographically along `view`, emitting one closed
    /// plane-local polyline per face. Depth is discarded here; occlusion is
    /// [`HiddenLineRemoval`]'s job.
    pub fn project(&self, view: ViewType, source: &[Volume]) -> DrawingView {
        let mut edges = Vec::new();
        for volume in source {
            for face in &volume.faces {
                let projected: Vec<TickVec2> =
                    face.world_points().map(|p| view.project_point(p)).collect();
                if projected.len() >= 3 {
                    edges.push(Path2D::closed(projected));
                }
            }
        }
        DrawingView::new(view, edges)
    }
}

/// Removes occluded / coincident edges from a projected view. Over a solid this is heavy; the
/// scaffold implements the coincident-line case exactly: faces that project onto the same line
/// (e.g. a prism's front and back caps in elevation) double their silhouette segments, so the pass
/// collapses each undirected segment to a single occurrence.
#[derive(Clone, Copy, Debug, Default)]
pub struct HiddenLineRemoval;

impl HiddenLineRemoval {
    /// A fresh hidden-line remover.
    pub fn new() -> HiddenLineRemoval {
        HiddenLineRemoval
    }

    /// Collapse coincident segments: explode each polyline into its edges, normalize each to an
    /// undirected segment, and emit each distinct segment once as a two-point polyline. The result
    /// is the clean, de-duplicated linework the sheet stage draws.
    pub fn remove(&self, view: &DrawingView) -> DrawingView {
        let mut seen: BTreeSet<((i32, i32), (i32, i32))> = BTreeSet::new();
        let mut edges = Vec::new();
        for path in &view.edges {
            for (a, b) in segments(path) {
                let key = undirected_key(a, b);
                if seen.insert(key) {
                    edges.push(Path2D::open(vec![a, b]));
                }
            }
        }
        DrawingView {
            view: view.view,
            edges,
            scale: view.scale,
        }
    }
}

/// The undirected (orientation-independent) key for a segment, so `a→b` and `b→a` coincide.
fn undirected_key(a: TickVec2, b: TickVec2) -> ((i32, i32), (i32, i32)) {
    let pa = (a.u.raw(), a.v.raw());
    let pb = (b.u.raw(), b.v.raw());
    if pa <= pb { (pa, pb) } else { (pb, pa) }
}

/// The boundary segments of a polyline, closing the loop when the path is closed.
fn segments(path: &Path2D) -> Vec<(TickVec2, TickVec2)> {
    let v = path.vertices();
    if v.len() < 2 {
        return Vec::new();
    }
    let mut out: Vec<(TickVec2, TickVec2)> = v.windows(2).map(|w| (w[0], w[1])).collect();
    if path.is_closed()
        && let (Some(&first), Some(&last)) = (v.first(), v.last())
    {
        out.push((last, first));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use geometry_kernel::{EntityId, GeometryKernel, Plane, Tick, TickVec2, TickVec3, UnitVec3};

    /// A 1ft × 1ft × 8ft prism, extruded up +Z from the XY plane.
    fn box_volume() -> Volume {
        let ft = geometry_kernel::TICKS_PER_FOOT;
        let square = Path2D::closed(vec![
            TickVec2::new(Tick(0), Tick(0)),
            TickVec2::new(Tick(ft), Tick(0)),
            TickVec2::new(Tick(ft), Tick(ft)),
            TickVec2::new(Tick(0), Tick(ft)),
        ]);
        GeometryKernel::new()
            .extrude(
                EntityId(1),
                square,
                Plane::xy(TickVec3::ZERO),
                UnitVec3::Z,
                Tick(8 * ft),
            )
            .expect("a unit square extrudes")
    }

    #[test]
    fn projection_emits_one_polyline_per_face() {
        let vol = box_volume();
        let view = Projection::new().project(ViewType::Plan, std::slice::from_ref(&vol));
        // The prism has a base + top cap face; both project to the same square in plan.
        assert_eq!(view.edge_count(), vol.faces.len());
        assert_eq!(view.view, Some(ViewType::Plan));
    }

    #[test]
    fn hidden_line_removal_collapses_coincident_segments() {
        let vol = box_volume();
        let projected = Projection::new().project(ViewType::Plan, std::slice::from_ref(&vol));
        // In plan, base and top caps coincide → every silhouette segment appears twice.
        let raw_segments: usize = projected.edges.iter().map(|p| p.vertex_count()).sum();
        let cleaned = HiddenLineRemoval::new().remove(&projected);
        assert!(
            cleaned.edge_count() < raw_segments,
            "coincident segments collapse: {} < {}",
            cleaned.edge_count(),
            raw_segments
        );
        // The square's four distinct edges survive exactly once.
        assert_eq!(cleaned.edge_count(), 4);
    }
}
