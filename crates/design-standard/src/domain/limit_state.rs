//! [`LimitStateCheck`] — the uniform demand-vs-capacity record every failure-mode check produces,
//! and [`LimitStateId`], the open vocabulary of modes.
//!
//! The `origin` tag is what lets a leaf **add** modes without the core enumerating them.

/// An open identifier for a failure mode (`bending` | `shear` | `deflection` | `columnBuckling` |
/// `webCrippling` | …). Core supplies bending/shear/deflection; leaves extend it.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct LimitStateId(pub String);

impl LimitStateId {
    /// Borrow the mode id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Core bending mode.
    pub fn bending() -> LimitStateId {
        LimitStateId("bending".to_owned())
    }
    /// Core shear mode.
    pub fn shear() -> LimitStateId {
        LimitStateId("shear".to_owned())
    }
    /// Core deflection (serviceability) mode.
    pub fn deflection() -> LimitStateId {
        LimitStateId("deflection".to_owned())
    }
    /// Wood column-buckling (CP) mode.
    pub fn column_buckling() -> LimitStateId {
        LimitStateId("columnBuckling".to_owned())
    }
    /// Wood bearing (Fc⊥ crush) mode.
    pub fn bearing() -> LimitStateId {
        LimitStateId("bearing".to_owned())
    }
}

impl From<&str> for LimitStateId {
    fn from(s: &str) -> Self {
        LimitStateId(s.to_owned())
    }
}

/// Whether a check is a shared core mode or a strategy-supplied material mode.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CheckOrigin {
    /// Shared: bending/shear/deflection, capacity injected by the strategy.
    Core,
    /// Material-specific mode supplied wholly by the leaf.
    Strategy,
}

/// A uniform demand-vs-capacity record. `ratio ≤ 1.0` passes; the governing utilization is
/// `max(ratio)` across the set. Real engineering units (lb·in for bending, lb for shear/axial,
/// in for deflection).
#[derive(Clone, PartialEq, Debug)]
pub struct LimitStateCheck {
    /// The failure mode.
    pub id: LimitStateId,
    /// Applied effect for this mode.
    pub demand: f64,
    /// Allowable/factored resistance for this mode (per-mode Ω/φ already folded in).
    pub capacity: f64,
    /// `demand / capacity`, unitless.
    pub ratio: f64,
    /// Core vs strategy.
    pub origin: CheckOrigin,
    /// Derived: `ratio ≤ 1.0`.
    pub pass: bool,
}

impl LimitStateCheck {
    /// Build a check from a demand and capacity, deriving `ratio` and `pass`. A non-positive
    /// capacity is treated as infinite utilization (a failing/over-stressed section).
    pub fn new(
        id: LimitStateId,
        demand: f64,
        capacity: f64,
        origin: CheckOrigin,
    ) -> LimitStateCheck {
        let ratio = if capacity > 0.0 {
            demand.abs() / capacity
        } else if demand.abs() == 0.0 {
            0.0
        } else {
            f64::INFINITY
        };
        LimitStateCheck {
            id,
            demand,
            capacity,
            ratio,
            origin,
            pass: ratio <= 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passing_and_failing_checks() {
        let ok = LimitStateCheck::new(LimitStateId::bending(), 800.0, 1000.0, CheckOrigin::Core);
        assert!(ok.pass);
        assert!((ok.ratio - 0.8).abs() < 1e-9);

        let over = LimitStateCheck::new(LimitStateId::shear(), 1200.0, 1000.0, CheckOrigin::Core);
        assert!(!over.pass);

        let degenerate =
            LimitStateCheck::new(LimitStateId::bending(), 1.0, 0.0, CheckOrigin::Strategy);
        assert!(!degenerate.pass);
        assert_eq!(degenerate.ratio, f64::INFINITY);
    }
}
