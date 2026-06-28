//! [`SizingArbiter`] — the material-blind orchestrator of the structural-check stage.
//!
//! It holds no design values. It runs [`BeamStatics`] for the core demand-vs-capacity checks
//! (bending/shear/deflection, capacities injected by the strategy), asks the strategy for its
//! extra material-specific checks, aggregates the governing utilization, and returns a
//! [`SizingResult`]. Identical code for wood, steel, concrete, masonry — only the injected strategy
//! differs. It never branches on material.

use crate::application::beam_statics::BeamStatics;
use crate::domain::factors::ModificationFactor;
use crate::domain::limit_state::{CheckOrigin, LimitStateCheck, LimitStateId};
use crate::domain::philosophy::DesignPhilosophy;
use crate::domain::sizing::{Escape, SizingMethod, SizingQuery, SizingResult};
use crate::ports::design_standard::DesignStandard;

/// The keystone of material-blindness. Carries the cross-cutting ASD/LRFD flag (defaulting to the
/// strategy's declared philosophy) and the prescriptive-first policy.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SizingArbiter {
    /// A per-project ASD/LRFD override; `None` uses the strategy's declared philosophy.
    pub philosophy_override: Option<DesignPhilosophy>,
    /// When true, try the strategy's prescriptive table before closed-form sizing.
    pub prescriptive_first: bool,
}

impl SizingArbiter {
    /// A default arbiter (no override, closed-form first).
    pub fn new() -> SizingArbiter {
        SizingArbiter::default()
    }

    /// Size a member through the supplied strategy. Material-blind: every material-specific value
    /// is fetched **through** the interface.
    pub fn size(&self, query: &SizingQuery, standard: &dyn DesignStandard) -> SizingResult {
        let philosophy = self
            .philosophy_override
            .unwrap_or_else(|| standard.philosophy());
        let basis = standard.design_values(query);
        let factors = standard.modification_factors(&query.context);
        let demand = &query.demand;

        let mut checks: Vec<LimitStateCheck> = Vec::new();

        // Core bending: capacity = adjusted bending stress × section modulus.
        if let Some(fb) = basis.strength("Fb").or_else(|| basis.strength("Fy")) {
            let capacity =
                adjusted(fb, &factors, &LimitStateId::bending()) * basis.section.section_modulus;
            checks.push(LimitStateCheck::new(
                LimitStateId::bending(),
                demand.moment,
                capacity,
                CheckOrigin::Core,
            ));
        }

        // Core shear: rectangular allowable V = (2/3)·Fv·A.
        if let Some(fv) = basis.strength("Fv") {
            let capacity =
                adjusted(fv, &factors, &LimitStateId::shear()) * basis.section.area * (2.0 / 3.0);
            checks.push(LimitStateCheck::new(
                LimitStateId::shear(),
                demand.shear,
                capacity,
                CheckOrigin::Core,
            ));
        }

        // Core deflection (serviceability): computed deflection vs the loads layer's limit.
        if let Some(w_plf) = demand.uniform_load {
            let w_per_in = w_plf / 12.0;
            let delta = BeamStatics::deflection(
                w_per_in,
                query.span_in,
                basis.e,
                basis.section.moment_of_inertia,
            );
            checks.push(LimitStateCheck::new(
                LimitStateId::deflection(),
                delta,
                demand.deflection_limit,
                CheckOrigin::Core,
            ));
        }

        // Strategy-supplied modes (CP, web crippling, …): the leaf computes; the arbiter only
        // enumerates. A `None` is a declared-but-unpopulated mode (stub leaf).
        for mode in standard.extra_limit_states() {
            if let Some(check) = standard.strategy_check(&mode, query, &basis, &factors) {
                checks.push(check);
            }
        }

        let governing = checks.iter().max_by(|a, b| {
            a.ratio
                .partial_cmp(&b.ratio)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let (governing_check, utilization) = match governing {
            Some(c) => (c.id.clone(), c.ratio),
            None => (LimitStateId::bending(), 0.0),
        };
        let pass = checks.iter().all(|c| c.pass);

        SizingResult {
            pass,
            governing_check,
            utilization,
            checks,
            sized_spec: query.candidate_spec.clone(),
            method: Some(SizingMethod::ClosedForm),
            philosophy_used: philosophy,
            escape: Some(if pass {
                Escape::Ok
            } else {
                Escape::EngineeredDesign
            }),
        }
    }
}

/// Apply the factor stack to a base design value for a given mode: `base × Π(factor.value)` over
/// the factors that apply to that mode (stack-wide factors apply to all).
fn adjusted(base: f64, factors: &[ModificationFactor], mode: &LimitStateId) -> f64 {
    factors
        .iter()
        .filter(|f| f.applies_to_mode(mode))
        .fold(base, |acc, f| acc * f.value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::factors::FactorId;

    #[test]
    fn adjusted_multiplies_applicable_factors() {
        let factors = vec![
            ModificationFactor::contextual("CD", 1.6),
            ModificationFactor {
                id: FactorId::from("phi"),
                value: 0.9,
                kind: crate::domain::factors::FactorKind::Contextual,
                applies_to: None,
                limit_state: Some(LimitStateId::shear()), // only shear
                source: None,
            },
        ];
        // Bending mode: only CD (stack-wide) applies → 900 * 1.6.
        assert!((adjusted(900.0, &factors, &LimitStateId::bending()) - 1440.0).abs() < 1e-9);
        // Shear mode: CD and phi → 180 * 1.6 * 0.9.
        assert!(
            (adjusted(180.0, &factors, &LimitStateId::shear()) - 180.0 * 1.6 * 0.9).abs() < 1e-9
        );
    }
}
