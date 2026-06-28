//! [`Demand`] — one required cut, and the end-cut geometry that trims it.
//!
//! The COMPLETE `Demand[]` is built before cutting begins; greedy as-you-go packing loses to
//! whole-picture optimization. A demand is an immutable value line: its `line_key` is
//! content-addressable, its real identity lives on the source [`MemberPlacementId`].

use crate::keys::DemandLineKey;
use building::MemberPlacementId;
use geometry_kernel::Tick;
use materials::SpecKey;

/// The cut role — selects min-reusable rules and groups the takeoff. An open vocabulary
/// (the schema lists `stud | plate | header | cripple | sill | block | …`), so it is a string
/// key, not a closed enum: a new framing role is data, never a code edit.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct CutRole(pub String);

impl CutRole {
    /// Borrow the role as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for CutRole {
    fn from(s: &str) -> Self {
        CutRole(s.to_owned())
    }
}

/// The cut geometry at a member's ends. Affects effective length / saw waste at the cut, not the
/// buy decision. Defaults to [`EndCut::Square`].
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum EndCut {
    /// A square crosscut — the default.
    #[default]
    Square,
    /// An angled cut in plane (rake walls, etc).
    Miter,
    /// An angled cut through thickness.
    Bevel,
    /// The seat + plumb notch where a rafter crosses a plate.
    Birdsmouth,
}

/// One required cut: a final length the framing needs, with multiplicity, role, spec, and
/// provenance back to the member that demanded it. Immutable value.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Demand {
    /// Stable content line-key; downstream cuts/pieces cite this exact line.
    pub line_key: DemandLineKey,
    /// Final cut length in 1/32in ticks (NOT inches — 92.625in = 2964 ticks).
    pub length: Tick,
    /// How many identical cuts of this length+role+spec are needed.
    pub qty: u32,
    /// `stud | plate | header | …` — selects min-reusable rules and groups the takeoff.
    pub role: CutRole,
    /// → [`materials::StockSpec`] flyweight (material-agnostic). A fulfilling stock must satisfy it.
    pub spec_ref: SpecKey,
    /// → the placement-layer member instance this cut serves. Where the demand's real identity lives.
    pub source_placement: MemberPlacementId,
    /// End-cut geometry; defaults to square.
    pub end_cut: EndCut,
}

impl Demand {
    /// A square-cut demand of `qty` identical sticks at `length` for `role`/`spec`.
    pub fn new(
        line_key: DemandLineKey,
        length: Tick,
        qty: u32,
        role: CutRole,
        spec_ref: SpecKey,
        source_placement: MemberPlacementId,
    ) -> Demand {
        Demand {
            line_key,
            length,
            qty,
            role,
            spec_ref,
            source_placement,
            end_cut: EndCut::Square,
        }
    }

    /// Total linear length this line demands across its multiplicity, in ticks.
    pub fn total_length(&self) -> Tick {
        Tick(self.length.raw() * self.qty as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_length_multiplies_by_qty() {
        let d = Demand::new(
            DemandLineKey(1),
            Tick(2964),
            4,
            CutRole::from("stud"),
            SpecKey::from("SPF-STUD"),
            MemberPlacementId(1),
        );
        assert_eq!(d.total_length(), Tick(2964 * 4));
        assert_eq!(d.end_cut, EndCut::Square);
    }
}
