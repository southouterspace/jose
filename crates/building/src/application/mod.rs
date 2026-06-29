//! The building application layer: the use-case service that orchestrates the domain. The
//! [`FramingSolver`](framing_solver::FramingSolver) turns a wall (plus its junctions) into a
//! derived, stable `MemberPlacement[]`.

pub mod framing_solver;
pub mod junction_detector;
