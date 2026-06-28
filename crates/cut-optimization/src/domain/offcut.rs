//! [`Offcut`] and the stateful [`OffcutPool`].
//!
//! An offcut is a reusable remainder produced by a prior cut — zero-marginal-cost supply once
//! produced, because its cost was already booked to the origin stick. The pool is the *only
//! entity* in this layer: statefulness is the point. It is consulted BEFORE new stock is opened,
//! so leftovers become supply.

use crate::keys::{AssignmentId, OffcutId};
use geometry_kernel::Tick;
use materials::{SkuKey, SpecKey};

/// One reusable remainder produced by a prior cut. Traces to exactly one purchased stick.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Offcut {
    /// Stable id so a later assignment can declare it consumed.
    pub id: OffcutId,
    /// Remaining usable length in ticks.
    pub length: Tick,
    /// → [`materials::StockSpec`] — an offcut can only fulfill demand of the same spec.
    pub spec_ref: SpecKey,
    /// → the [`CutAssignment`](crate::CutAssignment) that produced this remainder.
    pub parent_assignment_ref: AssignmentId,
    /// → the originally-bought stick at the root of this offcut's cost chain. Always present, so
    /// the bottom-up takeoff never has an uncostable orphan offcut.
    pub origin_sku_ref: SkuKey,
}

/// How an offcut is matched to remaining demand.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MatchMode {
    /// Fit any offcut to any remaining same-spec demand — the default.
    #[default]
    Smart,
    /// Only an exact-length offcut may fulfill a demand.
    Exact,
    /// Never reuse offcuts.
    None,
}

/// Whether the pool resets per solve or persists across a project's runs.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PoolScope {
    /// Reset per solve — the default.
    #[default]
    Solve,
    /// Persist across a project's runs: yesterday's scrap supplies today's cut.
    Project,
}

/// The running, mutable set of reusable remainders. The sole entity in the layer.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct OffcutPool {
    /// Identity persists — the pool is stateful, not a snapshot.
    pub id: OffcutId,
    /// Current reusable remainders.
    pub pieces: Vec<Offcut>,
    /// Reuse threshold ≈ clear bay span (14.5in @ 16in OC = 464 ticks). Below this, an offcut is
    /// kept only if a shorter demand still fits; otherwise it is waste.
    pub min_reusable: Tick,
    /// `smart | exact | none` — defaults to smart.
    pub match_mode: MatchMode,
    /// `solve | project` — defaults to solve.
    pub scope: PoolScope,
}

impl OffcutPool {
    /// An empty pool with the given reuse threshold and default smart/solve policy.
    pub fn new(id: OffcutId, min_reusable: Tick) -> OffcutPool {
        OffcutPool {
            id,
            pieces: Vec::new(),
            min_reusable,
            match_mode: MatchMode::Smart,
            scope: PoolScope::Solve,
        }
    }

    /// Find the best offcut that can supply a cut of `length`/`spec`, honoring the match mode.
    /// "Best" = the *tightest* fit (smallest sufficient remainder) so long sticks stay whole.
    /// Returns the index into [`OffcutPool::pieces`], or `None` if nothing fits.
    pub fn best_fit(&self, length: Tick, spec: &SpecKey) -> Option<usize> {
        if self.match_mode == MatchMode::None {
            return None;
        }
        self.pieces
            .iter()
            .enumerate()
            .filter(|(_, o)| &o.spec_ref == spec)
            .filter(|(_, o)| match self.match_mode {
                MatchMode::Exact => o.length == length,
                _ => o.length.raw() >= length.raw(),
            })
            .min_by_key(|(_, o)| o.length.raw())
            .map(|(i, _)| i)
    }

    /// Remove and return the offcut at `index` (it has been consumed).
    pub fn take(&mut self, index: usize) -> Offcut {
        self.pieces.remove(index)
    }

    /// Decide whether a remainder of `length` is worth pooling (≥ `min_reusable`).
    pub fn worth_pooling(&self, length: Tick) -> bool {
        length.raw() >= self.min_reusable.raw()
    }

    /// Add a newly-produced offcut to the pool.
    pub fn push(&mut self, offcut: Offcut) {
        self.pieces.push(offcut);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offcut(id: u128, length: i32, spec: &str) -> Offcut {
        Offcut {
            id: OffcutId(id),
            length: Tick(length),
            spec_ref: SpecKey::from(spec),
            parent_assignment_ref: AssignmentId(1),
            origin_sku_ref: SkuKey::from("HD-2x4-8"),
        }
    }

    #[test]
    fn best_fit_picks_the_tightest_sufficient_offcut() {
        let mut pool = OffcutPool::new(OffcutId(1), Tick(100));
        pool.push(offcut(10, 4000, "SPF"));
        pool.push(offcut(11, 1000, "SPF"));
        pool.push(offcut(12, 500, "SPF")); // too short for a 600-tick cut
        let idx = pool.best_fit(Tick(600), &SpecKey::from("SPF")).unwrap();
        assert_eq!(pool.pieces[idx].id, OffcutId(11)); // 1000 is the tightest ≥ 600
    }

    #[test]
    fn best_fit_respects_spec_and_match_mode() {
        let mut pool = OffcutPool::new(OffcutId(1), Tick(100));
        pool.push(offcut(10, 4000, "DF")); // wrong spec
        assert!(pool.best_fit(Tick(600), &SpecKey::from("SPF")).is_none());

        pool.match_mode = MatchMode::None;
        pool.push(offcut(11, 4000, "SPF"));
        assert!(pool.best_fit(Tick(600), &SpecKey::from("SPF")).is_none());
    }

    #[test]
    fn worth_pooling_uses_the_threshold() {
        let pool = OffcutPool::new(OffcutId(1), Tick(464));
        assert!(pool.worth_pooling(Tick(500)));
        assert!(!pool.worth_pooling(Tick(400)));
    }
}
