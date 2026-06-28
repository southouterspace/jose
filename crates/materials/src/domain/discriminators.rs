//! The two material-seam discriminators: [`MaterialClass`] (what a stock is made of) and
//! [`StockForm`] (its geometric form, which decides the pipeline output path).
//!
//! Routing is by `stockForm`, never by `material` — the rule that lets concrete bypass the
//! linear cut optimizer without the core knowing about concrete.

use reference_data::{MaterialFamilyKey, StandardKey};

/// Discriminator naming the material family a [`Stock`](crate::Stock) is made of — the first
/// half of the material seam. Carries no physics; it only names the family so the
/// `DesignStandard` strategy routes to the correct leaf (NDS_Wood / AISI_CFS / …).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MaterialClass {
    /// `wood` | `cold-formed-steel` | `hot-rolled-steel` | `concrete` | `masonry`. Open key —
    /// adding a value is the only core change to onboard a new material family.
    pub material: MaterialFamilyKey,
    /// Opaque key into the design-standard catalog (`NDS` | `AISI` | `AISC` | `ACI` | `TMS`).
    /// Decouples material identity from the strategy implementation; never a mechanical value.
    pub design_standard_ref: StandardKey,
}

/// The geometric form of raw stock. Decides the pipeline output path: `linear` → cut
/// optimizer, `sheet` → nesting, `cast` → formwork/volume, `unit` → count.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Form {
    /// Dimensional lumber / studs / mill lengths → linear cut optimizer + offcut pool.
    Linear,
    /// Sheet goods (OSB, plywood, gypsum) → nesting.
    Sheet,
    /// Cast-in-place concrete → formwork + volume, bypassing the cut optimizer.
    Cast,
    /// Laid units (CMU, brick) → unit count + grout/mortar volume.
    Unit,
}

/// The quantity dimension the estimator rolls up for a given [`Form`]. Canonically determined
/// by form; [`QuantityTakeoff`](crate::QuantityTakeoff) mirrors it.
///
/// Open in spirit (board-feet is wood-only, not core) but the five canonical bases are fixed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum QuantityBasis {
    /// Linear feet (linear stock).
    LinearFeet,
    /// Board feet (wood-only volumetric-by-nominal measure).
    BoardFeet,
    /// Area in ft² (sheet goods).
    Area,
    /// Volume in ft³ (cast).
    Volume,
    /// Count of units (masonry/CMU).
    Count,
}

/// Discriminator naming the geometric form of raw stock plus the takeoff basis it implies.
/// `stockForm` — not material — chooses cut vs nest vs formwork, and is the canonical source of
/// the takeoff dimension every [`QuantityTakeoff`](crate::QuantityTakeoff) mirrors.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StockForm {
    /// The geometric form, which selects the downstream solver.
    pub form: Form,
    /// The quantity dimension the estimator rolls up — the explicit takeoff contract,
    /// single-sourced here.
    pub takeoff_basis: QuantityBasis,
}

impl StockForm {
    /// The conventional takeoff basis for each form (linear→lf, sheet→area, cast→volume,
    /// unit→count). Board-feet, being wood-only, is opted into explicitly rather than implied.
    pub fn for_form(form: Form) -> StockForm {
        let takeoff_basis = match form {
            Form::Linear => QuantityBasis::LinearFeet,
            Form::Sheet => QuantityBasis::Area,
            Form::Cast => QuantityBasis::Volume,
            Form::Unit => QuantityBasis::Count,
        };
        StockForm {
            form,
            takeoff_basis,
        }
    }

    /// Whether this form routes to the linear cut optimizer (and so emits cut waste).
    pub fn is_linear(self) -> bool {
        matches!(self.form, Form::Linear)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_picks_canonical_basis() {
        assert_eq!(
            StockForm::for_form(Form::Linear).takeoff_basis,
            QuantityBasis::LinearFeet
        );
        assert_eq!(
            StockForm::for_form(Form::Cast).takeoff_basis,
            QuantityBasis::Volume
        );
        assert_eq!(
            StockForm::for_form(Form::Unit).takeoff_basis,
            QuantityBasis::Count
        );
        assert!(StockForm::for_form(Form::Linear).is_linear());
        assert!(!StockForm::for_form(Form::Cast).is_linear());
    }

    #[test]
    fn material_class_routes_by_open_key() {
        let mc = MaterialClass {
            material: MaterialFamilyKey {
                key: "wood".into(),
                standard_key: StandardKey::from("nds"),
                default_stock_form: reference_data::StockForm::from("dimensional-lumber"),
            },
            design_standard_ref: StandardKey::from("nds"),
        };
        assert_eq!(mc.material.key, "wood");
    }
}
