//! The loads application layer: the services that distribute and combine loads. [`LoadPath`]
//! orders the connection graph, [`LoadRollup`] folds sources into unfactored demand, and
//! [`LoadSolver`] applies the combination set to emit per-member `MemberDemand[]`.

pub mod load_path;
pub mod load_rollup;
pub mod load_solver;
