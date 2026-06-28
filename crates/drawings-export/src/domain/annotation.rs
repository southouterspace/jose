//! [`Dimension`] + [`TitleBlock`] — sheet annotations.
//!
//! A dimension anchors two canonical tick points and carries the derived real-inch value the sheet
//! prints; the tick→inch conversion happens once, here. A title block is the per-sheet metadata
//! per National CAD Standard convention.

use crate::keys::ProjectRef;
use geometry_kernel::TickVec3;
use std::collections::BTreeMap;

/// An annotated dimension: two tick anchor points + the derived real-inch value printed on the
/// sheet. `value` is `|b − a| / 32` (ticks → inches), computed once.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Dimension {
    /// First anchor (canonical ticks).
    pub a: TickVec3,
    /// Second anchor (canonical ticks).
    pub b: TickVec3,
    /// Derived real inches = |b − a| / 32.
    pub value: f64,
}

impl Dimension {
    /// A dimension between two world points; the printed value is their 3D distance in inches,
    /// derived from ticks exactly once.
    pub fn between(a: TickVec3, b: TickVec3) -> Dimension {
        let dx = (b.x - a.x).to_inches();
        let dy = (b.y - a.y).to_inches();
        let dz = (b.z - a.z).to_inches();
        Dimension {
            a,
            b,
            value: (dx * dx + dy * dy + dz * dz).sqrt(),
        }
    }
}

/// Sheet metadata block per National CAD Standard convention.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TitleBlock {
    /// Sheet number, e.g. `A-101`.
    pub sheet_no: String,
    /// → the project this sheet belongs to.
    pub project: ProjectRef,
    /// Free-form fields: scale, date, revision, stamp, …
    pub fields: BTreeMap<String, String>,
}

impl TitleBlock {
    /// A title block for `sheet_no` on `project`, no extra fields yet.
    pub fn new(sheet_no: impl Into<String>, project: ProjectRef) -> TitleBlock {
        TitleBlock {
            sheet_no: sheet_no.into(),
            project,
            fields: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geometry_kernel::{TICKS_PER_FOOT, Tick};

    #[test]
    fn dimension_derives_inches_from_ticks_once() {
        // A 10ft horizontal run → 120in.
        let a = TickVec3::new(Tick(0), Tick(0), Tick(0));
        let b = TickVec3::new(Tick(10 * TICKS_PER_FOOT), Tick(0), Tick(0));
        let d = Dimension::between(a, b);
        assert!((d.value - 120.0).abs() < 1e-9);
    }
}
