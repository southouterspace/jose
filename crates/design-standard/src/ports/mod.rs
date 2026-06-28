//! Ports: the abstractions the application core depends on. The [`DesignStandard`] Strategy
//! interface is the seam — the arbiter (application) depends on it; the material leaves (adapters)
//! implement it.

pub mod design_standard;
