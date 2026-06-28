//! # estimating
//!
//! The **Estimating & Cost** bounded context — the `estimating-cost` layer of the domain MODEL.
//! Bottom-up project estimating: turn the solved domain graph (the [`cut_optimization::CutPlan`]
//! cut list + offcut/kerf waste) and the supplier catalog into a fully cost-coded, marked-up
//! project [`Estimate`], with every dollar traceable back to the domain object that demanded it
//! (the [`TakeoffItem`] chain), and validated top-down against an external benchmark through the
//! [`CostBenchmark`] seam.
//!
//! ## Material-agnostic by construction
//!
//! A [`MaterialLine`] prices off `Stock.stockForm` (carried, not redeclared), NOT species: a CFS
//! stud, a rebar stick, a sheet of OSB, a cubic yard of concrete, and a box of prefab hangers are
//! all a `material` line with a different UOM and SKU. [`CostCode`] is a pluggable flyweight over
//! MasterFormat / Uniformat; [`AssemblyCost`] and [`ResourceRate`] are flyweight catalogs; new
//! materials add catalog *rows*, never types. [`CostType`] is a closed, material-blind economic
//! enum.
//!
//! ## The two halves
//!
//! - **Bottom-up.** [`TakeoffBuilder`] walks a cut plan into traceable [`TakeoffItem`]s;
//!   [`MaterialLine`] / [`ResourceLine`] / [`SubcontractLine`] price them; [`CostRollup`] sums them
//!   by WBS / economic axis and applies the [`Markup`] / [`Allowance`] stack deterministically into
//!   [`RollupNode`]s and a grand total.
//! - **Top-down.** [`CostBenchmark`] (an «interface» Strategy seam — the cost-side mirror of
//!   `design-standard`'s `DesignStandard`) returns an independent cost; [`RsMeansBenchmark`] is one
//!   leaf, ENR / supplier-index / historical-job plug in beside it. The variance lands on
//!   [`Estimate::benchmark_variance`], closing the bottom-up/top-down loop.
//!
//! Money is real decimal USD throughout — never cents (single-sourced via the materials
//! `PriceQuote`). Linear takeoff quantities convert from canonical int ticks to a UOM exactly once
//! (the [`UnitOfMeasure`] seam), retaining the source ticks for audit, so no field is ever tick².

mod adapters;
mod application;
mod domain;
mod keys;
mod ports;

pub use adapters::rsmeans::RsMeansBenchmark;
pub use application::cost_rollup::{CostRollup, GroupBy, RollupInput, RollupOutput};
pub use application::takeoff::TakeoffBuilder;
pub use domain::catalog::{
    AssemblyComponent, AssemblyCost, AssemblyMethod, ResourceKind, ResourceRate,
};
pub use domain::change_order::{ChangeOrder, ChangeOrderStatus};
pub use domain::classification::{
    CodeSystem, CostCode, CostKind, CostType, Dimension, UnitOfMeasure,
};
pub use domain::estimate::Estimate;
pub use domain::lines::{
    CostLine, MaterialLine, PayItem, PriceQuoteRef, ResourceLine, StockFormPath, SubcontractLine,
};
pub use domain::markup::{
    Allowance, AllowanceKind, AllowanceMethod, AppliesToBase, Markup, MarkupKind, MarkupMethod,
};
pub use domain::rollup::RollupNode;
pub use domain::takeoff::{DomainRef, TakeoffItem};
pub use keys::{
    AllowanceId, AssemblyKey, ChangeOrderId, CostCodeKey, EstimateId, MarkupId, MaterialLineId,
    PayItemId, ProjectRef, RateKey, ResourceLineId, TakeoffId, UomKey,
};
pub use ports::cost_benchmark::{BenchmarkResult, BenchmarkSource, CostBenchmark};
