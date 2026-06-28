//! [`CutEligibility`] — the typed `stockForm` gate (design-standard gotcha #3).
//!
//! Cut optimization + offcut reuse is a capability of `stockForm=linear` ONLY. Sheet goods route
//! to nesting, cast routes to formwork+volume, unit stock bypasses entirely. Modeling the verdict
//! as a value object — not an implicit assumption — is what keeps the solver from ever growing a
//! per-material branch: adding hot-rolled (linear) or masonry (unit/cast) later only sets
//! `stockForm` upstream behind the `DesignStandard` seam.

use materials::Form;

/// Where an *ineligible* demand is routed instead of the cutting-stock solver.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RouteTo {
    /// Sheet goods → nesting.
    Nest,
    /// Cast → formwork + volume.
    FormworkVolume,
    /// Unit stock → straight to the estimate, no cutting.
    Direct,
}

/// The `stockForm` verdict that decides whether a demand may enter the optimizer. Read the form
/// from the spec (the `DesignStandard` seam owns the value) and call [`CutEligibility::classify`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CutEligibility {
    /// The form read from the spec — only [`Form::Linear`] is eligible.
    pub stock_form: Form,
    /// Derived: `stock_form == Linear`. False routes the demand away from this layer.
    pub eligible: bool,
    /// When ineligible, names the downstream output path; `None` when eligible.
    pub route_to: Option<RouteTo>,
}

impl CutEligibility {
    /// Classify a stock form into the cut-eligibility verdict. The form→route mapping is owned
    /// here only as a *record* of the seam's decision; it never duplicates per-material logic.
    pub fn classify(stock_form: Form) -> CutEligibility {
        let (eligible, route_to) = match stock_form {
            Form::Linear => (true, None),
            Form::Sheet => (false, Some(RouteTo::Nest)),
            Form::Cast => (false, Some(RouteTo::FormworkVolume)),
            Form::Unit => (false, Some(RouteTo::Direct)),
        };
        CutEligibility {
            stock_form,
            eligible,
            route_to,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_linear_is_eligible() {
        assert!(CutEligibility::classify(Form::Linear).eligible);
        assert_eq!(CutEligibility::classify(Form::Linear).route_to, None);

        let sheet = CutEligibility::classify(Form::Sheet);
        assert!(!sheet.eligible);
        assert_eq!(sheet.route_to, Some(RouteTo::Nest));

        assert_eq!(
            CutEligibility::classify(Form::Cast).route_to,
            Some(RouteTo::FormworkVolume)
        );
        assert_eq!(
            CutEligibility::classify(Form::Unit).route_to,
            Some(RouteTo::Direct)
        );
    }
}
