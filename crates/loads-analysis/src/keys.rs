//! Opaque handles to **downstream** design-standard concepts.
//!
//! The loads layer sits upstream of the structural seam in the pipeline, yet the schema has it
//! *referencing* a few design-standard types (the connection graph it walks, the ASD/LRFD
//! philosophy a combination points at, the strategy the solver delegates governing-combo
//! selection to). To keep the crate graph acyclic — loads-analysis must not depend on the
//! design-standard crate — those cross-references are modeled as opaque key handles here, exactly
//! the "reference by key" idiom the schema uses pervasively. The design-standard crate resolves
//! them; this crate only points.

macro_rules! ref_key {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        pub struct $name(pub String);

        impl $name {
            /// Borrow the key as a string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                $name(s.to_owned())
            }
        }
    };
}

ref_key!(
    /// Handle to the design-standard `ConnectionGraph` the [`LoadPath`](crate::LoadPath) walks.
    /// The graph is single-homed downstream; the path traverses it by key.
    ConnectionGraphRef
);
ref_key!(
    /// Handle to the design-standard `DesignPhilosophy` (ASD | LRFD) a
    /// [`LoadCombination`](crate::LoadCombination) is built under. Referenced, not redefined —
    /// the flag is single-homed downstream.
    DesignPhilosophyRef
);
ref_key!(
    /// Handle to the active design-standard `DesignStandard` strategy the
    /// [`LoadSolver`](crate::LoadSolver) delegates governing-combination selection to.
    DesignStandardRef
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refs_construct() {
        assert_eq!(ConnectionGraphRef::from("g1").as_str(), "g1");
        assert_eq!(
            DesignPhilosophyRef::from("ASD"),
            DesignPhilosophyRef("ASD".to_owned())
        );
    }
}
