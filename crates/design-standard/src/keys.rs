//! Opaque key aliases for the design-standard seam.

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
    };
}

string_key!(
    /// Selects the strategy leaf, e.g. `NDS-2018`, `AISI-S100-16`.
    DesignStandardId
);
string_key!(
    /// Opaque key into the `reference-data` `MechanicalProperties` flyweight (the single canonical
    /// wood design-value source). Null for materials whose values are code constants.
    MechanicalPropertiesKey
);
string_key!(
    /// Identity of a [`ConnectionGraph`](crate::ConnectionGraph); persists across recompute.
    ConnectionGraphId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_construct() {
        assert_eq!(DesignStandardId::from("NDS-2018").as_str(), "NDS-2018");
    }
}
