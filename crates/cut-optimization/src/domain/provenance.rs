//! [`PieceProvenance`] — the cut-layer's per-cut provenance record.
//!
//! Owned HERE, not on [`materials::Piece`]: rather than bloating the materials entity with
//! cut-specific fields, the cut layer holds this thin VO that links one canonical piece to the
//! [`CutAssignment`](crate::CutAssignment) that produced it and the [`Demand`](crate::Demand) line
//! it satisfied. The chain `Stock/Offcut → CutAssignment → PieceProvenance → materials::Piece` is
//! what the estimating layer walks bottom-up to attribute cost back to a member.

use crate::keys::{AssignmentId, DemandLineKey};
use materials::PieceId;

/// Links a canonical [`materials::Piece`] to the assignment that produced it and the demand it
/// fulfilled. Single-sourced — never restates piece fields.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PieceProvenance {
    /// → [`materials::Piece`] — the canonical cut result this record annotates.
    pub piece_ref: PieceId,
    /// → the [`CutAssignment`](crate::CutAssignment) this piece came from (the stick→cut link).
    pub produced_by: AssignmentId,
    /// → the [`Demand`](crate::Demand) line satisfied.
    pub fulfills: DemandLineKey,
    /// True if cut from a reused offcut rather than newly-bought stock — zero marginal material
    /// cost for the estimate.
    pub is_offcut_reuse: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provenance_links_the_chain() {
        let p = PieceProvenance {
            piece_ref: PieceId(42),
            produced_by: AssignmentId(3),
            fulfills: DemandLineKey(7),
            is_offcut_reuse: true,
        };
        assert_eq!(p.piece_ref, PieceId(42));
        assert!(p.is_offcut_reuse);
    }
}
