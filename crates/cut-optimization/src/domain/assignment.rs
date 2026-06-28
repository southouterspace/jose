//! [`CutAssignment`] — one stock stick's plan, and the [`CutLine`] cuts taken from it.
//!
//! The provenance row of the cut layer: this stick (bought or reused) → these cuts → this leftover
//! → reused or scrapped. Exactly one of `stock_ref` / `source_offcut_ref` is set — the
//! buy-vs-reuse discriminator that lets the estimating layer count only real purchases and treat
//! reuses as zero marginal cost. Stock length is read *through* the ref, never copied here.

use crate::keys::{AssignmentId, DemandLineKey, OffcutId};
use geometry_kernel::Tick;
use materials::SkuKey;

/// A tiny cut within an assignment: which demand line it fulfills, at what length. No identity, no
/// cross-schema reuse, so it is kept inline rather than promoted to a top-level type.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CutLine {
    /// → [`Demand::line_key`](crate::Demand::line_key) this cut fulfills.
    pub demand_ref: DemandLineKey,
    /// Cut length in ticks.
    pub length: Tick,
}

/// The fate of a stick's final remainder.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RemainderFate {
    /// ≥ `min_reusable` (or fits a short demand): re-enters the pool as a new [`Offcut`].
    Pooled,
    /// Scrap.
    Waste,
}

/// Where the stick came from — the buy-vs-reuse discriminator. Exactly one variant per assignment.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum StickSource {
    /// A newly-bought stick, via the chosen [`StockOption`](crate::StockOption).
    Bought(SkuKey),
    /// An [`Offcut`](crate::Offcut) consumed instead of new stock — zero marginal cost.
    Reused(OffcutId),
}

// SkuKey is not `Copy`, so the discriminator carries an owned key; keep `StickSource` Clone.
impl StickSource {
    /// The bought SKU, if this stick was purchased.
    pub fn bought_sku(&self) -> Option<&SkuKey> {
        match self {
            StickSource::Bought(sku) => Some(sku),
            StickSource::Reused(_) => None,
        }
    }

    /// Whether this stick is a zero-cost offcut reuse.
    pub fn is_reuse(&self) -> bool {
        matches!(self, StickSource::Reused(_))
    }
}

/// One stock stick's plan: ordered cuts, kerf between them, and the fate of the remainder.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CutAssignment {
    /// Stable id; referenced by [`Offcut::parent_assignment_ref`](crate::Offcut::parent_assignment_ref)
    /// and [`PieceProvenance::produced_by`](crate::PieceProvenance::produced_by).
    pub id: AssignmentId,
    /// Bought stick or reused offcut — exactly one. The buy-vs-reuse discriminator.
    pub source: StickSource,
    /// Ordered cuts; order matters for kerf/remainder.
    pub cuts: Vec<CutLine>,
    /// Σ kerf over interior cuts + end trim. Derived; stored for audit.
    pub kerf_total: Tick,
    /// stock length − Σ cut lengths − kerf_total. The leftover, in ticks.
    pub remainder: Tick,
    /// Pooled (re-enters the pool) or waste (scrap).
    pub remainder_fate: RemainderFate,
}

impl CutAssignment {
    /// Σ of all cut lengths on this stick, in ticks.
    pub fn cut_length_total(&self) -> Tick {
        Tick(self.cuts.iter().map(|c| c.length.raw()).sum())
    }

    /// Utilization = Σ cut lengths / stock length. A ratio (renamed from `yield`, a JS reserved
    /// word) for the waste report — never a stored linear quantity. `None` for a zero-length stick.
    pub fn utilization(&self, stock_length: Tick) -> Option<f64> {
        if stock_length.raw() == 0 {
            return None;
        }
        Some(self.cut_length_total().to_inches() / stock_length.to_inches())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utilization_is_cuts_over_stock() {
        let a = CutAssignment {
            id: AssignmentId(1),
            source: StickSource::Bought(SkuKey::from("HD-2x4-8")),
            cuts: vec![
                CutLine {
                    demand_ref: DemandLineKey(1),
                    length: Tick(1000),
                },
                CutLine {
                    demand_ref: DemandLineKey(2),
                    length: Tick(1000),
                },
            ],
            kerf_total: Tick(8),
            remainder: Tick(560),
            remainder_fate: RemainderFate::Pooled,
        };
        assert_eq!(a.cut_length_total(), Tick(2000));
        let u = a.utilization(Tick(2568)).unwrap();
        assert!((u - 2000.0 / 2568.0).abs() < 1e-9);
        assert!(a.source.bought_sku().is_some());
        assert!(!a.source.is_reuse());
    }
}
