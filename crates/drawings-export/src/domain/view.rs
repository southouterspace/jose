//! [`ViewType`] + [`DrawingView`] — the projected, composed 2D linework ready for sheet placement.

use geometry_kernel::{Path2D, TickVec3};

/// The camera the [`Projection`](crate::Projection) projects along. Each view drops one world axis
/// to the view plane's local `(u, v)`; the schema sources this from the `workspace-render` layer's
/// `ViewType`, named locally here until that context lands.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ViewType {
    /// Top-down: world X→u, Y→v (drop Z).
    Plan,
    /// Front elevation: world X→u, Z→v (drop Y).
    Elevation,
    /// Side elevation: world Y→u, Z→v (drop X).
    Section,
}

impl ViewType {
    /// Project a world point onto this view's plane by selecting the two retained axes. The
    /// projection is orthographic — the dropped axis is simply discarded (depth handled by
    /// [`HiddenLineRemoval`](crate::HiddenLineRemoval), not here).
    pub fn project_point(self, p: TickVec3) -> geometry_kernel::TickVec2 {
        use geometry_kernel::TickVec2;
        match self {
            ViewType::Plan => TickVec2::new(p.x, p.y),
            ViewType::Elevation => TickVec2::new(p.x, p.z),
            ViewType::Section => TickVec2::new(p.y, p.z),
        }
    }
}

/// A composed 2D view: visible linework (plane-local tick polylines) ready for sheet placement.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct DrawingView {
    /// The camera this view was projected along.
    pub view: Option<ViewType>,
    /// Plane-local tick polylines.
    pub edges: Vec<Path2D>,
    /// Model→paper scale numerator/denominator, e.g. 1/48 (1/4" = 1ft). `None` until placed.
    pub scale: Option<(u32, u32)>,
}

impl DrawingView {
    /// A view of the given edges projected along `view`, unplaced (no scale yet).
    pub fn new(view: ViewType, edges: Vec<Path2D>) -> DrawingView {
        DrawingView {
            view: Some(view),
            edges,
            scale: None,
        }
    }

    /// Number of polylines in the view.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geometry_kernel::Tick;

    #[test]
    fn elevation_drops_the_y_axis() {
        let p = TickVec3::new(Tick(100), Tick(200), Tick(300));
        let uv = ViewType::Elevation.project_point(p);
        assert_eq!(uv.u, Tick(100)); // x
        assert_eq!(uv.v, Tick(300)); // z
    }

    #[test]
    fn plan_drops_the_z_axis() {
        let p = TickVec3::new(Tick(100), Tick(200), Tick(300));
        let uv = ViewType::Plan.project_point(p);
        assert_eq!(uv.u, Tick(100));
        assert_eq!(uv.v, Tick(200));
    }
}
