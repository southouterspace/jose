//! The cut/provenance chain: [`Piece`], [`Cut`], [`WasteRecord`], [`QuantityTakeoff`] and
//! [`ProvenanceLink`].
//!
//! Linear lengths are ticks; the priced/rolled-up measure is the real-unit [`QuantityTakeoff`].

use crate::domain::discriminators::QuantityBasis;
use crate::keys::{CutId, MeshKey, PieceId, StockId};
use geometry_kernel::{Plane, Tick};

/// Whether a [`Piece`] is the wanted product or an offcut.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PieceRole {
    /// The wanted member.
    Product,
    /// A remainder eligible for reuse / waste tracking.
    Offcut,
}

/// A material-blind measured quantity for one [`Piece`] (or [`Stock`](crate::Stock)), in the
/// dimension the [`StockForm`](crate::StockForm) dictates. The atomic unit of the bottom-up
/// estimate. A derived real (lf/bf/ft²/ft³/count) — never ticks².
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct QuantityTakeoff {
    /// Which measure is summed; mirrors `StockForm.takeoffBasis`.
    pub basis: QuantityBasis,
    /// Measured amount in the basis unit (derived real).
    pub quantity: f64,
    /// Whether kerf/offcut waste is already folded in.
    pub waste_included: Option<bool>,
}

impl QuantityTakeoff {
    /// The linear-feet takeoff for a piece of the given tick length.
    pub fn linear_feet(length: Tick) -> QuantityTakeoff {
        QuantityTakeoff {
            basis: QuantityBasis::LinearFeet,
            quantity: length.to_feet(),
            waste_included: Some(false),
        }
    }
}

/// The materials-domain provenance record: how a [`Piece`] traces back to its parent
/// [`Stock`](crate::Stock) through cut history and to its sibling offcuts.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProvenanceLink {
    /// The originating `Stock.id` (provenance root).
    pub parent_member_id: StockId,
    /// Ordered [`Cut`] ids from stock to this piece.
    pub cut_chain: Vec<CutId>,
    /// Other pieces (incl. offcuts) from the same parent — supports reuse/waste accounting.
    pub sibling_piece_ids: Option<Vec<PieceId>>,
    /// The provenance kind, distinguishing this from snap- and citation-provenance.
    pub kind: ProvenanceKind,
}

/// The provenance kind tag — shared vocabulary across the three distinct provenance concepts.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProvenanceKind {
    /// A piece produced by cutting stock.
    MaterialCut,
}

/// A result of cutting [`Stock`](crate::Stock), with a full provenance chain back to its parent.
/// Offcuts are pieces too. Also carries the per-piece quantity takeoff the estimator rolls up.
#[derive(Clone, PartialEq, Debug)]
pub struct Piece {
    /// Stable identity.
    pub id: PieceId,
    /// Provenance root — the original `Stock.id`.
    pub parent_member_id: StockId,
    /// Ordered [`Cut`] ids that produced this piece.
    pub cut_ids: Vec<CutId>,
    /// Product vs offcut — drives waste & reuse tracking and the estimating waste rollup.
    pub role: PieceRole,
    /// Key into the render-mesh catalog (Phase 4 render layer).
    pub geometry_ref: MeshKey,
    /// Resulting clear length after kerf, in ticks (linear stock; sheet/cast use the takeoff).
    pub length: Tick,
    /// Material-blind measured quantity — the bottom-up estimating leaf.
    pub quantity_takeoff: QuantityTakeoff,
    /// Parent + sibling pieces + cut-plane history.
    pub provenance: Option<ProvenanceLink>,
}

/// What a [`Cut`] is applied to: a [`Stock`](crate::Stock) or an already-cut [`Piece`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CutTarget {
    Stock(StockId),
    Piece(PieceId),
}

/// An immutable recorded cut event: a member divided along a [`Plane`] at an angle, consuming
/// kerf, producing traceable [`Piece`]s. The durable *record* of one cut — the *act* lives in
/// the cutting-stock solver.
#[derive(Clone, PartialEq, Debug)]
pub struct Cut {
    /// The provenance handle `Piece.cut_ids` points at.
    pub id: CutId,
    /// Stock or piece being cut.
    pub target_id: CutTarget,
    /// Position + orientation of the cut (a geometry-kernel primitive, referenced not owned).
    pub plane: Plane,
    /// Miter/bevel off-square, in real degrees (an angle, not linear geometry).
    pub angle: f64,
    /// Distance along the member axis, in ticks.
    pub position: Tick,
    /// Blade width removed (4 = 0.125in); material lost, feeds the [`WasteRecord`].
    pub kerf: Option<Tick>,
    /// Order within a multi-cut operation.
    pub sequence: Option<u32>,
    /// Emitted kerf + offcut measure (linear stock only).
    pub waste: Option<WasteRecord>,
}

/// The kerf and offcut produced by one [`Cut`], as a material-blind quantity the estimator nets
/// into the takeoff. Distinguishes recoverable offcut from true (kerf) waste.
#[derive(Clone, PartialEq, Debug)]
pub struct WasteRecord {
    /// Material consumed by the blade, in ticks — unrecoverable.
    pub kerf_length: Tick,
    /// Remainder length in ticks; becomes a role=offcut piece eligible for reuse.
    pub offcut_length: Option<Tick>,
    /// Net unrecoverable waste as a material-blind aggregate for the estimate rollup.
    pub net_waste: Option<QuantityTakeoff>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_feet_takeoff_converts_from_ticks() {
        // 92.625in = 2964 ticks → 7.71875 ft.
        let qt = QuantityTakeoff::linear_feet(Tick(2964));
        assert_eq!(qt.basis, QuantityBasis::LinearFeet);
        assert!((qt.quantity - 2964.0 / 384.0).abs() < 1e-9);
        assert_eq!(qt.waste_included, Some(false));
    }

    #[test]
    fn piece_carries_provenance_chain() {
        let p = Piece {
            id: PieceId(2),
            parent_member_id: StockId(1),
            cut_ids: vec![CutId(10)],
            role: PieceRole::Product,
            geometry_ref: MeshKey::from("mesh-2"),
            length: Tick(2964),
            quantity_takeoff: QuantityTakeoff::linear_feet(Tick(2964)),
            provenance: Some(ProvenanceLink {
                parent_member_id: StockId(1),
                cut_chain: vec![CutId(10)],
                sibling_piece_ids: Some(vec![PieceId(3)]),
                kind: ProvenanceKind::MaterialCut,
            }),
        };
        assert_eq!(p.provenance.unwrap().parent_member_id, StockId(1));
    }

    #[test]
    fn cut_targets_stock_or_piece() {
        let c = Cut {
            id: CutId(10),
            target_id: CutTarget::Stock(StockId(1)),
            plane: Plane::xy(geometry_kernel::TickVec3::ZERO),
            angle: 0.0,
            position: Tick(2964),
            kerf: Some(Tick(4)),
            sequence: Some(0),
            waste: Some(WasteRecord {
                kerf_length: Tick(4),
                offcut_length: Some(Tick(104)),
                net_waste: None,
            }),
        };
        assert_eq!(c.target_id, CutTarget::Stock(StockId(1)));
        assert_eq!(c.waste.unwrap().kerf_length, Tick(4));
    }
}
