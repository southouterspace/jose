//! [`SizingQuery`] (the request to the arbiter) and [`SizingResult`] (its output).
//!
//! `SizingQuery` consumes the loads layer's `MemberDemand` by value, keeping loads out of this
//! layer; `SizingResult.sizedSpec` is the deliberate seam to the estimating layer.

use crate::domain::factors::FactorContext;
use crate::domain::limit_state::{LimitStateCheck, LimitStateId};
use crate::domain::philosophy::DesignPhilosophy;
use building::MemberPlacementId;
use loads_analysis::MemberDemand;
use materials::{SectionProperties, SpecKey};

/// The request to size a member: its demand + a candidate section + the selected standard's
/// context. Loads are consumed from the loads layer, never modeled here.
#[derive(Clone, PartialEq, Debug)]
pub struct SizingQuery {
    /// Which placed member is being sized.
    pub member_id: MemberPlacementId,
    /// Per-member `{axial, moment, shear}` demand from the loads layer.
    pub demand: MemberDemand,
    /// The trial section/spec (the arbiter checks if it passes; may iterate to the next size).
    pub candidate_spec: SpecKey,
    /// Clear span in **real inches**, converted from the member's tick length at the call boundary.
    pub span_in: f64,
    /// The candidate's gross section geometry (the strategy reduces it per basis).
    pub section: SectionProperties,
    /// Contextual inputs for the factor stack.
    pub context: FactorContext,
}

/// Which path produced a result.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SizingMethod {
    /// BeamStatics + limit states.
    ClosedForm,
    /// A prescriptive table entry covered the query.
    Prescriptive,
}

/// Whether the query stayed inside the design envelope.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Escape {
    /// Covered by prescriptive or closed-form design.
    Ok,
    /// Outside both — needs an engineer.
    EngineeredDesign,
}

/// The arbiter's output — pass/fail with governing utilization, the full check set, and the sized
/// section. The hand-off to downstream estimating (no cost types leak in).
#[derive(Clone, PartialEq, Debug)]
pub struct SizingResult {
    /// True iff every check passes (or a prescriptive entry covers the query).
    pub pass: bool,
    /// The check id with max utilization — the binding constraint.
    pub governing_check: LimitStateId,
    /// `max(ratio)` across checks, unitless.
    pub utilization: f64,
    /// Full demand/capacity record per mode (core + strategy).
    pub checks: Vec<LimitStateCheck>,
    /// The spec that passed (= candidate, or the next size up). Feeds estimating downstream.
    pub sized_spec: SpecKey,
    /// Which path produced the result.
    pub method: Option<SizingMethod>,
    /// Which ASD/LRFD basis produced this result.
    pub philosophy_used: DesignPhilosophy,
    /// Set when out of both prescriptive and closed-form envelope.
    pub escape: Option<Escape>,
}
