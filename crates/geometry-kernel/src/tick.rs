//! The tick base unit.

use core::fmt;
use core::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

/// Ticks per inch. `1in = 32 ticks`, so a tick is 1/32 inch.
pub const TICKS_PER_INCH: i32 = 32;
/// Ticks per foot. `1ft = 12in = 384 ticks`.
pub const TICKS_PER_FOOT: i32 = TICKS_PER_INCH * 12;

/// A signed integer count of 1/32-inch increments — the base linear unit.
///
/// Every world-space linear magnitude in the system is a `Tick`, which eliminates float
/// drift on imperial fractions (32 = 1in, 112 = 3.5in = a dressed 2×4 face). Signed, so it
/// also represents relative offsets and insets.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Tick(pub i32);

impl Tick {
    /// The zero tick.
    pub const ZERO: Tick = Tick(0);

    /// The underlying signed 32-bit count.
    #[inline]
    pub const fn raw(self) -> i32 {
        self.0
    }

    /// Nearest tick to a measurement in inches (round half away from zero).
    #[inline]
    pub fn from_inches(inches: f64) -> Tick {
        Tick((inches * TICKS_PER_INCH as f64).round() as i32)
    }

    /// Nearest tick to a measurement in feet.
    #[inline]
    pub fn from_feet(feet: f64) -> Tick {
        Tick((feet * TICKS_PER_FOOT as f64).round() as i32)
    }

    /// This length in inches (a derived real).
    #[inline]
    pub fn to_inches(self) -> f64 {
        self.0 as f64 / TICKS_PER_INCH as f64
    }

    /// This length in feet (a derived real).
    #[inline]
    pub fn to_feet(self) -> f64 {
        self.0 as f64 / TICKS_PER_FOOT as f64
    }

    /// Absolute value.
    #[inline]
    pub const fn abs(self) -> Tick {
        Tick(self.0.abs())
    }

    /// The smaller of two ticks.
    #[inline]
    pub fn min(self, other: Tick) -> Tick {
        Tick(self.0.min(other.0))
    }

    /// The larger of two ticks.
    #[inline]
    pub fn max(self, other: Tick) -> Tick {
        Tick(self.0.max(other.0))
    }
}

impl Add for Tick {
    type Output = Tick;
    #[inline]
    fn add(self, rhs: Tick) -> Tick {
        Tick(self.0 + rhs.0)
    }
}
impl Sub for Tick {
    type Output = Tick;
    #[inline]
    fn sub(self, rhs: Tick) -> Tick {
        Tick(self.0 - rhs.0)
    }
}
impl Neg for Tick {
    type Output = Tick;
    #[inline]
    fn neg(self) -> Tick {
        Tick(-self.0)
    }
}
impl Mul<i32> for Tick {
    type Output = Tick;
    #[inline]
    fn mul(self, rhs: i32) -> Tick {
        Tick(self.0 * rhs)
    }
}
impl AddAssign for Tick {
    #[inline]
    fn add_assign(&mut self, rhs: Tick) {
        self.0 += rhs.0;
    }
}
impl SubAssign for Tick {
    #[inline]
    fn sub_assign(&mut self, rhs: Tick) {
        self.0 -= rhs.0;
    }
}

impl fmt::Debug for Tick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tick({} = {}in)", self.0, self.to_inches())
    }
}
impl fmt::Display for Tick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inch_and_foot_conversions() {
        assert_eq!(Tick::from_inches(1.0), Tick(32));
        assert_eq!(Tick::from_inches(3.5), Tick(112)); // dressed 2x4 face
        assert_eq!(Tick::from_feet(1.0), Tick(384));
        assert_eq!(Tick(112).to_inches(), 3.5);
        assert_eq!(Tick(384).to_feet(), 1.0);
    }

    #[test]
    fn rounds_to_nearest_tick() {
        // 1/64in is exactly half a tick -> rounds away from zero.
        assert_eq!(Tick::from_inches(1.0 / 64.0), Tick(1));
        assert_eq!(Tick::from_inches(-1.0 / 64.0), Tick(-1));
    }

    #[test]
    fn arithmetic_is_exact() {
        assert_eq!(Tick(112) + Tick(112), Tick(224));
        assert_eq!(Tick(384) - Tick(112), Tick(272));
        assert_eq!(-Tick(5), Tick(-5));
        assert_eq!(Tick(16) * 2, Tick(32));
        assert_eq!(Tick(-7).abs(), Tick(7));
    }
}
