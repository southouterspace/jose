//! # cut-optimization
//!
//! The **Cut & Optimization** bounded context — the `cut-optimization` layer of the domain MODEL.
//! The 1D cutting-stock / offcut tracer: every required cut becomes a [`Demand`] line; the
//! [`CuttingStockSolver`] assigns demand to real buyable [`StockOption`]s drawn ONLY from the
//! supplier catalog, kerf-aware ([`KerfSpec`]), reusing remainders from the [`OffcutPool`] by smart
//! match. It couples *which sticks to buy* and *how to cut them* to minimize waste then cost, and
//! emits a [`CutPlan`] aggregate — [`CutAssignment`]s per consumed stick + a de-duplicated
//! [`BuyLine`] buy-list + derived waste / quantity / cost rollups — plus per-cut
//! [`PieceProvenance`] that back-links each cut to the canonical [`materials::Piece`], the producing
//! assignment, and the satisfied demand. The `CutPlan` is the single bottom-up handoff to
//! estimating.
//!
//! ## Material-agnostic by construction
//!
//! Capability is gated to `stockForm=linear` ONLY, via the typed [`CutEligibility`] verdict
//! (design-standard gotcha #3): sheet goods route to nesting, cast routes to formwork+volume, unit
//! stock bypasses entirely. Cold-formed-steel studs and rebar are *also* `linear` and flow through
//! the same solver unchanged; adding masonry or hot-rolled later only sets a `stockForm`
//! classification upstream behind the `DesignStandard` seam — never a new branch here. The solver
//! never names a material: it matches on spec, length, kerf, and the eligibility verdict alone.
//!
//! ## Flyweight discipline at the seam
//!
//! [`StockOption`] and [`Offcut`] store **no** intrinsic SKU data — length, pack, and price live on
//! [`materials::SupplierSku`] / `PriceQuote` and are read *through* the
//! [`StockCatalog`](ports::StockCatalog) port. The composition root backs that port with the
//! materials catalog; the solver depends only on the trait, so the flyweight is never copied.

mod application;
mod domain;
mod keys;
pub mod ports;

pub use application::solver::{CutRequest, CuttingStockSolver};
pub use domain::assignment::{CutAssignment, CutLine, RemainderFate, StickSource};
pub use domain::demand::{CutRole, Demand, EndCut};
pub use domain::eligibility::{CutEligibility, RouteTo};
pub use domain::kerf::{KerfSpec, ToolDefinitionKey};
pub use domain::objective::{CutObjective, Primary, SolveMethod};
pub use domain::offcut::{MatchMode, Offcut, OffcutPool, PoolScope};
pub use domain::option::StockOption;
pub use domain::plan::{BuyLine, CutPlan, DerivedQuantity};
pub use domain::provenance::PieceProvenance;
pub use keys::{AssignmentId, CutPlanId, DemandLineKey, OffcutId};
pub use ports::stock_catalog::StockCatalog;
