//! [`LoadRollup`] — folds source loads down a [`LoadPath`](crate::LoadPath) order, owning the
//! `span × plf` and `area × psf` force arithmetic (the only place span and tributary area meet a
//! pressure). Produces per-member, per-source **unfactored** demand the combination then factors.

use crate::domain::demand::AccumulatedDemand;
use crate::domain::sources::{Effect, LoadSource, LoadSourcePayload};
use crate::domain::tributary::TributaryArea;
use building::MemberPlacementId;
use std::collections::BTreeMap;

/// How reactions are modeled at each node.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RollupMethod {
    /// Statically determinate tributary bearing.
    SimpleBearing,
    /// Multi-span continuous-beam reactions.
    ContinuousBeam,
}

/// The rollup service. Stateless apart from its reaction model.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LoadRollup {
    pub method: RollupMethod,
}

impl Default for LoadRollup {
    fn default() -> Self {
        LoadRollup {
            method: RollupMethod::SimpleBearing,
        }
    }
}

impl LoadRollup {
    /// Fold the gravity sources over the path `order`, accumulating per-member, per-source
    /// unfactored demand. Each member's own tributary load is computed as a line load; simple
    /// statics give the moment/shear/reaction. (Lateral sources are routed by the lateral path,
    /// not folded here.)
    pub fn roll(
        &self,
        order: &[MemberPlacementId],
        sources: &[LoadSource],
        tributaries: &[TributaryArea],
    ) -> Vec<AccumulatedDemand> {
        let trib_by: BTreeMap<MemberPlacementId, &TributaryArea> =
            tributaries.iter().map(|t| (t.member_ref, t)).collect();

        let mut out = Vec::new();
        for &member in order {
            let Some(trib) = trib_by.get(&member) else {
                continue; // no tributary attributed → nothing to fold for this node
            };
            let width_ft = trib.tributary_width.to_feet();
            let span_in = trib.span.to_inches();
            let span_ft = trib.span.to_feet();
            let at_ft2 = trib.area_ft2();

            for src in sources.iter().filter(|s| s.effect == Effect::Gravity) {
                let plf = source_line_load(src, width_ft, at_ft2);
                if plf == 0.0 {
                    continue;
                }
                let w_lb_per_in = plf / 12.0; // plf → lb/in
                let moment = w_lb_per_in * span_in * span_in / 8.0; // wL²/8, lb·in
                let shear = plf * span_ft / 2.0; // wL/2, lb
                out.push(AccumulatedDemand {
                    member_ref: member,
                    source_kind: src.kind(),
                    axial: 0.0, // axial accumulation is applied per role in the solver
                    line_load: plf,
                    moment,
                    shear,
                    reaction: shear, // one support carries half for a simple span
                });
            }
        }
        out
    }
}

/// The per-member line load (plf) a gravity source contributes over a tributary width. Dead adds
/// member self-weight plus the superimposed assembly dead; live uses its reduced pressure.
fn source_line_load(src: &LoadSource, width_ft: f64, at_ft2: f64) -> f64 {
    match &src.payload {
        LoadSourcePayload::Dead(d) => d.self_weight_plf + d.assembly_dead_psf * width_ft,
        LoadSourcePayload::Live(l) => l.reduced_psf_for(at_ft2) * width_ft,
        LoadSourcePayload::Snow(s) => s.design_snow() * width_ft,
        // Wind/seismic are lateral — they do not contribute to the gravity line load.
        LoadSourcePayload::Wind(_) | LoadSourcePayload::Seismic(_) => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::sources::{DeadLoad, LiveLoad, LiveOccupancy};
    use geometry_kernel::{Tick, UnitVec3};

    fn trib() -> TributaryArea {
        // 12ft span, 16in trib width.
        TributaryArea::strip(MemberPlacementId(1), Tick(4608), Tick(512))
    }

    #[test]
    fn rolls_dead_and_live_into_line_loads() {
        let sources = vec![
            LoadSource::dead(DeadLoad {
                self_weight_plf: 2.0,
                assembly_dead_psf: 10.0,
                direction: UnitVec3::Y.flipped(),
                source_ref: None,
            }),
            LoadSource::live(LiveLoad {
                occupancy: LiveOccupancy::LivingArea,
                base_psf: 40.0,
                is_roof_live: false,
                element_factor_kll: None,
                reduction_factor: None,
                reduced_psf: None,
                source_ref: None,
            }),
        ];
        let r = LoadRollup::default();
        let acc = r.roll(&[MemberPlacementId(1)], &sources, &[trib()]);
        assert_eq!(acc.len(), 2);
        // Dead plf = 2 + 10 * (16/12 ft) = 2 + 13.333… = 15.333…
        let dead = acc
            .iter()
            .find(|a| {
                a.member_ref == MemberPlacementId(1) && a.line_load > 14.0 && a.line_load < 16.0
            })
            .unwrap();
        assert!((dead.line_load - (2.0 + 10.0 * (16.0 / 12.0))).abs() < 1e-9);
        // Live plf = 40 * (16/12) = 53.333…
        let live = acc.iter().find(|a| a.line_load > 50.0).unwrap();
        assert!((live.line_load - 40.0 * (16.0 / 12.0)).abs() < 1e-9);
        // Shear = wL/2 for the live case: plf * 12ft / 2.
        assert!((live.shear - live.line_load * 12.0 / 2.0).abs() < 1e-9);
    }
}
