//! [`TributaryArea`] — the plan/elevation area a single member carries, converting a psf source
//! pressure into a per-member line or point load.
//!
//! THE fix for the tick²-area bug: `area` is real in² (derived); the linear spans/widths that
//! derive it stay integer ticks.

use building::MemberPlacementId;
use geometry_kernel::{Tick, TickVec3};

/// How the tributary area integrates over the member's catchment.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TributaryShape {
    /// Joist/stud strip (half-spacing each side).
    Strip,
    /// Girder rectangle.
    Rectangular,
    /// One-way triangular distribution to a ridge.
    Triangular,
    /// General polygon.
    Polygon,
}

/// The area a member carries, derived from spacing and span geometry. Pure geometry → area;
/// `psf × area = force` happens in the rollup, keeping this a clean value object.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct TributaryArea {
    /// The member this area is attributed to.
    pub member_ref: MemberPlacementId,
    /// Member span (clear/effective length) in ticks — linear, stays ticks.
    pub span: Tick,
    /// Half-spacing to each adjacent member in ticks — linear, stays ticks.
    pub tributary_width: Tick,
    /// Derived real area in in² (`(span/32)·(tributaryWidth/32)`) — never tick².
    pub area: f64,
    /// Integration rule.
    pub shape: Option<TributaryShape>,
    /// Application point of the resultant in world ticks (a committed position).
    pub load_centroid: Option<TickVec3>,
}

impl TributaryArea {
    /// A rectangular/strip tributary from a member span and tributary width, deriving the real
    /// in² area from the two tick lengths.
    pub fn strip(
        member_ref: MemberPlacementId,
        span: Tick,
        tributary_width: Tick,
    ) -> TributaryArea {
        TributaryArea {
            member_ref,
            span,
            tributary_width,
            area: span.to_inches() * tributary_width.to_inches(),
            shape: Some(TributaryShape::Strip),
            load_centroid: None,
        }
    }

    /// The tributary area expressed in ft² (the unit the live-load reduction and psf math use).
    pub fn area_ft2(&self) -> f64 {
        self.span.to_feet() * self.tributary_width.to_feet()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn area_is_real_never_tick_squared() {
        // 12ft span (4608t) @ 16in trib width (512t): 144in × 16in = 2304 in² = 16 ft².
        let t = TributaryArea::strip(MemberPlacementId(1), Tick(4608), Tick(512));
        assert!((t.area - 144.0 * 16.0).abs() < 1e-9);
        assert!((t.area_ft2() - 16.0).abs() < 1e-9);
    }
}
