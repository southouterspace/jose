//! [`TakeoffItem`] — the traceability atom of bottom-up estimating.
//!
//! One measured quantity tied back to the exact domain object that demanded it. Every dollar in
//! the estimate descends from a takeoff item, so any cost can be drilled to its source geometry.
//! This is what makes the takeoff auditable rather than a flat spreadsheet.

use crate::domain::classification::UnitOfMeasure;
use crate::keys::{CostCodeKey, TakeoffId};

/// What kind of domain object a takeoff measures, and the typed pointer back to it. The
/// `DomainRef` payload is the traceability link into the connection graph + cut list.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DomainRef {
    /// → a `materials::Piece` (by raw id).
    Piece(u128),
    /// → a `building::MemberPlacement` (by raw id).
    MemberPlacement(u128),
    /// → a `design-standard` connection-graph edge (fastener/hardware/prefab connector).
    Connection(u128),
    /// → a `cut-optimization::CutAssignment` (remainder + kerf source).
    CutAssignment(u128),
    /// → a `cut-optimization::Offcut` scrapped as waste.
    OffcutWaste(u128),
    /// Kerf loss attributed to a `CutAssignment`.
    KerfWaste(u128),
    /// An RSMeans-style assembly expansion.
    Assembly(String),
    /// A manually entered quantity (no domain source).
    Manual,
}

impl DomainRef {
    /// Whether this source is paid-for waste (offcut scrap or kerf loss) — lets the rollup separate
    /// installed quantity from waste for value engineering.
    pub fn is_waste(&self) -> bool {
        matches!(self, DomainRef::OffcutWaste(_) | DomainRef::KerfWaste(_))
    }
}

/// One measured quantity traced to its source domain object. Derived-real quantity in the line's
/// UOM, converted from canonical ticks exactly once; the original tick magnitude is retained for
/// round-trip audit on linear takeoffs.
#[derive(Clone, PartialEq, Debug)]
pub struct TakeoffItem {
    /// Stable identity for audit/round-trip.
    pub id: TakeoffId,
    /// Typed pointer to the source domain object.
    pub source: DomainRef,
    /// Measured quantity in the line's UOM (derived-real).
    pub quantity: f64,
    /// Unit of measure; carries the tick conversion for linear dimensions.
    pub uom: UnitOfMeasure,
    /// Original canonical int-tick magnitude for linear takeoffs, retained to prove no precision
    /// was lost. `None` for count/area/volume/weight.
    pub source_ticks: Option<i32>,
    /// True when the source is offcut or kerf waste (mirrors [`DomainRef::is_waste`]).
    pub waste_flag: bool,
    /// → the [`CostCode`](crate::CostCode) this quantity rolls up under.
    pub cost_code_key: CostCodeKey,
    /// Human breadcrumb, e.g. "king studs, gable wall".
    pub note: Option<String>,
}

impl TakeoffItem {
    /// A linear takeoff: convert a tick length to feet through the UOM exactly once, retaining the
    /// source ticks for audit.
    pub fn linear(
        id: TakeoffId,
        source: DomainRef,
        ticks: i32,
        cost_code_key: CostCodeKey,
    ) -> TakeoffItem {
        let uom = UnitOfMeasure::linear_feet();
        let quantity = uom.from_ticks(ticks).unwrap_or(0.0);
        let waste_flag = source.is_waste();
        TakeoffItem {
            id,
            source,
            quantity,
            uom,
            source_ticks: Some(ticks),
            waste_flag,
            cost_code_key,
            note: None,
        }
    }

    /// A count takeoff (EA): a derived-real count, no tick magnitude.
    pub fn count(
        id: TakeoffId,
        source: DomainRef,
        quantity: f64,
        cost_code_key: CostCodeKey,
    ) -> TakeoffItem {
        let waste_flag = source.is_waste();
        TakeoffItem {
            id,
            source,
            quantity,
            uom: UnitOfMeasure::each(),
            source_ticks: None,
            waste_flag,
            cost_code_key,
            note: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_takeoff_converts_and_retains_ticks() {
        let t = TakeoffItem::linear(
            TakeoffId(1),
            DomainRef::MemberPlacement(7),
            3840, // 10ft
            CostCodeKey::from("MF-06-11-00"),
        );
        assert_eq!(t.quantity, 10.0);
        assert_eq!(t.source_ticks, Some(3840));
        assert!(!t.waste_flag);
    }

    #[test]
    fn kerf_waste_sets_the_waste_flag() {
        let t = TakeoffItem::linear(
            TakeoffId(2),
            DomainRef::KerfWaste(9),
            4,
            CostCodeKey::from("MF-06-11-00"),
        );
        assert!(t.waste_flag);
    }
}
