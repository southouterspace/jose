//! Extension-stub assemblies: [`Floor`], [`Roof`], [`Sheathing`]. Each reuses the exact `Wall`
//! promotion pattern (`sourceFace` → assembly entity) and emits `MemberPlacement[]` through the
//! same [`FramingSolver`](crate::FramingSolver) keyed by `assemblyKind`, so new assemblies land
//! without restructuring the placement layer.

use crate::domain::spacing::SpacingModule;
use crate::keys::{FaceRef, FloorId, RoofId, SheathingId, WallId};
use geometry_kernel::Tick;
use materials::SpecKey;

/// A floor/ceiling assembly promoted from a horizontal kernel face. Joists/rims/blocking emit
/// through the framer (`assemblyKind = floor`).
#[derive(Clone, PartialEq, Debug)]
pub struct Floor {
    pub id: FloorId,
    /// The horizontal kernel face this floor was promoted from.
    pub source_face: FaceRef,
    /// Clear span in ticks; drives joist depth/spacing.
    pub span: Option<Tick>,
    /// On-center layout module (a parameter, not ticks).
    pub spacing: Option<SpacingModule>,
}

/// A unitless rise/run roof pitch (e.g. 6:12) — **not** ticks. Drives rafter length and
/// bird's-mouth geometry.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RisePerRun {
    pub rise: u32,
    pub run: u32,
}

impl RisePerRun {
    /// The pitch as a real slope (rise ÷ run).
    pub fn slope(self) -> f64 {
        self.rise as f64 / self.run as f64
    }
}

/// A roof assembly promoted from sloped kernel faces. Rafters/trusses/ridge emit through the
/// framer (`assemblyKind = roof`).
#[derive(Clone, PartialEq, Debug)]
pub struct Roof {
    pub id: RoofId,
    /// The sloped kernel face(s) this roof was promoted from.
    pub source_face: FaceRef,
    /// Rise/run pitch (a unitless ratio).
    pub pitch: Option<RisePerRun>,
    /// On-center layout module (a parameter, not ticks).
    pub spacing: Option<SpacingModule>,
}

/// Which assembly face a [`Sheathing`] skin covers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AssemblyFace {
    Wall(WallId),
    Floor(FloorId),
    Roof(RoofId),
}

/// A sheet-good skin (OSB/plywood/gypsum) applied to a wall/floor/roof face. Promotes to
/// `MemberPlacement`s of `role = panel` **and** supplies the dominant bracing source
/// (`bracedBy = sheathing`) for the members it covers. An entity — it owns identity, is referenced
/// by bracing, and emits placements through the framer.
#[derive(Clone, PartialEq, Debug)]
pub struct Sheathing {
    pub id: SheathingId,
    /// The assembly face this skin covers.
    pub applies_to: AssemblyFace,
    /// Sheet-good spec (material-agnostic); cost-takeoff key for sheet goods.
    pub panel_spec_ref: Option<SpecKey>,
    /// Fastener edge spacing in ticks; a shear-capacity input.
    pub edge_spacing: Option<Tick>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use geometry_kernel::EntityId;

    #[test]
    fn pitch_is_a_unitless_ratio() {
        let p = RisePerRun { rise: 6, run: 12 };
        assert!((p.slope() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn sheathing_covers_an_assembly_face() {
        let s = Sheathing {
            id: SheathingId(1),
            applies_to: AssemblyFace::Wall(WallId(1)),
            panel_spec_ref: Some(SpecKey::from("OSB-7/16")),
            edge_spacing: Some(Tick(192)),
        };
        assert_eq!(s.applies_to, AssemblyFace::Wall(WallId(1)));
        let _ = FaceRef {
            volume: EntityId(1),
            face_index: 0,
        };
    }
}
