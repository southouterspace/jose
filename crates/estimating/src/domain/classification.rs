//! The classification axes: [`CostType`] (what *kind* of dollar), [`CostCode`] (*where* the dollar
//! lands in the WBS), and [`UnitOfMeasure`] (the single seam between the int-tick geometry world
//! and the real-decimal cost world).

use crate::keys::{CostCodeKey, UomKey};
use geometry_kernel::TICKS_PER_FOOT;

/// The five canonical economic categories every cost line resolves to. Material-blind, closed enum
/// — the axis along which an estimate is summarized and subcontractor scope is carved out.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CostKind {
    /// Material — sticks, sheets, concrete, hardware.
    Material,
    /// Labor — crew time.
    Labor,
    /// Equipment — tools, machines.
    Equipment,
    /// Subcontract — scope carved out to a sub.
    Subcontract,
    /// Overhead — indirect cost.
    Overhead,
}

/// A value enum, not a flyweight — identity-free and intrinsically tiny. [`CostCode`] partitions
/// WHERE a dollar lands (WBS); `CostType` partitions WHAT KIND of dollar it is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CostType {
    /// The economic category.
    pub kind: CostKind,
    /// Whether this category is eligible to carry markup (subcontract often gets less than
    /// self-perform material+labor).
    pub is_markup_base: bool,
}

impl CostType {
    /// A markup-eligible cost type of the given kind.
    pub const fn markup_base(kind: CostKind) -> CostType {
        CostType {
            kind,
            is_markup_base: true,
        }
    }
}

/// How a quantity's dimension aggregates and which derived-real rule applies.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dimension {
    /// `EA` — a count.
    Count,
    /// `LF` — linear length (the only tick-convertible dimension).
    Length,
    /// `SF | SY` — area.
    Area,
    /// `CY` — volume.
    Volume,
    /// `LB | TON` — weight.
    Weight,
    /// `HR | DAY` — time.
    Time,
    /// `LS` — lump sum.
    LumpSum,
}

/// The unit a quantity is expressed and priced in — the SINGLE seam between the int-tick geometry
/// world and the real-decimal cost world. It carries the tick conversion ONLY for `Length`; every
/// other dimension is derived-real and carries no tick exponent, which is what guarantees no field
/// is ever typed tick².
#[derive(Clone, PartialEq, Debug)]
pub struct UnitOfMeasure {
    /// `EA | LF | BF | SF | CY | LB | HR | LS | …`.
    pub code: UomKey,
    /// How the rollup aggregates this unit.
    pub dimension: Dimension,
    /// Conversion from canonical int ticks to this unit, ONLY for `Length` (LF = 384 ticks/ft).
    /// `None` for every non-linear dimension.
    pub ticks_per_unit: Option<f64>,
}

impl UnitOfMeasure {
    /// Linear feet — the canonical tick-convertible unit (384 ticks per foot).
    pub fn linear_feet() -> UnitOfMeasure {
        UnitOfMeasure {
            code: UomKey::from("LF"),
            dimension: Dimension::Length,
            ticks_per_unit: Some(TICKS_PER_FOOT as f64),
        }
    }

    /// Each — a count unit.
    pub fn each() -> UnitOfMeasure {
        UnitOfMeasure {
            code: UomKey::from("EA"),
            dimension: Dimension::Count,
            ticks_per_unit: None,
        }
    }

    /// Board feet — a derived-real volume-ish measure with no tick exponent.
    pub fn board_feet() -> UnitOfMeasure {
        UnitOfMeasure {
            code: UomKey::from("BF"),
            dimension: Dimension::Volume,
            ticks_per_unit: None,
        }
    }

    /// An hour of resource time.
    pub fn hour() -> UnitOfMeasure {
        UnitOfMeasure {
            code: UomKey::from("HR"),
            dimension: Dimension::Time,
            ticks_per_unit: None,
        }
    }

    /// Convert a linear tick magnitude into this unit's real quantity. The tick→real seam happens
    /// here exactly once; returns `None` for non-linear units (which never carry ticks).
    pub fn from_ticks(&self, ticks: i32) -> Option<f64> {
        self.ticks_per_unit.map(|per| ticks as f64 / per)
    }
}

/// The coding system a [`CostCode`] belongs to — lets steel/concrete trades add divisions without
/// new types.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CodeSystem {
    /// CSI MasterFormat (`06 11 00`).
    MasterFormat,
    /// Uniformat (`B1010`).
    Uniformat,
    /// A project-custom WBS.
    Custom,
}

/// A shared flyweight classification node — the WBS coordinate a cost lands on. Looked up by key,
/// never copied per line. The estimate's columnar structure comes from these.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CostCode {
    /// Opaque catalog key, e.g. `MF-06-11-00`.
    pub key: CostCodeKey,
    /// `masterformat | uniformat | custom`.
    pub code_system: CodeSystem,
    /// As-published code, e.g. `06 11 00`.
    pub code: String,
    /// Human label, e.g. `Wood Framing`.
    pub title: String,
    /// Hierarchy edge → Division/section roll-up; `None` at root.
    pub parent_key: Option<CostCodeKey>,
    /// Depth in the WBS (0 = division).
    pub level: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_feet_converts_ticks_once() {
        let lf = UnitOfMeasure::linear_feet();
        // 10ft = 10 * 384 ticks = 3840 ticks → 10.0 LF.
        assert_eq!(lf.from_ticks(3840), Some(10.0));
        // A non-linear unit never converts ticks.
        assert_eq!(UnitOfMeasure::each().from_ticks(3840), None);
    }

    #[test]
    fn cost_type_markup_base() {
        let m = CostType::markup_base(CostKind::Material);
        assert!(m.is_markup_base);
        assert_eq!(m.kind, CostKind::Material);
    }
}
