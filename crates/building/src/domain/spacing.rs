//! [`SpacingModule`] — on-center layout *intent* as a parameter, not stored geometry.
//!
//! 19.2in OC = 8ft/5 = 614.4 ticks is **not** an integer tick, which is exactly why OC spacing
//! must not be typed as ticks. The [`FramingSolver`](crate::FramingSolver) derives actual member
//! positions as integer-tick points, absorbing the sub-tick remainder into the layout.

use geometry_kernel::{TICKS_PER_INCH, Tick};

/// Open key naming a standard OC module (`12` | `16` | `19.2` | `24` | `custom`). Open, so
/// non-standard modules are data not code.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SpacingKey(pub String);

impl From<&str> for SpacingKey {
    fn from(s: &str) -> Self {
        SpacingKey(s.to_owned())
    }
}

/// How an on-center run is anchored before rounding — drives layout stability ("anchor the
/// grid, don't redraw it").
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SpacingAnchor {
    /// Lay out from the wall start.
    WallStart,
    /// Lay out from a project gridline.
    Gridline,
    /// Lay out from a sheet edge (sheathing module alignment).
    SheetEdge,
}

/// On-center layout intent: a real inch module the solver rounds member positions from. Geometry
/// stays integer ticks; the module that *generates* it is real.
#[derive(Clone, PartialEq, Debug)]
pub struct SpacingModule {
    /// Open key, e.g. `16` or `19.2`.
    pub module: SpacingKey,
    /// Precise inch value — the source of truth the solver rounds from (19.2 for fifths-of-8ft).
    pub exact_inches: f64,
    /// How the run is anchored before rounding.
    pub anchor: Option<SpacingAnchor>,
}

impl SpacingModule {
    /// A standard module from a whole-inch OC value (`12`, `16`, `24`).
    pub fn inches(inches: u32) -> SpacingModule {
        SpacingModule {
            module: SpacingKey(inches.to_string()),
            exact_inches: inches as f64,
            anchor: Some(SpacingAnchor::WallStart),
        }
    }

    /// The 19.2in (fifths-of-8ft) module — the canonical case proving OC spacing is not a tick.
    pub fn nineteen_two() -> SpacingModule {
        SpacingModule {
            module: SpacingKey::from("19.2"),
            exact_inches: 19.2,
            anchor: Some(SpacingAnchor::SheetEdge),
        }
    }

    /// The integer-tick step the solver lays members out on — the exact inch module rounded onto
    /// the tick lattice. 19.2in → 614 ticks (the sub-tick remainder is absorbed into the layout).
    pub fn step_ticks(&self) -> Tick {
        Tick((self.exact_inches * TICKS_PER_INCH as f64).round() as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_inch_module_is_exact_ticks() {
        assert_eq!(SpacingModule::inches(16).step_ticks(), Tick(512));
        assert_eq!(SpacingModule::inches(24).step_ticks(), Tick(768));
    }

    #[test]
    fn nineteen_two_rounds_off_lattice() {
        // 19.2in = 614.4 ticks → rounds to 614, never stored as the float.
        let m = SpacingModule::nineteen_two();
        assert_eq!(m.step_ticks(), Tick(614));
        assert!((m.exact_inches - 19.2).abs() < 1e-9);
    }
}
