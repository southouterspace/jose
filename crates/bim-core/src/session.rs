//! The in-session composition root: holds canonical state and runs the **draw → recompute**
//! pipeline, wiring the building context (semantic promotion + the [`FramingSolver`]) into the SoA
//! [`MemberBuffer`] the render mirror reads.
//!
//! This is the one place the bounded contexts are composed. Today the slice is
//! `promote wall → frame members → write buffer`; loads-analysis and the design-standard check
//! slot in here as later pipeline stages without changing the boundary or the buffer contract.

use crate::buffer::{MemberBuffer, MemberRow, NOMINAL_WIDTH};
use crate::command::{Command, DrawWall};
use building::{
    AssemblyKind, FramingSolver, MemberPlacement, RuleSet, SpacingAnchor, SpacingKey,
    SpacingModule, Wall, WallRole,
};
use building::{FaceRef, WallId};
use geometry_kernel::{EntityId, Segment, Tick, TickVec3};
use materials::SpecKey;

/// Nominal framed wall thickness — a 2x4 wall = 3.5in = 112 ticks.
const WALL_THICKNESS: i32 = 112;

/// The canonical in-session model. Owns the recompute state (the framer's id counter) and the SoA
/// buffer; a `Command` in, a rewritten buffer out.
#[derive(Clone, Debug)]
pub struct Session {
    framer: FramingSolver,
    buffer: MemberBuffer,
}

impl Default for Session {
    fn default() -> Self {
        Session::new()
    }
}

impl Session {
    /// A fresh session with an empty buffer.
    pub fn new() -> Session {
        Session {
            framer: FramingSolver::new(),
            buffer: MemberBuffer::new(),
        }
    }

    /// The canonical SoA bytes — the read-only render mirror handed to JS.
    pub fn buffer_bytes(&self) -> &[u8] {
        self.buffer.as_bytes()
    }

    /// Number of live member rows in the buffer.
    pub fn member_count(&self) -> usize {
        self.buffer.len()
    }

    /// Apply one command and recompute, returning the new live member count.
    pub fn apply(&mut self, command: Command) -> usize {
        match command {
            Command::DrawWall(draw) => self.draw_wall(draw),
        }
    }

    fn draw_wall(&mut self, draw: DrawWall) -> usize {
        // Promote the baseline into a wall (the building context derives length, stud counts, etc).
        let baseline = Segment::new(
            TickVec3::new(Tick(draw.x0), Tick(draw.y0), Tick::ZERO),
            TickVec3::new(Tick(draw.x1), Tick(draw.y1), Tick::ZERO),
        );
        let wall = Wall::promote(
            WallId(1),
            FaceRef {
                volume: EntityId(1),
                face_index: 0,
            },
            baseline,
            Tick(draw.height),
            Tick(WALL_THICKNESS),
            WallRole::Bearing,
            spacing_module(draw.spacing_inches),
        );

        // Frame it (a conventional light-frame wall pack), then flatten to render rows.
        let rules = RuleSet::light_frame_wall(
            SpecKey::from("SPF-STUD"),
            SpecKey::from("SPF-PLATE"),
            SpecKey::from("DF-HEADER"),
        );
        let members = self.framer.frame(AssemblyKind::Wall, &wall, &[], &rules);

        self.framer = FramingSolver::new(); // stable ids per recompute; the buffer is the truth
        self.buffer.clear();
        for member in &members {
            self.buffer.push(member_row(member));
        }
        self.buffer.len()
    }
}

/// Build a `SpacingModule` from a real inch value — directly, since a wall may be framed at any
/// module (16, 19.2, 24); the building context's whole-inch constructor cannot express 19.2.
fn spacing_module(inches: f64) -> SpacingModule {
    SpacingModule {
        module: SpacingKey::from(format!("{inches}").as_str()),
        exact_inches: inches,
        anchor: Some(SpacingAnchor::WallStart),
    }
}

/// Flatten one placed member into a render row (a wall-local elevation segment + width + role id).
///
/// Vertical members extend up the wall (+Z); horizontal members run along the baseline (+X). The
/// [`FramingRole`](building::FramingRole) knows which it is and carries its own `roleId`, so the
/// buffer encoding needs no string lookup and no fallback.
fn member_row(member: &MemberPlacement) -> MemberRow {
    let origin = member.transform.origin;
    let length = member.length.raw();
    let (x1, y1, z1) = if member.role.is_vertical() {
        (origin.x.raw(), origin.y.raw(), origin.z.raw() + length)
    } else {
        (origin.x.raw() + length, origin.y.raw(), origin.z.raw())
    };
    MemberRow {
        x0: origin.x.raw(),
        y0: origin.y.raw(),
        z0: origin.z.raw(),
        x1,
        y1,
        z1,
        width: NOMINAL_WIDTH,
        role_id: member.role.id(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::member_placement as layout;
    use building::FramingRole;
    use geometry_kernel::TICKS_PER_FOOT;

    #[test]
    fn framing_role_ids_match_the_generated_buffer_vocabulary() {
        // FramingRole (building) and the generated ROLES table (codegen) are two encodings of one
        // vocabulary. This guard fails loudly if a schema edit or a new variant makes them drift,
        // instead of letting member_row mis-encode a roleId at runtime.
        assert_eq!(FramingRole::ALL.len(), layout::ROLES.len());
        for (i, &role) in FramingRole::ALL.iter().enumerate() {
            assert_eq!(
                role.id() as usize,
                i,
                "{role:?} id must equal its ROLES index"
            );
            assert_eq!(
                role.as_str(),
                layout::ROLES[i],
                "{role:?} string must match ROLES[{i}]"
            );
            assert_eq!(layout::role_id(role.as_str()), Some(role.id()));
        }
    }

    fn draw_10ft_wall() -> Command {
        Command::DrawWall(DrawWall {
            x0: 0,
            y0: 0,
            x1: 10 * TICKS_PER_FOOT, // 10ft
            y1: 0,
            height: 8 * TICKS_PER_FOOT, // 8ft
            spacing_inches: 16.0,
        })
    }

    #[test]
    fn draw_wall_populates_the_buffer() {
        let mut s = Session::new();
        assert_eq!(s.member_count(), 0);
        let count = s.apply(draw_10ft_wall());
        assert!(count > 0);
        assert_eq!(s.member_count(), count);
        // The buffer is the full generated block regardless of live count.
        assert_eq!(s.buffer_bytes().len(), layout::BUFFER_BYTES);
    }

    #[test]
    fn redraw_replaces_rather_than_appends() {
        let mut s = Session::new();
        let first = s.apply(draw_10ft_wall());
        let second = s.apply(draw_10ft_wall());
        assert_eq!(first, second); // identical input → identical count, not doubled
    }

    #[test]
    fn footprint_and_volume_buffers_are_pure_column_major() {
        // ADR 0008: the plan view reads the `footprint` buffer (one row per world-XY vertex),
        // the 3D view reads `footprint` + `volume`. Lock the generated contract — column count,
        // capacity, and that every column is exactly CAPACITY contiguous elements at a generated
        // offset (pure column-major, mirroring the member_placement assertions above).
        use crate::layout::{footprint, volume};

        // footprint: x (i32) | y (i32) | spaceId (u32) — three 4-byte columns.
        assert_eq!(footprint::DOMAIN_TYPE, "Footprint");
        assert_eq!(footprint::CAPACITY, 1024);
        assert_eq!(footprint::ELEMENT_STRIDE, 12); // 3 columns * 4 bytes
        assert_eq!(footprint::BUFFER_BYTES, footprint::CAPACITY * 12);
        // Columns are CAPACITY contiguous elements each, back to back from base.
        assert_eq!(footprint::X_OFFSET, 0);
        assert_eq!(footprint::Y_OFFSET, footprint::CAPACITY * 4);
        assert_eq!(footprint::SPACE_ID_OFFSET, footprint::CAPACITY * 4 * 2);

        // volume: volumeId (u32) | spaceId (u32) | height (i32) — three 4-byte columns.
        assert_eq!(volume::DOMAIN_TYPE, "Volume");
        assert_eq!(volume::CAPACITY, 256);
        assert_eq!(volume::ELEMENT_STRIDE, 12);
        assert_eq!(volume::BUFFER_BYTES, volume::CAPACITY * 12);
        assert_eq!(volume::VOLUME_ID_OFFSET, 0);
        assert_eq!(volume::SPACE_ID_OFFSET, volume::CAPACITY * 4);
        assert_eq!(volume::HEIGHT_OFFSET, volume::CAPACITY * 4 * 2);
    }

    #[test]
    fn vertical_studs_extend_in_z_plates_in_x() {
        // Decode the first stud and the first plate straight out of the buffer's column bytes.
        let mut s = Session::new();
        s.apply(draw_10ft_wall());
        let bytes = s.buffer_bytes();
        let read = |off: usize, i: usize| {
            i32::from_le_bytes(bytes[off + i * 4..off + i * 4 + 4].try_into().unwrap())
        };
        let read_role = |i: usize| {
            u32::from_le_bytes(
                bytes[layout::ROLE_ID_OFFSET + i * 4..layout::ROLE_ID_OFFSET + i * 4 + 4]
                    .try_into()
                    .unwrap(),
            )
        };

        let plate_id = layout::role_id("plate").unwrap();
        let stud_id = layout::role_id("stud").unwrap();
        let n = s.member_count();
        let mut saw_plate = false;
        let mut saw_stud = false;
        for i in 0..n {
            let role = read_role(i);
            let (x0, z0, x1, z1) = (
                read(layout::X0_OFFSET, i),
                read(layout::Z0_OFFSET, i),
                read(layout::X1_OFFSET, i),
                read(layout::Z1_OFFSET, i),
            );
            if role == plate_id {
                saw_plate = true;
                assert!(x1 > x0 && z1 == z0); // runs along the baseline
            }
            if role == stud_id {
                saw_stud = true;
                assert!(z1 > z0 && x1 == x0); // extends up the wall
            }
        }
        assert!(saw_plate && saw_stud);
    }
}
