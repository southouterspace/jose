//! Code provisions and prescriptive lookup tables (material-blind).

use crate::citation::CitationKey;
use crate::keys::{LookupKey, ProvisionKey, TableKey};
use crate::registry::Flyweight;
use std::collections::BTreeMap;

/// What kind of rule a [`CodeProvision`] encodes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProvisionKind {
    /// A prescriptive limit or requirement applied directly.
    Prescriptive,
    /// A reference to a licensed lookup table.
    TableLookup,
    /// A definition or scoping rule.
    Definitional,
}

/// A material-blind, tagged container for a single code rule the engine implements directly
/// — a pointer plus provenance.
#[derive(Clone, PartialEq, Debug)]
pub struct CodeProvision {
    pub key: ProvisionKey,
    pub kind: ProvisionKind,
    pub citation: CitationKey,
    /// What the provision applies to (open tag), if scoped.
    pub applies_to: Option<String>,
    /// The lookup it resolves to, for `TableLookup` provisions.
    pub rule_ref: Option<LookupKey>,
}

impl Flyweight for CodeProvision {
    type Key = ProvisionKey;
    fn flyweight_key(&self) -> ProvisionKey {
        self.key.clone()
    }
}

/// A material-blind licensed-table lookup: given input keys (span, load, spacing…) returns
/// a prescriptive answer (member size, fastening schedule…).
#[derive(Clone, PartialEq, Debug)]
pub struct PrescriptiveLookup {
    pub key: LookupKey,
    pub table: PrescriptiveTable,
    /// Names of the input axes, in order.
    pub inputs: Vec<String>,
    pub citation: CitationKey,
}

impl PrescriptiveLookup {
    /// Resolve `coords` against the backing table.
    pub fn resolve(&self, coords: &BTreeMap<String, String>) -> Option<&PrescriptiveRow> {
        self.table.lookup(coords)
    }
}

/// The licensed lookup table itself: a labeled set of citation-backed rows.
#[derive(Clone, PartialEq, Debug)]
pub struct PrescriptiveTable {
    pub key: TableKey,
    pub label: String,
    /// Axis names that a row's `coords` are keyed on.
    pub axes: Vec<String>,
    pub rows: Vec<PrescriptiveRow>,
    pub refs: Vec<CitationKey>,
}

impl PrescriptiveTable {
    /// First row whose coordinates match `coords` exactly on every axis.
    pub fn lookup(&self, coords: &BTreeMap<String, String>) -> Option<&PrescriptiveRow> {
        self.rows.iter().find(|r| &r.coords == coords)
    }
}

impl Flyweight for PrescriptiveTable {
    type Key = TableKey;
    fn flyweight_key(&self) -> TableKey {
        self.key.clone()
    }
}

/// One row of a [`PrescriptiveTable`]: a coordinate along the table's axes → a result, with
/// its own per-row citation.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PrescriptiveRow {
    pub coords: BTreeMap<String, String>,
    pub result: String,
    pub citation: Option<CitationKey>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span_table() -> PrescriptiveTable {
        PrescriptiveTable {
            key: TableKey::from("joist-spans"),
            label: "Floor joist spans".to_owned(),
            axes: vec!["size".to_owned(), "spacing".to_owned()],
            rows: vec![PrescriptiveRow {
                coords: BTreeMap::from([
                    ("size".to_owned(), "2x8".to_owned()),
                    ("spacing".to_owned(), "16".to_owned()),
                ]),
                result: "12-1".to_owned(),
                citation: None,
            }],
            refs: vec![CitationKey::book("IRC").at("Table R502.3.1(1)")],
        }
    }

    #[test]
    fn table_lookup_matches_on_all_axes() {
        let t = span_table();
        let hit = BTreeMap::from([
            ("size".to_owned(), "2x8".to_owned()),
            ("spacing".to_owned(), "16".to_owned()),
        ]);
        assert_eq!(t.lookup(&hit).unwrap().result, "12-1");

        let miss = BTreeMap::from([
            ("size".to_owned(), "2x8".to_owned()),
            ("spacing".to_owned(), "24".to_owned()),
        ]);
        assert!(t.lookup(&miss).is_none());
    }

    #[test]
    fn lookup_wraps_table() {
        let l = PrescriptiveLookup {
            key: LookupKey::from("joist-span-lookup"),
            table: span_table(),
            inputs: vec!["size".to_owned(), "spacing".to_owned()],
            citation: CitationKey::book("IRC"),
        };
        let coords = BTreeMap::from([
            ("size".to_owned(), "2x8".to_owned()),
            ("spacing".to_owned(), "16".to_owned()),
        ]);
        assert!(l.resolve(&coords).is_some());
    }
}
