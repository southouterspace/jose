//! [`Weight`] (self-weight mass properties) and [`ConnectionPoint`] (per-member fastener
//! locations). Both are intrinsic stock properties that *feed* downstream layers — `Weight`
//! feeds the loads dead-load rollup; `ConnectionPoint` feeds the connection graph.

use crate::domain::geometry::Dimensions;
use crate::keys::{ConnectionMethodKey, ConnectionTypeKey};
use geometry_kernel::{TickVec3, UnitVec3};

/// Self-weight mass properties of a member, driven by spec density × volume. The only
/// pre-existing piece of the loads layer — it *feeds* the dead-load rollup; it performs no load
/// combination itself. All real units (lb/ft³, lb/ft, lb).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Weight {
    /// Density at the stated moisture/service condition, lb/ft³. Sourced from `StockSpec.density`.
    pub density: f64,
    /// Derived linear weight, lb/ft.
    pub weight_per_foot: Option<f64>,
    /// Derived total weight, lb — the self-weight contribution to the dead-load rollup.
    pub total_weight: f64,
}

impl Weight {
    /// Derive self-weight from a density (lb/ft³) and member [`Dimensions`]. Volume converts
    /// from in³ to ft³ (÷1728); the linear weight is the total spread over the member length.
    pub fn from_density_and_dimensions(density: f64, dims: &Dimensions) -> Weight {
        let volume_ft3 = dims.volume_in3() / 1728.0;
        let total_weight = density * volume_ft3;
        let length_ft = dims.length.to_feet();
        let weight_per_foot = if length_ft > 0.0 {
            Some(total_weight / length_ft)
        } else {
            None
        };
        Weight {
            density,
            weight_per_foot,
            total_weight,
        }
    }
}

/// A per-member location/normal where fasteners land. Bridges this member into the connection
/// graph. Per-instance contextual data, distinct from the aggregate graph that consumes it.
#[derive(Clone, PartialEq, Debug)]
pub struct ConnectionPoint {
    /// On the member surface — a committed world position in ticks (vec3<int>).
    pub position: TickVec3,
    /// Face direction for fastener seating — a unitless direction, explicitly not a tick position.
    pub normal: UnitVec3,
    /// Connection topology (open key), if known.
    pub connection_type: Option<ConnectionTypeKey>,
    /// Fastening method (open key) — resolves capacity through the `DesignStandard` seam.
    pub method: Option<ConnectionMethodKey>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use geometry_kernel::Tick;

    #[test]
    fn self_weight_from_density() {
        // 2x4 @ 8ft, SPF density 31.2 lb/ft³. Volume = 504 in³ = 0.291667 ft³.
        let dims = Dimensions::rectangular("2x4", Tick(3072), Tick(112), Tick(48));
        let w = Weight::from_density_and_dimensions(31.2, &dims);
        let expected_total = 31.2 * (504.0 / 1728.0);
        assert!((w.total_weight - expected_total).abs() < 1e-9);
        // weight per foot = total / 8ft.
        assert!((w.weight_per_foot.unwrap() - expected_total / 8.0).abs() < 1e-9);
    }

    #[test]
    fn connection_point_keeps_position_and_direction_distinct() {
        let cp = ConnectionPoint {
            position: TickVec3::new(Tick(0), Tick(0), Tick(96)),
            normal: UnitVec3::Z,
            connection_type: Some(ConnectionTypeKey::from("stud-to-plate")),
            method: Some(ConnectionMethodKey::from("end-nail")),
        };
        assert_eq!(cp.position.z, Tick(96));
        assert_eq!(cp.method.unwrap().as_str(), "end-nail");
    }
}
