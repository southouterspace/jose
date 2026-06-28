//! [`LoadPath`] — walks the connection graph downward (sheathing → stud → plate → header →
//! foundation), producing the acyclic, ordered sequence the rollup folds over.
//!
//! The graph itself is single-homed downstream (design-standard); `LoadPath` traverses it by an
//! opaque [`ConnectionGraphRef`] and is handed the edge list to order. A cycle is not a valid
//! load path (it means an unstable/over-constrained model) and yields an empty order.

use crate::keys::ConnectionGraphRef;
use building::MemberPlacementId;
use geometry_kernel::UnitVec3;
use std::collections::{BTreeMap, VecDeque};

/// Where the downward walk stops — load delivered to a support.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PathTerminal {
    Foundation,
    BearingWall,
    Beam,
    ShearwallChord,
}

/// How a node splits its load among multiple downstream edges.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShareRule {
    Tributary,
    Stiffness,
    RigidDiaphragm,
    FlexibleDiaphragm,
}

/// A directed load-transfer edge: load flows from `from` (upstream) to `to` (the member that
/// receives it).
pub type LoadEdge = (MemberPlacementId, MemberPlacementId);

/// The traversal configuration plus its derived ordering. A verb (graph walk), not retained model
/// state — `ordered_nodes` is the computed output.
#[derive(Clone, PartialEq, Debug)]
pub struct LoadPath {
    /// The connection graph this path traverses (by key — single-sourced downstream).
    pub graph: ConnectionGraphRef,
    /// Starting node; `None` → all gravity roots.
    pub root: Option<MemberPlacementId>,
    /// Where the walk stops.
    pub terminal: PathTerminal,
    /// Derived topological order (output of the walk, not stored state).
    pub ordered_nodes: Vec<MemberPlacementId>,
    /// How a node splits load among downstream edges.
    pub share_rule: ShareRule,
    /// Gravity vs lateral path direction.
    pub load_direction: UnitVec3,
    /// True for the wind/seismic lateral path.
    pub is_lateral: Option<bool>,
}

impl LoadPath {
    /// A gravity bearing path over `graph`, terminating at the foundation, tributary share.
    pub fn gravity(graph: ConnectionGraphRef) -> LoadPath {
        LoadPath {
            graph,
            root: None,
            terminal: PathTerminal::Foundation,
            ordered_nodes: Vec::new(),
            share_rule: ShareRule::Tributary,
            load_direction: UnitVec3::Y.flipped(),
            is_lateral: Some(false),
        }
    }

    /// Order the supplied edges into an acyclic downward sequence (Kahn topological sort) and
    /// store it in `ordered_nodes`. Returns the ordering. An empty result signals a cycle (an
    /// unstable model) once nodes exist but none can be sequenced.
    pub fn walk(&mut self, edges: &[LoadEdge]) -> &[MemberPlacementId] {
        self.ordered_nodes = topological_order(edges);
        &self.ordered_nodes
    }
}

/// Kahn's algorithm over a directed edge list. Deterministic (nodes processed in id order). On a
/// cycle, the nodes that *can* be ordered are returned and the cyclic remainder is dropped.
fn topological_order(edges: &[LoadEdge]) -> Vec<MemberPlacementId> {
    let mut indegree: BTreeMap<MemberPlacementId, usize> = BTreeMap::new();
    let mut adj: BTreeMap<MemberPlacementId, Vec<MemberPlacementId>> = BTreeMap::new();
    for &(from, to) in edges {
        indegree.entry(from).or_insert(0);
        *indegree.entry(to).or_insert(0) += 1;
        adj.entry(from).or_default().push(to);
    }

    let mut ready: VecDeque<MemberPlacementId> = indegree
        .iter()
        .filter(|&(_, &d)| d == 0)
        .map(|(&n, _)| n)
        .collect();

    let mut order = Vec::new();
    while let Some(n) = ready.pop_front() {
        order.push(n);
        if let Some(children) = adj.get(&n) {
            for &c in children {
                if let Some(d) = indegree.get_mut(&c) {
                    *d -= 1;
                    if *d == 0 {
                        ready.push_back(c);
                    }
                }
            }
        }
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(n: u128) -> MemberPlacementId {
        MemberPlacementId(n)
    }

    #[test]
    fn orders_sheathing_to_foundation() {
        // sheathing(1) → stud(2) → plate(3) → foundation(4)
        let edges = [(m(1), m(2)), (m(2), m(3)), (m(3), m(4))];
        let mut p = LoadPath::gravity(ConnectionGraphRef::from("g"));
        let order = p.walk(&edges).to_vec();
        assert_eq!(order, vec![m(1), m(2), m(3), m(4)]);
    }

    #[test]
    fn a_cycle_yields_no_complete_order() {
        // 1 → 2 → 1 is unstable: neither node has indegree 0.
        let edges = [(m(1), m(2)), (m(2), m(1))];
        let mut p = LoadPath::gravity(ConnectionGraphRef::from("g"));
        assert!(p.walk(&edges).is_empty());
    }
}
