//! [`KerfSpec`] — the material loss per cut.
//!
//! Kerf is intrinsic to the cutting tool, not the demand, so it is factored out of per-demand data
//! and counted once between adjacent cuts on a stick. When tool-derived, the blade width is read
//! through a `ToolDefinition` flyweight rather than copied per solve.

use geometry_kernel::Tick;

/// A handle to the `ToolDefinition` flyweight whose blade width sources the kerf. The drawing /
/// workspace-render layer owns that catalog (Phase 4+); here it is referenced by opaque key.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ToolDefinitionKey(pub String);

impl From<&str> for ToolDefinitionKey {
    fn from(s: &str) -> Self {
        ToolDefinitionKey(s.to_owned())
    }
}

/// The material loss per cut, counted between adjacent cuts on a stick.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct KerfSpec {
    /// Blade width consumed per cut ≈ 1/8in = 4 ticks. Counted once per interior cut on a stick.
    pub kerf: Tick,
    /// Squaring/defect trim removed from each new stick before the first cut. Defaults to 0.
    pub end_trim: Tick,
    /// → the `ToolDefinition` flyweight when kerf is tool-derived; `None` when given directly.
    pub tool_ref: Option<ToolDefinitionKey>,
}

impl KerfSpec {
    /// A standard 1/8in (4-tick) saw kerf with no end trim.
    pub fn saw() -> KerfSpec {
        KerfSpec {
            kerf: Tick(4),
            end_trim: Tick::ZERO,
            tool_ref: None,
        }
    }

    /// A kerf of the given blade width (ticks), no end trim.
    pub fn of(kerf: Tick) -> KerfSpec {
        KerfSpec {
            kerf,
            end_trim: Tick::ZERO,
            tool_ref: None,
        }
    }
}

impl Default for KerfSpec {
    fn default() -> Self {
        KerfSpec::saw()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saw_kerf_is_an_eighth_inch() {
        assert_eq!(KerfSpec::saw().kerf, Tick(4));
        assert_eq!(KerfSpec::default().end_trim, Tick::ZERO);
    }
}
