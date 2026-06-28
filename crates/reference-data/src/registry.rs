//! The Flyweight pattern: a keyed catalog whose entries are stored once and referenced.

use std::collections::BTreeMap;

/// A value that carries its own catalog key, so a [`Registry`] can index a collection of
/// them automatically.
pub trait Flyweight {
    /// The key type this flyweight is looked up by.
    type Key: Ord + Clone;
    /// This value's key.
    fn flyweight_key(&self) -> Self::Key;
}

/// A keyed catalog of intrinsic, shared values — the Flyweight store. Entries live here
/// once; everything else holds a key and looks them up, never a per-use copy.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Registry<K: Ord + Clone, V> {
    entries: BTreeMap<K, V>,
}

impl<K: Ord + Clone, V> Registry<K, V> {
    /// An empty catalog.
    pub fn new() -> Registry<K, V> {
        Registry {
            entries: BTreeMap::new(),
        }
    }

    /// Insert (or replace) an entry under `key`, returning the previous value if any.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.entries.insert(key, value)
    }

    /// Look up a shared entry by key.
    pub fn get(&self, key: &K) -> Option<&V> {
        self.entries.get(key)
    }

    /// Whether `key` is catalogued.
    pub fn contains(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    /// Number of catalogued entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the catalog is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate `(key, value)` pairs in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter()
    }
}

impl<K: Ord + Clone, V> Default for Registry<K, V> {
    fn default() -> Self {
        Registry::new()
    }
}

impl<V: Flyweight> Registry<V::Key, V> {
    /// Build a catalog from values that key themselves. On a duplicate key the later value
    /// wins.
    pub fn index(values: impl IntoIterator<Item = V>) -> Registry<V::Key, V> {
        let mut reg = Registry::new();
        for v in values {
            let k = v.flyweight_key();
            reg.insert(k, v);
        }
        reg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, PartialEq, Eq, Debug)]
    struct Sku {
        id: String,
        price_cents: u32,
    }
    impl Flyweight for Sku {
        type Key = String;
        fn flyweight_key(&self) -> String {
            self.id.clone()
        }
    }

    #[test]
    fn index_builds_lookup_by_self_key() {
        let reg = Registry::index([
            Sku {
                id: "2x4-8".into(),
                price_cents: 398,
            },
            Sku {
                id: "2x6-8".into(),
                price_cents: 612,
            },
        ]);
        assert_eq!(reg.len(), 2);
        assert_eq!(reg.get(&"2x4-8".to_owned()).unwrap().price_cents, 398);
        assert!(reg.contains(&"2x6-8".to_owned()));
        assert!(!reg.contains(&"nope".to_owned()));
    }

    #[test]
    fn later_duplicate_wins() {
        let reg = Registry::index([
            Sku {
                id: "x".into(),
                price_cents: 1,
            },
            Sku {
                id: "x".into(),
                price_cents: 2,
            },
        ]);
        assert_eq!(reg.len(), 1);
        assert_eq!(reg.get(&"x".to_owned()).unwrap().price_cents, 2);
    }
}
