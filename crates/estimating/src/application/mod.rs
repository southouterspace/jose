//! The estimating layer's application services — the pure verbs over the domain.
//!
//! [`cost_rollup::CostRollup`] aggregates the priced lines into [`RollupNode`](crate::RollupNode)s
//! and a grand total via a deterministic markup/allowance stack. [`takeoff::TakeoffBuilder`] is the
//! bottom-up handoff from the cut layer — it walks a `cut_optimization::CutPlan` into traceable
//! [`TakeoffItem`](crate::TakeoffItem)s.

pub mod cost_rollup;
pub mod takeoff;
