//! Entity ids and cross-context reference handles for the building layer.
//!
//! References that point *downstream* (to the loads layer, not yet built) are opaque handles
//! defined here rather than imports, so this context stays upstream of loads-analysis — the
//! placement layer captures factor *inputs* as neutral facts but never depends on the load model.

use geometry_kernel::EntityId;

macro_rules! uuid_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        pub struct $name(pub u128);

        impl $name {
            /// The raw 128-bit handle.
            #[inline]
            pub const fn raw(self) -> u128 {
                self.0
            }
        }
    };
}

uuid_id!(
    /// Stable identity of a [`Wall`](crate::Wall).
    WallId
);
uuid_id!(
    /// Stable identity of a [`MemberPlacement`](crate::MemberPlacement).
    MemberPlacementId
);
uuid_id!(
    /// Stable identity of a [`Floor`](crate::Floor).
    FloorId
);
uuid_id!(
    /// Stable identity of a [`Roof`](crate::Roof).
    RoofId
);
uuid_id!(
    /// Stable identity of a [`Sheathing`](crate::Sheathing).
    SheathingId
);
uuid_id!(
    /// Structural handle to a [`Junction`](crate::Junction) (a value object); lets a wall hold a
    /// back-reference to the junctions it participates in.
    JunctionRef
);
uuid_id!(
    /// Handle to a `materials::ConnectionPoint` (a value object in the materials layer), used to
    /// tie a placement/end to a concrete fastener detail without copying it.
    ConnectionPointRef
);
uuid_id!(
    /// **Downstream** handle to a `loads-analysis::MemberDemand`. Opaque on purpose: the
    /// placement layer links to the demand the member carries without depending on the load model
    /// (resolves the draft's invented `loadDurationContext` loose ref — loads stay single-homed).
    MemberDemandRef
);

/// A reference to a `geometry-kernel` [`Face`](geometry_kernel::Face): the volume it belongs to
/// plus the face's stable index. The promotion source a wall/floor/roof is derived from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FaceRef {
    /// The owning solid.
    pub volume: EntityId,
    /// The face's stable index within that solid.
    pub face_index: u32,
}

/// A reference to a [`Wall`](crate::Wall) — its id.
pub type WallRef = WallId;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_and_face_ref_construct() {
        assert_eq!(WallId(3).raw(), 3);
        let f = FaceRef {
            volume: EntityId(1),
            face_index: 2,
        };
        assert_eq!(f.face_index, 2);
    }
}
