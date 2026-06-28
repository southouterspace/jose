//! Open registry keys.
//!
//! Each is a thin newtype over `String` rather than a closed enum, so new
//! materials/standards/tables are added as catalog data without touching code.

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

string_key!(
    /// Key resolving a `(material, standard)` family's reference design values.
    DesignValueKey
);
string_key!(
    /// Key naming a design standard family (e.g. `nds`, `aisi-s100`, `aci-318`).
    StandardKey
);
string_key!(
    /// Key naming a directly-implemented code provision.
    ProvisionKey
);
string_key!(
    /// Key naming a licensed prescriptive-table lookup.
    LookupKey
);
string_key!(
    /// Key naming a prescriptive table.
    TableKey
);
string_key!(
    /// Open key naming a stock form (e.g. `dimensional-lumber`, `cfs-c-stud`, `rebar`).
    StockForm
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_construct_and_compare() {
        assert_eq!(StandardKey::from("nds"), StandardKey("nds".to_owned()));
        assert_eq!(DesignValueKey::from("df-l-no2").as_str(), "df-l-no2");
    }
}
