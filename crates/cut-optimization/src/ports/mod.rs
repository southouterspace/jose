//! Ports — the traits this context needs from the outside.
//!
//! The solver is material-blind and flyweight-respecting: it never copies a SKU's intrinsic
//! length onto a [`StockOption`](crate::StockOption). Instead it reads stock length (and, when
//! priced, unit cost) *through* the [`StockCatalog`] port, which the composition root backs with
//! the materials catalog. This is the hexagonal seam: the solver depends on the port, not on a
//! concrete catalog.

pub mod stock_catalog;
