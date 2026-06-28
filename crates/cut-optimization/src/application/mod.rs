//! The cut layer's application services — the verbs that operate over the domain.
//!
//! [`CuttingStockSolver`] is the one service: a stateless-verb-over-stateful-pool solver that
//! couples *which sticks to buy* and *how to cut them*. It reads SKU facts through the
//! [`StockCatalog`](crate::ports::StockCatalog) port and mutates the [`OffcutPool`] in place.

pub mod solver;
