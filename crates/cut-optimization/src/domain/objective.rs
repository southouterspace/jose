//! [`CutObjective`] — the optimizer's tunable goal.
//!
//! The user dial between minimizing waste and minimizing cost, plus the solve method. Captured on
//! the plan so the resulting estimate is reproducible and explainable ("optimized for cost at
//! weight 0.3 via ffd-plus-pool").

/// The dominant objective.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Primary {
    /// Minimize total scrapped length — the default (cost as tiebreak).
    #[default]
    MinWaste,
    /// Minimize total purchase cost.
    MinCost,
}

/// The solve method recorded for reproducibility.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SolveMethod {
    /// Integer-linear-programming optimum (small N).
    IlpOptimal,
    /// First-fit-decreasing plus the offcut pool — the workhorse heuristic.
    FfdPlusPool,
    /// Pick by demand count.
    #[default]
    Auto,
}

/// The optimizer's goal: the waste↔cost dial plus the method.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CutObjective {
    /// The dominant objective.
    pub primary: Primary,
    /// 0.0–1.0 blend (0 = pure waste, 1 = pure cost). The user dial; defaults to 0.
    pub cost_weight: f64,
    /// The solve method; defaults to auto.
    pub method: SolveMethod,
}

impl Default for CutObjective {
    fn default() -> Self {
        CutObjective {
            primary: Primary::MinWaste,
            cost_weight: 0.0,
            method: SolveMethod::Auto,
        }
    }
}

impl CutObjective {
    /// The default waste-first objective.
    pub fn min_waste() -> CutObjective {
        CutObjective::default()
    }

    /// A cost-first objective at full cost weight.
    pub fn min_cost() -> CutObjective {
        CutObjective {
            primary: Primary::MinCost,
            cost_weight: 1.0,
            method: SolveMethod::Auto,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_waste_first() {
        let o = CutObjective::default();
        assert_eq!(o.primary, Primary::MinWaste);
        assert_eq!(o.cost_weight, 0.0);
    }
}
