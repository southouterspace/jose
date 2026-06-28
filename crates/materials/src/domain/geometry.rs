//! Parametric stock geometry: [`Dimensions`] (the tick-invariant reference implementation)
//! and [`SectionProperties`] (pure cross-section structural geometry).
//!
//! Linear fields are integer [`Tick`]s; area/volume/section properties are **derived reals**
//! (in², in³, in⁴), never stored as tick² — the base-unit invariant the whole schema conforms
//! to (finding S2-D).

use crate::keys::ProfileKey;
use geometry_kernel::Tick;

/// Dressed-edge bevel of a stock member.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EdgeProfile {
    /// Rounded (eased) edge — affects render bevel and nominal subtraction.
    Eased,
    /// Sharp square edge.
    Square,
}

/// Which bending axis a [`SectionProperties`] describes. The strong/weak choice is a ~5–6×
/// swing in bending capacity for the same stick.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Axis {
    /// Loaded on the wide face — the stiff axis (joist orientation).
    Strong,
    /// Loaded on the narrow face — the flexible axis.
    Weak,
}

/// Parametric geometry of a stock member: `length` is the free variable; the cross-section is
/// fixed by nominal size. The canonical tick-invariant reference — linear fields are integer
/// ticks, area/volume are derived reals (never tick²).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Dimensions {
    /// Naming only, e.g. `2x4` (wood) / `600S162` (CFS) — not real dimensions.
    pub nominal_size: String,
    /// THE parameter: member length in 1/32in ticks (3072 = 96in).
    pub length: Tick,
    /// Dressed face width in ticks (112 = 3.5in).
    pub actual_width: Tick,
    /// Dressed edge thickness in ticks (48 = 1.5in).
    pub actual_thickness: Tick,
    /// Open profile key (`rectangular` | `I` | `C` | `HSS` | `round`); `None` = rectangular.
    pub cross_section_shape: Option<ProfileKey>,
    /// Edge bevel; `None` leaves it unspecified.
    pub edge_profile: Option<EdgeProfile>,
}

impl Dimensions {
    /// A rectangular member of the given nominal name and tick dimensions.
    pub fn rectangular(
        nominal_size: impl Into<String>,
        length: Tick,
        actual_width: Tick,
        actual_thickness: Tick,
    ) -> Dimensions {
        Dimensions {
            nominal_size: nominal_size.into(),
            length,
            actual_width,
            actual_thickness,
            cross_section_shape: None,
            edge_profile: None,
        }
    }

    /// Derived single-ply **gross** face area in in² (label / quick-calc). Explicitly a real,
    /// not tick² — the counter-example to a tick²-typed area. For the axis-aware, ply-multiplied
    /// structural area use [`SectionProperties::area`].
    pub fn cross_section_area_in2(&self) -> f64 {
        self.actual_width.to_inches() * self.actual_thickness.to_inches()
    }

    /// Derived volume in in³ = `(length/32) × crossSectionArea`. Feeds [`Weight`](crate::Weight)
    /// and the dead-load rollup.
    pub fn volume_in3(&self) -> f64 {
        self.length.to_inches() * self.cross_section_area_in2()
    }
}

/// Pure cross-section structural geometry: area `A`, section modulus `S`, moment of inertia `I`
/// about a chosen bending axis. Public-domain math identical across materials. Carries no `E`
/// and no design values — the strategy injects stiffness and computes effective/cracked/
/// transformed bases; this is the gross geometry.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SectionProperties {
    /// `A = b·d·ply`, the structural section area — real in², never tick² (fixes S2-D).
    pub area: f64,
    /// `S = b·d²/6·ply` about this axis, real in³.
    pub section_modulus: f64,
    /// `I = b·d³/12·ply` about this axis, real in⁴. Feeds beam-deflection statics.
    pub moment_of_inertia: f64,
    /// Which bending axis these properties describe.
    pub axis: Axis,
    /// Number of laminations (built-up members); multiplies `A`/`S`/`I`.
    pub ply: u32,
}

impl SectionProperties {
    /// Gross rectangular section properties from real breadth/depth (inches), about `axis`,
    /// for a `ply`-lamination member. `b` is the dimension parallel to the bending axis; `d` is
    /// the depth resisting the moment.
    pub fn rectangular(b_in: f64, d_in: f64, axis: Axis, ply: u32) -> SectionProperties {
        let p = ply.max(1) as f64;
        SectionProperties {
            area: b_in * d_in * p,
            section_modulus: b_in * d_in * d_in / 6.0 * p,
            moment_of_inertia: b_in * d_in * d_in * d_in / 12.0 * p,
            axis,
            ply: ply.max(1),
        }
    }

    /// Section properties of a dressed rectangular member bending about `axis`. For the strong
    /// axis the depth is the wider dressed dimension; for the weak axis it is the narrower.
    pub fn from_dimensions(dims: &Dimensions, axis: Axis, ply: u32) -> SectionProperties {
        let w = dims.actual_width.to_inches();
        let t = dims.actual_thickness.to_inches();
        let (b, d) = match axis {
            // Strong: load on the wide face → depth is the larger dimension.
            Axis::Strong => (t, w),
            // Weak: depth is the smaller dimension.
            Axis::Weak => (w, t),
        };
        SectionProperties::rectangular(b, d, axis, ply)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_by_four() -> Dimensions {
        // 2x4: 1.5in x 3.5in, 8ft long.
        Dimensions::rectangular("2x4", Tick(3072), Tick(112), Tick(48))
    }

    #[test]
    fn derived_area_and_volume_are_real() {
        let d = two_by_four();
        assert!((d.cross_section_area_in2() - 5.25).abs() < 1e-9); // 1.5 * 3.5
        // 96in * 5.25in² = 504 in³.
        assert!((d.volume_in3() - 504.0).abs() < 1e-9);
    }

    #[test]
    fn strong_axis_modulus_exceeds_weak() {
        let d = two_by_four();
        let strong = SectionProperties::from_dimensions(&d, Axis::Strong, 1);
        let weak = SectionProperties::from_dimensions(&d, Axis::Weak, 1);
        // S_strong = 1.5*3.5²/6 = 3.0625 ; S_weak = 3.5*1.5²/6 = 1.3125
        assert!((strong.section_modulus - 3.0625).abs() < 1e-9);
        assert!((weak.section_modulus - 1.3125).abs() < 1e-9);
        assert!(strong.section_modulus > weak.section_modulus);
    }

    #[test]
    fn ply_multiplies_properties() {
        let single = SectionProperties::rectangular(1.5, 9.25, Axis::Strong, 1);
        let triple = SectionProperties::rectangular(1.5, 9.25, Axis::Strong, 3);
        assert!((triple.moment_of_inertia - single.moment_of_inertia * 3.0).abs() < 1e-6);
        assert_eq!(triple.ply, 3);
    }
}
