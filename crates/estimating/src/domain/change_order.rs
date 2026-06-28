//! [`ChangeOrder`] — an immutable delta against an [`Estimate`](crate::Estimate) baseline.
//!
//! The cost impact of a scope/geometry change, traced (like a [`TakeoffItem`](crate::TakeoffItem))
//! to the domain objects that changed. Captures the associativity payoff: edit geometry → re-solve
//! → the cost delta is itemized, not re-keyed by hand. An approved CO is never edited; a correction
//! is a new CO.

use crate::domain::takeoff::TakeoffItem;
use crate::keys::ChangeOrderId;

/// The lifecycle state of a change order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChangeOrderStatus {
    /// Submitted, not yet decided.
    Pending,
    /// Approved — its delta rolls into the estimate grand total.
    Approved,
    /// Rejected.
    Rejected,
    /// Voided.
    Void,
}

/// An immutable cost-impact delta against an estimate baseline.
#[derive(Clone, PartialEq, Debug)]
pub struct ChangeOrder {
    /// Stable identity.
    pub id: ChangeOrderId,
    /// Sequential CO number for the project.
    pub number: u32,
    /// `pending | approved | rejected | void`.
    pub status: ChangeOrderStatus,
    /// 'Added 4ft to garage gable wall'.
    pub reason: String,
    /// The `Estimate.revision` this delta is measured against.
    pub baseline_revision: u32,
    /// New quantities introduced (traced to new/changed domain objects).
    pub added_takeoff: Vec<TakeoffItem>,
    /// Quantities deleted by the change.
    pub removed_takeoff: Vec<TakeoffItem>,
    /// Net direct cost change (added − removed). Real USD.
    pub delta_direct_cost: Option<f64>,
    /// Markup applied to the delta per the estimate's stack.
    pub delta_markup: Option<f64>,
    /// Net all-in cost impact. Real USD; rolls into the grand total when approved.
    pub delta_total: f64,
    /// Approval timestamp; `None` while pending.
    pub approved_at: Option<String>,
}

impl ChangeOrder {
    /// Whether this CO's delta currently affects the estimate grand total.
    pub fn is_in_effect(&self) -> bool {
        self.status == ChangeOrderStatus::Approved
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_approved_change_orders_are_in_effect() {
        let mut co = ChangeOrder {
            id: ChangeOrderId(1),
            number: 1,
            status: ChangeOrderStatus::Pending,
            reason: "added gable".to_owned(),
            baseline_revision: 1,
            added_takeoff: vec![],
            removed_takeoff: vec![],
            delta_direct_cost: Some(500.0),
            delta_markup: Some(75.0),
            delta_total: 575.0,
            approved_at: None,
        };
        assert!(!co.is_in_effect());
        co.status = ChangeOrderStatus::Approved;
        assert!(co.is_in_effect());
    }
}
