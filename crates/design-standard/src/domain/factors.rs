//! [`ModificationFactor`] (the single home for the applied factor stack) and [`FactorContext`]
//! (the per-member install context that selects it).
//!
//! The intrinsic/contextual split: the factor *value* applied to a member is mostly contextual
//! (load duration, spacing, bracing), so factors live per-placement — never folded into the
//! intrinsic `MechanicalProperties` flyweight.

use crate::domain::limit_state::LimitStateId;
use crate::domain::philosophy::FactorSide;
use loads_analysis::DurationClass;

/// An open identifier for a modification factor (`CD`|`CM`|`Cr`|`CP`|`CL` for wood; `phi`|`omega`|
/// `Q` for steel; …). Leaves extend it.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FactorId(pub String);

impl FactorId {
    /// Borrow the id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for FactorId {
    fn from(s: &str) -> Self {
        FactorId(s.to_owned())
    }
}

/// Whether a factor is intrinsic to the section or contextual to the install. Only size (`CF`) is
/// genuinely intrinsic; `CD`/`Cr`/`CP`/`CL`/`φ`/`Q` are contextual.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FactorKind {
    Intrinsic,
    Contextual,
}

/// One applied modification factor in the strategy's stack. Capacity = base design value ×
/// Π(factor.value) on the applicable side/mode.
#[derive(Clone, PartialEq, Debug)]
pub struct ModificationFactor {
    /// Which factor this is.
    pub id: FactorId,
    /// Multiplier applied to the base design value (e.g. CD=1.0, CF=1.1, φ=0.90).
    pub value: f64,
    /// Intrinsic vs contextual — fixes the storage location ambiguity.
    pub kind: FactorKind,
    /// Which side the factor hits (where per-mode Ω/φ live), if applicable.
    pub applies_to: Option<FactorSide>,
    /// If mode-specific (φ=0.90 flexure vs 0.75 shear), the mode it applies to. `None` = stack-wide.
    pub limit_state: Option<LimitStateId>,
    /// What contextual input selected this factor (the load case for CD, etc.).
    pub source: Option<String>,
}

impl ModificationFactor {
    /// A stack-wide contextual factor (applies to every mode).
    pub fn contextual(id: impl Into<String>, value: f64) -> ModificationFactor {
        ModificationFactor {
            id: FactorId(id.into()),
            value,
            kind: FactorKind::Contextual,
            applies_to: None,
            limit_state: None,
            source: None,
        }
    }

    /// Whether this factor applies to `mode` (stack-wide factors apply to all).
    pub fn applies_to_mode(&self, mode: &LimitStateId) -> bool {
        self.limit_state.as_ref().is_none_or(|m| m == mode)
    }
}

/// Service condition selecting the wet-service factor `CM`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MoistureCondition {
    Dry,
    Wet,
}

/// The contextual inputs a strategy needs to select the applied factor stack — the request type
/// for `modificationFactors()`. Everything here is per-member install context, not intrinsic spec
/// data.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct FactorContext {
    /// Load-duration class → wood `CD` (from the governing load case, not the stick).
    pub load_duration: Option<DurationClass>,
    /// Service condition → `CM`.
    pub moisture_condition: Option<MoistureCondition>,
    /// `≥ 3` members `@ ≤ 24in` OC → `Cr`.
    pub repetitive: Option<bool>,
    /// `Le` for column (CP) / beam (CL) stability — real inches (converted from tick spacing).
    pub unbraced_length_in: Option<f64>,
    /// Loaded on the flat → `CFu`.
    pub flat_use: Option<bool>,
    /// Pressure-treatment incising → `Ci`.
    pub incised: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stackwide_factor_applies_to_every_mode() {
        let cd = ModificationFactor::contextual("CD", 1.0);
        assert!(cd.applies_to_mode(&LimitStateId::bending()));
        assert!(cd.applies_to_mode(&LimitStateId::shear()));
    }

    #[test]
    fn mode_specific_factor_is_targeted() {
        let phi = ModificationFactor {
            id: FactorId::from("phi"),
            value: 0.9,
            kind: FactorKind::Contextual,
            applies_to: Some(FactorSide::Resistance),
            limit_state: Some(LimitStateId::bending()),
            source: None,
        };
        assert!(phi.applies_to_mode(&LimitStateId::bending()));
        assert!(!phi.applies_to_mode(&LimitStateId::shear()));
    }
}
