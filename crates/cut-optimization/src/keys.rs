//! Opaque ids and line-keys for the cut layer.
//!
//! Cut artifacts are *immutable values*: a [`Demand`](crate::Demand) is keyed by a content
//! line-key (identity belongs to its source member, not the line); an
//! [`Offcut`](crate::Offcut) and a [`CutAssignment`](crate::CutAssignment) carry stable ids so
//! provenance can back-link them. All are UUID-shaped `u128` handles, matching the materials
//! layer's entity-id idiom.

/// Define a thin newtype over `u128` — an opaque, stable id (UUID-shaped).
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
    /// Content line-key for a [`Demand`](crate::Demand). NOT entity identity — the demand is an
    /// immutable line item; its real identity lives on `sourcePlacement`. Downstream cuts and
    /// pieces cite the exact demand line through this key.
    DemandLineKey
);
uuid_id!(
    /// Stable id of an [`Offcut`](crate::Offcut), so a later assignment can declare it consumed.
    OffcutId
);
uuid_id!(
    /// Stable id of a [`CutAssignment`](crate::CutAssignment) — the stick-plan provenance row.
    AssignmentId
);
uuid_id!(
    /// Identity of a whole [`CutPlan`](crate::CutPlan) solve result (a versionable snapshot).
    CutPlanId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_expose_their_raw_handle() {
        assert_eq!(DemandLineKey(7).raw(), 7);
        assert_eq!(OffcutId(9).raw(), 9);
        assert_eq!(AssignmentId(11), AssignmentId(11));
    }
}
