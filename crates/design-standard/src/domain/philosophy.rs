//! [`DesignPhilosophy`] — the cross-cutting ASD/LRFD flag, plus the [`DesignCode`] and
//! [`MaterialKind`] discriminators a leaf declares.

/// Allowable Stress Design vs Load & Resistance Factor Design.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PhilosophyMode {
    Asd,
    Lrfd,
}

/// Which side the safety factor hits.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FactorSide {
    /// ASD — factors live in the load combinations.
    Load,
    /// LRFD — φ reduces capacity.
    Resistance,
}

/// The cross-cutting ASD|LRFD flag — **not** a per-material constant. Decides whether safety
/// factors hit the load side (ASD) or the resistance side (LRFD). Carried by the arbiter; each
/// leaf declares its default. Per-limit-state Ω/φ are emitted as `ModificationFactor`s, not as
/// scalars here — this VO carries only the genuinely cross-cutting mode.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DesignPhilosophy {
    pub mode: PhilosophyMode,
    pub factor_side: FactorSide,
}

impl DesignPhilosophy {
    /// Allowable Stress Design (wood default).
    pub const ASD: DesignPhilosophy = DesignPhilosophy {
        mode: PhilosophyMode::Asd,
        factor_side: FactorSide::Load,
    };
    /// Load & Resistance Factor Design (steel/concrete default).
    pub const LRFD: DesignPhilosophy = DesignPhilosophy {
        mode: PhilosophyMode::Lrfd,
        factor_side: FactorSide::Resistance,
    };
}

/// Which standard a leaf implements.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DesignCode {
    Nds,
    Aisi,
    Aisc,
    Aci,
    Tms,
}

/// The material a leaf sizes (steel covers both AISI cold-formed and AISC hot-rolled).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MaterialKind {
    Wood,
    Steel,
    Concrete,
    Masonry,
}

/// Which section basis the strategy computes its properties on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SectionBasisKind {
    Gross,
    Effective,
    Transformed,
    Cracked,
    Net,
    Plastic,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn philosophy_pairs_mode_with_side() {
        assert_eq!(DesignPhilosophy::ASD.factor_side, FactorSide::Load);
        assert_eq!(DesignPhilosophy::LRFD.factor_side, FactorSide::Resistance);
    }
}
