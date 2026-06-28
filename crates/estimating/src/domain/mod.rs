//! The estimating layer's value objects, flyweights, and the one aggregate root ([`Estimate`]).
//!
//! Money is real decimal USD throughout — never cents (single-sourced via the materials
//! `PriceQuote`). Linear takeoff quantities are sourced from canonical int-tick fields and
//! converted to a unit of measure exactly once (see [`classification::UnitOfMeasure`]), retaining
//! the source ticks for audit, so no field is ever typed tick².

pub mod catalog;
pub mod change_order;
pub mod classification;
pub mod estimate;
pub mod lines;
pub mod markup;
pub mod rollup;
pub mod takeoff;
