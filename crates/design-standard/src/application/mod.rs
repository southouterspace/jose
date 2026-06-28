//! The design-standard application core: the material-blind orchestration that calls *through* the
//! [`DesignStandard`](crate::ports::design_standard::DesignStandard) port. [`BeamStatics`] supplies
//! the pure demand mechanics; [`SizingArbiter`] runs the check stage and aggregates the governing
//! utilization. Neither branches on material.

pub mod beam_statics;
pub mod sizing_arbiter;
