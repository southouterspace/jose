//! The first-class connection graph: [`ConnectionGraph`] (nodes = members, edges = typed joints),
//! [`Connection`] (the material-blind topology half), and [`ConnectionCapacity`] (the per-strategy
//! capacity half).
//!
//! Connection *existence/topology* is shared and material-blind (here); connection *capacity* is
//! per-strategy (computed by the leaf). Edge count × fastener-per-edge feeds estimating takeoff.

use crate::keys::ConnectionGraphId;
use building::ConnectionPointRef;
use materials::PieceId;

/// The joint type, supplied/validated by the leaf's `connectionCapacity()`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConnectionMethod {
    NailYield,
    ScrewShear,
    ScrewPullout,
    BoltYield,
    BoltBearing,
    Weld,
    RebarDevelopment,
    LapSplice,
    AnchorBolt,
    Dowel,
}

/// One typed joint edge: a fastened connection between two members, with a per-instance fastener
/// count. The material-blind topology half.
#[derive(Clone, PartialEq, Debug)]
pub struct Connection {
    /// The joint type.
    pub method: ConnectionMethod,
    /// Fasteners in this joint — a primary bottom-up takeoff quantity.
    pub count: Option<u32>,
    /// References to the materials-layer connection points (geometry stays in materials).
    pub fastener_locations: Vec<ConnectionPointRef>,
}

/// First-class connection graph: nodes = members (by `Piece` ref), edges = typed joints. Identity
/// persists across recompute (it is the building's joint topology) — an entity.
#[derive(Clone, PartialEq, Debug)]
pub struct ConnectionGraph {
    /// Identity, stable across solver recompute.
    pub id: ConnectionGraphId,
    /// The connected members (referenced, not restated).
    pub nodes: Vec<PieceId>,
    /// Typed joints between nodes.
    pub edges: Vec<Connection>,
}

/// Output of the strategy's `connectionCapacity()` — the allowable/factored capacity of one typed
/// connection. Material-specific computation behind a uniform shape.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ConnectionCapacity {
    /// Mirrors `Connection.method`.
    pub method: ConnectionMethod,
    /// Nominal/allowable capacity per fastener (wood EYM `Z`, CFS screw shear, …), real force.
    pub z: f64,
    /// `Cg` / row reduction for multiple fasteners; total = `Z × count × groupFactor`.
    pub group_factor: Option<f64>,
}

impl ConnectionCapacity {
    /// Total joint capacity `Z × count × groupFactor` for a connection's fastener count.
    pub fn total_for(&self, conn: &Connection) -> f64 {
        let count = conn.count.unwrap_or(1) as f64;
        self.z * count * self.group_factor.unwrap_or(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_capacity_scales_with_count_and_group() {
        let conn = Connection {
            method: ConnectionMethod::NailYield,
            count: Some(4),
            fastener_locations: vec![],
        };
        let cap = ConnectionCapacity {
            method: ConnectionMethod::NailYield,
            z: 100.0,
            group_factor: Some(0.9),
        };
        // 100 * 4 * 0.9 = 360
        assert!((cap.total_for(&conn) - 360.0).abs() < 1e-9);
    }
}
