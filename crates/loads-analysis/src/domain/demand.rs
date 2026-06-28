//! [`MemberDemand`] — the layer's only downstream product — plus [`AccumulatedDemand`], the
//! rollup's internal unfactored record.
//!
//! `MemberDemand` carries demand only (no capacity, no pass/fail) in real engineering units; the
//! strategy seam compares it against material capacity. `memberRole` + the deflection ratio/limit
//! are the contextual cues the seam needs to pick limit states without this layer knowing any
//! material capacity.

use crate::domain::sources::SourceKind;
use building::MemberPlacementId;
use geometry_kernel::{Tick, TickVec3};

/// Which limit-state set the strategy applies to this member.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MemberRole {
    /// Axial compression (stud-as-column) → buckling.
    Column,
    /// Bending (header/joist-as-beam) → bending + deflection.
    Beam,
    /// Bearing/crush.
    Bearing,
    /// Axial tension.
    TensionTie,
    /// Lateral brace.
    Brace,
}

/// Per-member, per-source **unfactored** accumulated load — the rollup's internal record. Reaction
/// at each node feeds the next node down the load path.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct AccumulatedDemand {
    pub member_ref: MemberPlacementId,
    /// Which source produced this contribution.
    pub source_kind: SourceKind,
    /// Axial demand (lb).
    pub axial: f64,
    /// Distributed line load (plf).
    pub line_load: f64,
    /// Max bending moment (lb·in).
    pub moment: f64,
    /// Max shear (lb).
    pub shear: f64,
    /// Support reaction delivered downstream (lb).
    pub reaction: f64,
}

/// Per-member **factored** structural demand under the governing combination. The clean hand-off
/// to the structural seam — `{axial, moment, shear, deflection}` plus the serviceability limit and
/// the role that selects which limit states apply. Material-blind real units.
#[derive(Clone, PartialEq, Debug)]
pub struct MemberDemand {
    /// The member this demand sizes — also the join key downstream estimating traces quantity by.
    pub member_ref: MemberPlacementId,
    /// Selects which limit-state set the seam applies.
    pub member_role: MemberRole,
    /// Which combination produced this demand (chosen by the strategy, recorded here).
    pub governing_combo: String,
    /// Factored axial demand (compression +, tension −), lb.
    pub axial: f64,
    /// Factored max bending moment, lb·in.
    pub moment: f64,
    /// Factored max shear, lb.
    pub shear: f64,
    /// Service-load deflection magnitude, real in (E injected by the strategy).
    pub deflection: Option<f64>,
    /// Derived serviceability limit `span_in / ratio`, real in.
    pub deflection_limit: f64,
    /// The serviceability denominator (360 live, 240 total) — the contextual cause.
    pub deflection_ratio: Option<i32>,
    /// The governing factored line load `w` BeamStatics integrates — passed through for the seam.
    pub uniform_load: Option<f64>,
    /// Effective span for statics, ticks (linear). The seam divides by 32 for inches.
    pub span: Tick,
    /// Lateral-unbraced length for CL/CP, ticks (linear).
    pub unbraced_length: Option<Tick>,
    /// Support reaction delivered to the next member down the load path, lb.
    pub reaction: Option<f64>,
    /// Resultant application point in world ticks (a committed position).
    pub applied_at: Option<TickVec3>,
}

impl MemberDemand {
    /// The serviceability deflection limit in real inches for a tick span and a ratio denominator
    /// (e.g. 360 → L/360). Derived from `span/32`, reported real — no tick ambiguity.
    pub fn deflection_limit_for(span: Tick, ratio: i32) -> f64 {
        if ratio == 0 {
            return f64::INFINITY;
        }
        span.to_inches() / ratio as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deflection_limit_is_real_inches() {
        // 12ft span (4608t = 144in), L/360 → 0.4in.
        assert!((MemberDemand::deflection_limit_for(Tick(4608), 360) - 0.4).abs() < 1e-9);
        // L/240 → 0.6in.
        assert!((MemberDemand::deflection_limit_for(Tick(4608), 240) - 0.6).abs() < 1e-9);
    }
}
