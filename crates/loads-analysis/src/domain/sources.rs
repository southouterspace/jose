//! The ASCE 7 / IRC load **sources** — `D`, `L`/`Lr`, `S`, `W`, `E` — as material-blind value
//! objects that speak psf/plf/lb, plus the [`LoadSource`] discriminated union over them.
//!
//! Each source quantifies its design pressure per code; none of them knows about wood/steel/
//! concrete. Derived figures (reduced live load, flat-roof snow, velocity pressure, base shear)
//! are real numbers — area is never tick².

use geometry_kernel::UnitVec3;
use reference_data::CitationKey;

/// The ASCE 7 load symbol identifying a source. Open in spirit (rain `R`, flood `Fa`, ice slot in
/// without changing rollup/combination/demand signatures).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SourceKind {
    Dead,
    Live,
    RoofLive,
    Snow,
    Wind,
    Seismic,
}

/// How a source resolves onto a load path — replaces a raw direction enum, preserving the
/// gravity-vs-lateral-vs-uplift distinction that routes the source.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Effect {
    Gravity,
    Lateral,
    Uplift,
}

/// Permanent gravity load `D`: member self-weight (referenced from `materials::Weight`, real plf)
/// plus superimposed fixed assembly dead (sheathing, finishes, MEP). No total is stored —
/// `selfWeightPlf × span + assemblyDeadPsf × tributaryArea` is derived in the rollup.
#[derive(Clone, PartialEq, Debug)]
pub struct DeadLoad {
    /// Member self-weight as a line load (plf) — the bridge from the mass model. Not recomputed.
    pub self_weight_plf: f64,
    /// Superimposed dead of the supported assembly (psf), converted to force by the tributary area.
    pub assembly_dead_psf: f64,
    /// Gravity direction (0,-1,0) — a unitless direction, never a tick position.
    pub direction: UnitVec3,
    /// Provenance into the reference library (ASCE 7 Table C3 / IRC R301).
    pub source_ref: Option<CitationKey>,
}

/// Room/space occupancy class keying the unreduced live load per IRC R301.5 / ASCE 7 Table 4.3-1.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LiveOccupancy {
    SleepingRoom,
    LivingArea,
    AtticStorage,
    Deck,
    Stairs,
    RoofLr,
}

/// Transient occupancy load `L`/`Lr`. Carries the inputs to the code-permitted live-load
/// reduction so the reduction is recomputable, not a stale scalar.
#[derive(Clone, PartialEq, Debug)]
pub struct LiveLoad {
    /// Occupancy class → selects `base_psf`.
    pub occupancy: LiveOccupancy,
    /// Unreduced uniform live load (psf).
    pub base_psf: f64,
    /// Discriminates `Lr` (roof live) from `L` (floor live) — they enter different combinations.
    pub is_roof_live: bool,
    /// Live-load element factor `KLL` (ASCE 7 Table 4.7-1) — an input to the reduction.
    pub element_factor_kll: Option<f64>,
    /// Derived reduction per ASCE 7 4.7 (1.0 when not permitted).
    pub reduction_factor: Option<f64>,
    /// Derived effective live load fed to combinations.
    pub reduced_psf: Option<f64>,
    /// Provenance (IRC R301.5 / ASCE 7 4.7).
    pub source_ref: Option<CitationKey>,
}

impl LiveLoad {
    /// ASCE 7 §4.7 reduction factor for a tributary area `at_ft2`: `0.25 + 15/√(KLL·AT)`, applied
    /// only where `KLL·AT ≥ 400 ft²` and floored at 0.50 (members supporting one floor). Returns
    /// 1.0 (no reduction) otherwise.
    pub fn reduction_for(&self, at_ft2: f64) -> f64 {
        let kll = self.element_factor_kll.unwrap_or(1.0);
        let kll_at = kll * at_ft2;
        if self.is_roof_live || kll_at < 400.0 {
            return 1.0;
        }
        (0.25 + 15.0 / kll_at.sqrt()).clamp(0.5, 1.0)
    }

    /// The reduced design live load (psf) for a tributary area — `base_psf × reduction_for(at)`.
    pub fn reduced_psf_for(&self, at_ft2: f64) -> f64 {
        self.base_psf * self.reduction_for(at_ft2)
    }
}

/// Roof snow load `S` per ASCE 7 Ch. 7. Material-blind; reused across roof members.
#[derive(Clone, PartialEq, Debug)]
pub struct SnowLoad {
    /// Ground snow `pg` (psf) — the site input.
    pub ground_snow_pg: f64,
    /// Exposure factor `Ce`.
    pub exposure_ce: Option<f64>,
    /// Thermal factor `Ct`.
    pub thermal_ct: Option<f64>,
    /// Snow importance factor `Is`.
    pub importance_is: Option<f64>,
    /// Roof-slope reduction `Cs`.
    pub slope_cs: Option<f64>,
    /// Derived flat-roof snow `pf`.
    pub flat_roof_pf: Option<f64>,
    /// Derived sloped/drifted design snow fed to combinations.
    pub design_snow_psf: Option<f64>,
    /// Provenance (ASCE 7 Ch. 7).
    pub source_ref: Option<CitationKey>,
}

impl SnowLoad {
    /// Flat-roof snow `pf = 0.7·Ce·Ct·Is·pg` (factors default to 1.0 when unset).
    pub fn flat_roof(&self) -> f64 {
        let ce = self.exposure_ce.unwrap_or(1.0);
        let ct = self.thermal_ct.unwrap_or(1.0);
        is_factor(self.importance_is) * 0.7 * ce * ct * self.ground_snow_pg
    }

    /// Sloped design snow `ps = Cs·pf` (`Cs` defaults to 1.0).
    pub fn design_snow(&self) -> f64 {
        self.slope_cs.unwrap_or(1.0) * self.flat_roof()
    }
}

fn is_factor(is: Option<f64>) -> f64 {
    is.unwrap_or(1.0)
}

/// ASCE 7 wind exposure category.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WindExposure {
    B,
    C,
    D,
}

/// Which wind procedure produced the pressures.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WindMethod {
    /// ASCE 7 Ch. 27 directional.
    Directional,
    /// ASCE 7 Ch. 28 envelope (low-rise).
    Envelope,
    /// WFCM prescriptive alternative (lets IRC houses skip the full calc).
    WfcmPrescriptive,
}

/// Wind pressure `W` per ASCE 7. Carries both the MWFRS lateral demand and the C&C uplift so a
/// combination can build both gravity and net-uplift cases. Material-blind.
#[derive(Clone, PartialEq, Debug)]
pub struct WindLoad {
    /// Ultimate design wind speed `V` (mph) — site input.
    pub basic_speed_v: f64,
    /// Exposure category.
    pub exposure_category: WindExposure,
    /// Derived velocity pressure `qz`.
    pub velocity_pressure_qz: Option<f64>,
    /// MWFRS (lateral) pressure → load-path lateral demand.
    pub mwfrs_pressure_psf: Option<f64>,
    /// Components & cladding uplift → reverses the gravity sign in uplift combos.
    pub cc_uplift_psf: Option<f64>,
    /// Wind direction (lateral, not gravity) — a unitless direction.
    pub direction: UnitVec3,
    /// Procedure used.
    pub method: Option<WindMethod>,
    /// Provenance (ASCE 7 Ch. 26–28 / WFCM).
    pub source_ref: Option<CitationKey>,
}

impl WindLoad {
    /// Velocity pressure `qz = 0.00256·Kz·Kzt·Kd·Ke·V²` (psf). The `K` coefficients are passed
    /// in (site/height dependent); this is the pure ASCE 7 26.10 form.
    pub fn velocity_pressure(&self, kz: f64, kzt: f64, kd: f64, ke: f64) -> f64 {
        0.00256 * kz * kzt * kd * ke * self.basic_speed_v * self.basic_speed_v
    }
}

/// Seismic base shear `E` per ASCE 7 Ch. 11–12 ELF (`V = Cs·W`). Material-blind (`R` is a
/// lateral-system property, not a material constant).
#[derive(Clone, PartialEq, Debug)]
pub struct SeismicLoad {
    /// Design spectral acceleration, short period (`SDS`) — site input.
    pub sds: f64,
    /// Design spectral acceleration, 1-sec period (`SD1`).
    pub sd1: Option<f64>,
    /// Response modification coefficient `R` for the lateral system.
    pub response_r: f64,
    /// Seismic importance factor `Ie`.
    pub importance_ie: Option<f64>,
    /// Effective seismic weight `W` (lb).
    pub seismic_weight: Option<f64>,
    /// Derived base shear `V`.
    pub base_shear_v: Option<f64>,
    /// Lateral seismic direction — a unitless direction.
    pub direction: UnitVec3,
    /// Provenance (ASCE 7 Ch. 11–12).
    pub source_ref: Option<CitationKey>,
}

impl SeismicLoad {
    /// Seismic response coefficient `Cs = SDS / (R/Ie)` (`Ie` defaults to 1.0).
    pub fn cs(&self) -> f64 {
        let ie = self.importance_ie.unwrap_or(1.0);
        self.sds / (self.response_r / ie)
    }

    /// Base shear `V = Cs·W` for the supplied effective seismic weight (lb).
    pub fn base_shear(&self, seismic_weight: f64) -> f64 {
        self.cs() * seismic_weight
    }
}

/// The concrete source value object behind a [`LoadSource`].
#[derive(Clone, PartialEq, Debug)]
pub enum LoadSourcePayload {
    Dead(DeadLoad),
    Live(LiveLoad),
    Snow(SnowLoad),
    Wind(WindLoad),
    Seismic(SeismicLoad),
}

/// A discriminated source: its [`Effect`] (which path it follows) plus the concrete payload.
/// Lets the rollup, combination and solver pass a heterogeneous source set without the structural
/// seam knowing the concrete subtype — the extensibility seam for new ASCE 7 sources.
#[derive(Clone, PartialEq, Debug)]
pub struct LoadSource {
    /// Gravity / lateral / uplift routing.
    pub effect: Effect,
    /// The concrete source.
    pub payload: LoadSourcePayload,
}

impl LoadSource {
    /// The ASCE 7 symbol for this source (roof live is distinguished from floor live).
    pub fn kind(&self) -> SourceKind {
        match &self.payload {
            LoadSourcePayload::Dead(_) => SourceKind::Dead,
            LoadSourcePayload::Live(l) if l.is_roof_live => SourceKind::RoofLive,
            LoadSourcePayload::Live(_) => SourceKind::Live,
            LoadSourcePayload::Snow(_) => SourceKind::Snow,
            LoadSourcePayload::Wind(_) => SourceKind::Wind,
            LoadSourcePayload::Seismic(_) => SourceKind::Seismic,
        }
    }

    /// A gravity dead-load source.
    pub fn dead(load: DeadLoad) -> LoadSource {
        LoadSource {
            effect: Effect::Gravity,
            payload: LoadSourcePayload::Dead(load),
        }
    }

    /// A gravity live-load source.
    pub fn live(load: LiveLoad) -> LoadSource {
        LoadSource {
            effect: Effect::Gravity,
            payload: LoadSourcePayload::Live(load),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_load_reduction_floors_at_half() {
        let l = LiveLoad {
            occupancy: LiveOccupancy::LivingArea,
            base_psf: 40.0,
            is_roof_live: false,
            element_factor_kll: Some(2.0),
            reduction_factor: None,
            reduced_psf: None,
            source_ref: None,
        };
        // Small tributary: no reduction.
        assert_eq!(l.reduction_for(100.0), 1.0);
        // Large tributary: reduced but never below 0.5.
        let big = l.reduction_for(10_000.0);
        assert!((0.5..1.0).contains(&big));
        assert!((l.reduced_psf_for(100.0) - 40.0).abs() < 1e-9);
    }

    #[test]
    fn snow_and_seismic_formulas() {
        let s = SnowLoad {
            ground_snow_pg: 30.0,
            exposure_ce: Some(1.0),
            thermal_ct: Some(1.1),
            importance_is: Some(1.0),
            slope_cs: Some(0.9),
            flat_roof_pf: None,
            design_snow_psf: None,
            source_ref: None,
        };
        // pf = 0.7*1.0*1.1*1.0*30 = 23.1 ; ps = 0.9*23.1
        assert!((s.flat_roof() - 23.1).abs() < 1e-9);
        assert!((s.design_snow() - 0.9 * 23.1).abs() < 1e-9);

        let e = SeismicLoad {
            sds: 1.0,
            sd1: None,
            response_r: 6.5,
            importance_ie: Some(1.0),
            seismic_weight: Some(10_000.0),
            base_shear_v: None,
            direction: UnitVec3::X,
            source_ref: None,
        };
        // Cs = 1.0/(6.5/1.0) ; V = Cs*W
        assert!((e.cs() - 1.0 / 6.5).abs() < 1e-9);
        assert!((e.base_shear(10_000.0) - (1.0 / 6.5) * 10_000.0).abs() < 1e-6);
    }

    #[test]
    fn source_kind_distinguishes_roof_live() {
        let floor = LoadSource::live(LiveLoad {
            occupancy: LiveOccupancy::LivingArea,
            base_psf: 40.0,
            is_roof_live: false,
            element_factor_kll: None,
            reduction_factor: None,
            reduced_psf: None,
            source_ref: None,
        });
        assert_eq!(floor.kind(), SourceKind::Live);
        assert_eq!(floor.effect, Effect::Gravity);
    }
}
