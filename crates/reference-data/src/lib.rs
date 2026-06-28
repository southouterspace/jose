//! # reference-data
//!
//! The single home for every intrinsic, looked-up-by-key catalog in the system — the
//! `reference-flyweights` layer of the domain MODEL. Anything that is the same across
//! thousands of placements (a species' design values, a code provision, a prescriptive
//! table, a book citation) lives here once and is referenced by key.
//!
//! This is a **shared kernel**: pure data with no dependencies. The [`Registry`] /
//! [`Flyweight`] pair encodes the Flyweight pattern directly — intrinsic state is stored
//! once in a catalog and *referenced*, never copied per use.
//!
//! Following the MODEL's "open registry keys over closed enums" rule, material/standard
//! vocabularies are open string-backed keys ([`keys`]) rather than closed Rust enums, so
//! new materials are added as data, not code edits.

mod citation;
mod design_values;
mod keys;
mod prescriptive;
mod registry;

pub use citation::CitationKey;
pub use design_values::{MaterialDesignValueTable, MaterialFamilyKey, MechanicalProperties};
pub use keys::{DesignValueKey, LookupKey, ProvisionKey, StandardKey, StockForm, TableKey};
pub use prescriptive::{
    CodeProvision, PrescriptiveLookup, PrescriptiveRow, PrescriptiveTable, ProvisionKind,
};
pub use registry::{Flyweight, Registry};
