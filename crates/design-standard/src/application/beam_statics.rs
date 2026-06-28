//! [`BeamStatics`] — pure, material-blind statics. `M = wL²/8`, `V = wL/2`, `Δ = 5wL⁴/384EI`.
//!
//! `E` and `I` are **injected** by the strategy/section; this service owns no material knowledge.
//! Every input/output is real engineering units — `w` in lb/in, `L` in inches, zero ticks.

/// Public-domain simple-span mechanics identical across all materials.
#[derive(Clone, Copy, Debug, Default)]
pub struct BeamStatics;

impl BeamStatics {
    /// Maximum moment of a uniformly loaded simple span, `M = wL²/8` (lb·in).
    pub fn moment(w: f64, l: f64) -> f64 {
        w * l * l / 8.0
    }

    /// Maximum shear of a uniformly loaded simple span, `V = wL/2` (lb).
    pub fn shear(w: f64, l: f64) -> f64 {
        w * l / 2.0
    }

    /// Midspan deflection of a uniformly loaded simple span, `Δ = 5wL⁴/384EI` (in). `E` (psi) and
    /// `I` (in⁴) are injected from the strategy's section basis.
    pub fn deflection(w: f64, l: f64, e: f64, i: f64) -> f64 {
        if e <= 0.0 || i <= 0.0 {
            return f64::INFINITY;
        }
        5.0 * w * l.powi(4) / (384.0 * e * i)
    }

    /// Resolve a serviceability ratio (e.g. 360 → L/360) to a real-inch allowable.
    pub fn deflection_limit(l: f64, ratio: f64) -> f64 {
        if ratio == 0.0 {
            return f64::INFINITY;
        }
        l / ratio
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_span_formulas() {
        // w = 100 lb/in, L = 144in: M = 100*144²/8 = 259_200 lb·in ; V = 100*144/2 = 7200 lb.
        assert!((BeamStatics::moment(100.0, 144.0) - 259_200.0).abs() < 1e-6);
        assert!((BeamStatics::shear(100.0, 144.0) - 7200.0).abs() < 1e-9);
        // L/360 of 144in = 0.4in.
        assert!((BeamStatics::deflection_limit(144.0, 360.0) - 0.4).abs() < 1e-9);
    }

    #[test]
    fn deflection_decreases_with_stiffness() {
        let soft = BeamStatics::deflection(10.0, 144.0, 1.0e6, 50.0);
        let stiff = BeamStatics::deflection(10.0, 144.0, 1.6e6, 100.0);
        assert!(stiff < soft);
        assert_eq!(
            BeamStatics::deflection(10.0, 144.0, 0.0, 100.0),
            f64::INFINITY
        );
    }
}
