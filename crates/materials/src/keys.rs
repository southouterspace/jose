//! Opaque keys and entity ids for the materials layer.
//!
//! String-backed keys follow the schema's "open registry keys over closed enums" rule so new
//! materials/suppliers/profiles are catalog data, never code edits. Entity ids are opaque
//! `u128` handles (UUID-shaped) that provenance and cross-context refs point at by value.

/// Define a thin newtype over `String` — an open registry key.
macro_rules! string_key {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        pub struct $name(pub String);

        impl $name {
            /// Borrow the key as a string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                $name(s.to_owned())
            }
        }
        impl From<String> for $name {
            fn from(s: String) -> Self {
                $name(s)
            }
        }
    };
}

/// Define a thin newtype over `u128` — an opaque, stable entity id (UUID-shaped).
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

string_key!(
    /// Flyweight key into the [`StockSpec`](crate::StockSpec) catalog, e.g. `SPF-STUD-SDRY`
    /// (wood) or `362S162-33-50` (cold-formed steel). Material-neutral.
    SpecKey
);
string_key!(
    /// Flyweight key into the [`SupplierSku`](crate::SupplierSku) catalog, e.g. `HD-2x4-8-SPF`.
    SkuKey
);
string_key!(
    /// Key into the render-mesh catalog (lives in the render adapter, Phase 4). Resolves the
    /// schema's dangling `MeshRef`.
    MeshKey
);
string_key!(
    /// Open profile key (`rectangular` | `I` | `C` | `HSS` | `round`, extensible). Lets
    /// non-wood profiles extend [`Dimensions`](crate::Dimensions) without a new type.
    ProfileKey
);
string_key!(
    /// Open key naming a connection topology (`stud-to-plate` | `stud-to-stud` |
    /// `header-to-king`, extensible per material). Carried on a
    /// [`ConnectionPoint`](crate::ConnectionPoint).
    ConnectionTypeKey
);
string_key!(
    /// Open key naming a fastening method (`end-nail` | `toe-nail` | `screw` | `weld` | `bolt`,
    /// extensible). Resolved to capacity behind the `DesignStandard` connection-capacity seam.
    ConnectionMethodKey
);

uuid_id!(
    /// Stable identity of a [`Stock`](crate::Stock) — the provenance root for every
    /// [`Piece`](crate::Piece) cut from it.
    StockId
);
uuid_id!(
    /// Stable identity of a [`Piece`](crate::Piece).
    PieceId
);
uuid_id!(
    /// Stable identity of a [`Cut`](crate::Cut) — the handle `Piece::cut_ids` points at.
    CutId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_and_ids_construct() {
        assert_eq!(SpecKey::from("SPF-STUD-SDRY").as_str(), "SPF-STUD-SDRY");
        assert_eq!(StockId(7).raw(), 7);
        assert_eq!(
            SkuKey::from("HD-2x4-8".to_owned()),
            SkuKey("HD-2x4-8".into())
        );
    }
}
