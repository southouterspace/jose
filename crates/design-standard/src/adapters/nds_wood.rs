//! [`NdsWood`] — the wood leaf, the reference implementation, **fully populated**. Implements
//! [`DesignStandard`] for material = wood, code = NDS, philosophy = ASD.
//!
//! It single-sources its design values by holding the one canonical `MechanicalProperties`
//! flyweight (referenced by key) rather than restating Fb/Fv/E. It adds the wood-specific
//! strategy modes (column buckling `CP`, bearing `Fc⊥`) the core does not enumerate.

use crate::domain::connection::{Connection, ConnectionCapacity, ConnectionMethod};
use crate::domain::factors::{FactorContext, ModificationFactor, MoistureCondition};
use crate::domain::limit_state::{CheckOrigin, LimitStateCheck, LimitStateId};
use crate::domain::philosophy::{DesignCode, DesignPhilosophy, MaterialKind, SectionBasisKind};
use crate::domain::section::SectionBasis;
use crate::domain::sizing::SizingQuery;
use crate::keys::{DesignStandardId, MechanicalPropertiesKey};
use crate::ports::design_standard::DesignStandard;
use loads_analysis::{DurationClass, LoadCombination};
use reference_data::{CitationKey, MechanicalProperties};
use std::collections::BTreeMap;

/// Euler-buckling coefficient for the NDS column-stability equation (0.8 for sawn lumber).
const C_SAWN: f64 = 0.8;

/// The wood leaf, seeded with a canonical species/grade (Douglas Fir-Larch No.2).
#[derive(Clone, PartialEq, Debug)]
pub struct NdsWood {
    /// The leaf-selecting key, e.g. `NDS-2018`.
    pub id: DesignStandardId,
    /// Key into the `MechanicalProperties` flyweight (single canonical wood design-value source).
    pub design_values_ref: MechanicalPropertiesKey,
    /// The resolved reference design values (the flyweight this leaf points at).
    pub props: MechanicalProperties,
}

impl NdsWood {
    /// NDS-2018 seeded with DF-L No.2: Fb 900, Ft 575, Fc 1350, Fc⊥ 625, Fv 180, E 1.6e6,
    /// Emin 580k (psi).
    pub fn df_l_no2() -> NdsWood {
        NdsWood {
            id: DesignStandardId::from("NDS-2018"),
            design_values_ref: MechanicalPropertiesKey::from("df-l-no2"),
            props: MechanicalProperties {
                fb: 900.0,
                ft: 575.0,
                fc: 1350.0,
                fc_perp: 625.0,
                fv: 180.0,
                e: 1.6e6,
                e_min: 580_000.0,
                g: Some(0.50),
                source_ref: Some(CitationKey::book("NDS").at("Table 4A").edition("2018")),
            },
        }
    }

    /// The product of every stack-wide factor (those applicable to all modes) — the load-duration
    /// and service adjustments that scale every allowable stress.
    fn stackwide(factors: &[ModificationFactor]) -> f64 {
        factors
            .iter()
            .filter(|f| f.limit_state.is_none())
            .fold(1.0, |acc, f| acc * f.value)
    }
}

impl DesignStandard for NdsWood {
    fn id(&self) -> DesignStandardId {
        self.id.clone()
    }
    fn code(&self) -> DesignCode {
        DesignCode::Nds
    }
    fn material(&self) -> MaterialKind {
        MaterialKind::Wood
    }
    fn section_basis(&self) -> SectionBasisKind {
        SectionBasisKind::Gross
    }
    fn philosophy(&self) -> DesignPhilosophy {
        DesignPhilosophy::ASD
    }

    fn design_values(&self, query: &SizingQuery) -> SectionBasis {
        let mut strengths = BTreeMap::new();
        strengths.insert("Fb".to_owned(), self.props.fb);
        strengths.insert("Ft".to_owned(), self.props.ft);
        strengths.insert("Fc".to_owned(), self.props.fc);
        strengths.insert("FcPerp".to_owned(), self.props.fc_perp);
        strengths.insert("Fv".to_owned(), self.props.fv);
        SectionBasis {
            basis: SectionBasisKind::Gross,
            design_values_ref: Some(self.design_values_ref.clone()),
            strengths,
            e: self.props.e,
            e_min: Some(self.props.e_min),
            section: query.section, // wood uses the gross section unchanged
        }
    }

    fn modification_factors(&self, ctx: &FactorContext) -> Vec<ModificationFactor> {
        let mut out = Vec::new();
        // CD — load-duration (the default ten-year normal duration is 1.0).
        let cd = match ctx.load_duration {
            Some(DurationClass::Permanent) => 0.9,
            Some(DurationClass::TenYear) | None => 1.0,
            Some(DurationClass::TwoMonth) => 1.15,
            Some(DurationClass::SevenDay) => 1.25,
            Some(DurationClass::TenMinute) => 1.6,
            Some(DurationClass::Impact) => 2.0,
        };
        out.push(ModificationFactor::contextual("CD", cd));
        // CM — wet-service factor.
        if let Some(MoistureCondition::Wet) = ctx.moisture_condition {
            out.push(ModificationFactor::contextual("CM", 0.85));
        }
        // Cr — repetitive-member factor (studs/joists ≥3 @ ≤24in OC).
        if ctx.repetitive == Some(true) {
            out.push(ModificationFactor::contextual("Cr", 1.15));
        }
        out
    }

    fn extra_limit_states(&self) -> Vec<LimitStateId> {
        vec![LimitStateId::column_buckling(), LimitStateId::bearing()]
    }

    fn strategy_check(
        &self,
        mode: &LimitStateId,
        query: &SizingQuery,
        section: &SectionBasis,
        factors: &[ModificationFactor],
    ) -> Option<LimitStateCheck> {
        let area = section.section.area;
        if area <= 0.0 {
            return None;
        }
        let stack = Self::stackwide(factors);

        if *mode == LimitStateId::column_buckling() {
            // CP column-stability (NDS 3.7.1). Depth recovered from the rectangular section.
            let i = section.section.moment_of_inertia;
            let depth = (12.0 * i / area).sqrt();
            let le = query.context.unbraced_length_in.unwrap_or(query.span_in);
            if depth <= 0.0 || le <= 0.0 {
                return None;
            }
            let slenderness = le / depth;
            let e_min = section.e_min.unwrap_or(self.props.e_min);
            let fc_star = section.strength("Fc")? * stack;
            let fce = 0.822 * e_min / (slenderness * slenderness);
            let r = fce / fc_star;
            let base = (1.0 + r) / (2.0 * C_SAWN);
            let cp = base - (base * base - r / C_SAWN).max(0.0).sqrt();
            let capacity = fc_star * cp * area;
            return Some(LimitStateCheck::new(
                LimitStateId::column_buckling(),
                query.demand.axial.abs(),
                capacity,
                CheckOrigin::Strategy,
            ));
        }

        if *mode == LimitStateId::bearing() {
            // Bearing crush on Fc⊥ over the section's bearing area.
            let fc_perp = section.strength("FcPerp")? * stack;
            let capacity = fc_perp * area;
            let demand = query.demand.reaction.unwrap_or(query.demand.shear).abs();
            return Some(LimitStateCheck::new(
                LimitStateId::bearing(),
                demand,
                capacity,
                CheckOrigin::Strategy,
            ));
        }

        None
    }

    fn connection_capacity(&self, conn: &Connection) -> ConnectionCapacity {
        // EYM single-fastener yield values (placeholder reference Z, lb).
        let z = match conn.method {
            ConnectionMethod::NailYield => 100.0,
            ConnectionMethod::ScrewShear | ConnectionMethod::ScrewPullout => 130.0,
            _ => 0.0, // non-wood joints aren't this leaf's concern
        };
        ConnectionCapacity {
            method: conn.method,
            z,
            group_factor: None,
        }
    }

    fn governing_combination(&self, combos: &[LoadCombination]) -> Option<LoadCombination> {
        // Wood ASD governs on the combination giving the highest load-to-duration ratio. As a
        // material-blind stand-in (loads are not modeled here) the combo with the largest factor
        // sum is taken; the arbiter records demand under the loads layer's actual pick.
        combos
            .iter()
            .max_by(|a, b| {
                let sa: f64 = a.terms.iter().map(|t| t.factor).sum();
                let sb: f64 = b.terms.iter().map(|t| t.factor).sum();
                sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::factors::FactorContext;
    use loads_analysis::{MemberDemand, MemberRole};
    use materials::{Axis, SectionProperties, SpecKey};

    fn query(axial: f64, moment: f64) -> SizingQuery {
        SizingQuery {
            member_id: building::MemberPlacementId(1),
            demand: MemberDemand {
                member_ref: building::MemberPlacementId(1),
                member_role: MemberRole::Beam,
                governing_combo: "LRFD-2".to_owned(),
                axial,
                moment,
                shear: 500.0,
                deflection: None,
                deflection_limit: 0.4,
                deflection_ratio: Some(360),
                uniform_load: Some(50.0),
                span: geometry_kernel::Tick(4608),
                unbraced_length: None,
                reaction: Some(400.0),
                applied_at: None,
            },
            candidate_spec: SpecKey::from("SPF-2x8"),
            span_in: 144.0,
            section: SectionProperties::rectangular(1.5, 7.25, Axis::Strong, 1),
            context: FactorContext::default(),
        }
    }

    #[test]
    fn design_values_reference_the_flyweight() {
        let leaf = NdsWood::df_l_no2();
        let basis = leaf.design_values(&query(0.0, 1000.0));
        assert_eq!(basis.strength("Fb"), Some(900.0));
        assert_eq!(
            basis.design_values_ref,
            Some(MechanicalPropertiesKey::from("df-l-no2"))
        );
        assert_eq!(basis.basis, SectionBasisKind::Gross);
    }

    #[test]
    fn cd_scales_with_load_duration() {
        let leaf = NdsWood::df_l_no2();
        let normal = leaf.modification_factors(&FactorContext::default());
        assert!((normal[0].value - 1.0).abs() < 1e-9); // ten-year default
        let short = leaf.modification_factors(&FactorContext {
            load_duration: Some(DurationClass::TenMinute),
            ..FactorContext::default()
        });
        assert!((short[0].value - 1.6).abs() < 1e-9);
    }

    #[test]
    fn column_buckling_check_is_produced() {
        let leaf = NdsWood::df_l_no2();
        let q = query(2000.0, 0.0);
        let basis = leaf.design_values(&q);
        let check = leaf
            .strategy_check(&LimitStateId::column_buckling(), &q, &basis, &[])
            .unwrap();
        assert_eq!(check.origin, CheckOrigin::Strategy);
        assert!(check.capacity > 0.0);
        assert!((check.demand - 2000.0).abs() < 1e-9);
    }
}
