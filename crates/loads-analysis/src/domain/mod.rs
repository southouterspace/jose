//! The loads domain: the ASCE 7 source value objects, tributary geometry, the combination
//! recipe, and the demand records. Pure and material-blind — psf/plf/lb/in, never a material
//! constant.

pub mod combination;
pub mod demand;
pub mod sources;
pub mod tributary;
