//! # materials
//!
//! The **Materials & Stock** bounded context — the `materials-stock` layer of the domain MODEL.
//! It owns *what a member is made of*: the material-agnostic raw-stock data model
//! ([`Stock`] + its flyweight [`StockSpec`]), the parametric geometry ([`Dimensions`],
//! [`SectionProperties`]), the cut/provenance chain ([`Piece`], [`Cut`], [`WasteRecord`],
//! [`ProvenanceLink`]), self-weight that feeds the loads layer ([`Weight`]), per-member fastener
//! locations ([`ConnectionPoint`]), the material-blind takeoff measure ([`QuantityTakeoff`]),
//! and the purchasable-SKU + price-snapshot catalog ([`SupplierSku`], [`PriceQuote`],
//! [`PriceTier`]).
//!
//! It carries **no** mechanical design values and **no** load/cost computation — design values
//! resolve through the `DesignStandard` seam, and the load traversal and estimate rollup are
//! separate downstream concerns that *consume* these types. It is the upstream data floor of the
//! framing model and depends only on the two shared kernels.
//!
//! The whole layer obeys the base-unit invariant: linear geometry is integer [`Tick`]s @
//! 1/32in; area/volume/weight/money are derived reals — no area is ever typed tick².
//!
//! ## The two material seams
//!
//! - [`MaterialClass`] + [`StockForm`] route material identity and the pipeline output path
//!   without any per-material field on [`Stock`] — discriminated routing by `stockForm`, never
//!   by material.
//! - [`StockSpec`] carries only an opaque `design_value_ref` the strategy resolves, so a steel,
//!   concrete and wood spec are the *same type* — the leaf behind the seam owns the physics.

mod domain;
mod keys;

pub use domain::cut::{
    Cut, CutTarget, Piece, PieceRole, ProvenanceKind, ProvenanceLink, QuantityTakeoff, WasteRecord,
};
pub use domain::discriminators::{Form, MaterialClass, QuantityBasis, StockForm};
pub use domain::geometry::{Axis, Dimensions, EdgeProfile, SectionProperties};
pub use domain::pricing::{PriceQuote, PriceTier, SupplierSku, Timestamp, UnitOfSale};
pub use domain::stock::{Stock, StockSpec, Treatment};
pub use domain::weight::{ConnectionPoint, Weight};
pub use keys::{
    ConnectionMethodKey, ConnectionTypeKey, CutId, MeshKey, PieceId, ProfileKey, SkuKey, SpecKey,
    StockId,
};

// Re-export the geometry-kernel `Tick` so the doc links above resolve and downstream contexts
// can name the base unit through this facade.
pub use geometry_kernel::Tick;
