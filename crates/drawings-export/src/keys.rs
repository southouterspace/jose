//! Opaque ids for the drawings-export layer's two entities ([`Sheet`](crate::Sheet) and
//! [`DrawingSet`](crate::DrawingSet)) and the project a [`TitleBlock`](crate::TitleBlock) cites.

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
    /// Identity of a [`Sheet`](crate::Sheet).
    SheetId
);
uuid_id!(
    /// Identity of a [`DrawingSet`](crate::DrawingSet) — the deliverable artifact.
    DrawingSetId
);
uuid_id!(
    /// Handle to the project a [`TitleBlock`](crate::TitleBlock) belongs to (project-context layer).
    ProjectRef
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_expose_their_raw_handle() {
        assert_eq!(SheetId(3).raw(), 3);
        assert_eq!(DrawingSetId(4).raw(), 4);
    }
}
