//! Open registry keys and entity ids for the estimating layer.
//!
//! String-backed keys follow the schema's "open registry keys over closed enums" rule so new
//! cost codes, unit rates, RSMeans assemblies, and units of measure are catalog data, never code
//! edits. Entity ids are opaque `u128` (UUID-shaped) handles.

/// Define a thin newtype over `String` — an open registry / flyweight key.
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
    /// Unit-of-measure code, e.g. `EA | LF | BF | SF | CY | LB | HR | LS`. Open registry keyed by
    /// dimension — a new unit is data.
    UomKey
);
string_key!(
    /// Flyweight key into the [`CostCode`](crate::CostCode) catalog, e.g. `MF-06-11-00`.
    CostCodeKey
);
string_key!(
    /// Flyweight key into the [`ResourceRate`](crate::ResourceRate) catalog, e.g. `LAB-CARP-JOUR`.
    RateKey
);
string_key!(
    /// Flyweight key into the [`AssemblyCost`](crate::AssemblyCost) catalog, e.g.
    /// `RSM-061110-STUDWALL-2x4-16OC`.
    AssemblyKey
);

uuid_id!(
    /// Identity of an [`Estimate`](crate::Estimate) — survives revisions.
    EstimateId
);
uuid_id!(
    /// Identity of a [`TakeoffItem`](crate::TakeoffItem) — the traceability atom.
    TakeoffId
);
uuid_id!(
    /// Identity of a [`MaterialLine`](crate::MaterialLine).
    MaterialLineId
);
uuid_id!(
    /// Identity of a [`ResourceLine`](crate::ResourceLine).
    ResourceLineId
);
uuid_id!(
    /// Identity of a [`Markup`](crate::Markup).
    MarkupId
);
uuid_id!(
    /// Identity of an [`Allowance`](crate::Allowance).
    AllowanceId
);
uuid_id!(
    /// Identity of a [`ChangeOrder`](crate::ChangeOrder).
    ChangeOrderId
);
uuid_id!(
    /// Identity of a [`PayItem`](crate::PayItem).
    PayItemId
);
uuid_id!(
    /// Handle to the project/model snapshot an [`Estimate`](crate::Estimate) prices.
    ProjectRef
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_and_ids_round_trip() {
        assert_eq!(UomKey::from("LF").as_str(), "LF");
        assert_eq!(CostCodeKey::from("MF-06-11-00").as_str(), "MF-06-11-00");
        assert_eq!(EstimateId(5).raw(), 5);
    }
}
