//! The in-session composition root: holds canonical state and runs the **draw → recompute**
//! pipeline, wiring the building context (semantic promotion + the [`FramingSolver`]) into the SoA
//! [`MemberBuffer`] the render mirror reads.
//!
//! This is the one place the bounded contexts are composed. Today the slice is
//! `promote wall → frame members → write buffer`; loads-analysis and the design-standard check
//! slot in here as later pipeline stages without changing the boundary or the buffer contract.

use crate::buffer::{
    FootprintBuffer, FootprintRow, MemberBuffer, MemberRow, NOMINAL_WIDTH, VolumeBuffer, VolumeRow,
};
use crate::command::{Command, DrawFootprint, DrawWall, PushPull};
use building::{
    AssemblyKind, FramingSolver, MemberPlacement, RuleSet, SpacingAnchor, SpacingKey,
    SpacingModule, Wall, WallRole,
};
use building::{FaceRef, WallId};
use geometry_kernel::{
    EntityId, GeometryKernel, Path2D, Plane, PushPullMode, PushPullOp, Segment, TICKS_PER_FOOT,
    TOP_FACE, Tick, TickVec2, TickVec3, UnitVec3, Volume,
};
use materials::SpecKey;

/// Nominal framed wall thickness — a 2x4 wall = 3.5in = 112 ticks.
const WALL_THICKNESS: i32 = 112;

/// The default mass height a fresh footprint is extruded to: 8 ft.
const DEFAULT_HEIGHT: i32 = 8 * TICKS_PER_FOOT;

/// The single space/volume id for the MVP (one space at a time, like DrawWall's single wall).
const SPACE_ID: u32 = 1;
const VOLUME_ID: u32 = 1;

/// The canonical in-session model. Owns the recompute state (the framer's id counter) and the SoA
/// buffer; a `Command` in, a rewritten buffer out.
#[derive(Clone, Debug)]
pub struct Session {
    framer: FramingSolver,
    buffer: MemberBuffer,
    kernel: GeometryKernel,
    footprint_buffer: FootprintBuffer,
    volume_buffer: VolumeBuffer,
    /// The current space's footprint ring (world-XY ticks); `None` until a footprint is drawn.
    footprint: Vec<(i32, i32)>,
    /// The current extruded mass; `None` until a footprint is drawn.
    volume: Option<Volume>,
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
            kernel: GeometryKernel::new(),
            footprint_buffer: FootprintBuffer::new(),
            volume_buffer: VolumeBuffer::new(),
            footprint: Vec::new(),
            volume: None,
        }
    }

    /// The canonical SoA bytes — the read-only render mirror handed to JS.
    pub fn buffer_bytes(&self) -> &[u8] {
        self.buffer.as_bytes()
    }

    /// The footprint SoA bytes — the plan view's read-only render mirror.
    pub fn footprint_bytes(&self) -> &[u8] {
        self.footprint_buffer.as_bytes()
    }

    /// The volume SoA bytes — the 3D view reads these alongside the footprint.
    pub fn volume_bytes(&self) -> &[u8] {
        self.volume_buffer.as_bytes()
    }

    /// Number of live member rows in the buffer.
    pub fn member_count(&self) -> usize {
        self.buffer.len()
    }

    /// Number of live footprint vertex rows.
    pub fn footprint_count(&self) -> usize {
        self.footprint_buffer.len()
    }

    /// Number of live volume rows.
    pub fn volume_count(&self) -> usize {
        self.volume_buffer.len()
    }

    /// Apply one command and recompute, returning the new live member count.
    pub fn apply(&mut self, command: Command) -> usize {
        match command {
            Command::DrawWall(draw) => self.draw_wall(draw),
            Command::DrawFootprint(draw) => self.draw_footprint(draw),
            Command::PushPull(op) => self.push_pull(op),
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

    /// Draw (or redraw) the current space's footprint: store the closed ring, extrude it +Z from
    /// the ground plane to [`DEFAULT_HEIGHT`], and rewrite the footprint + volume buffers. Replaces
    /// any prior space (one space at a time, mirroring DrawWall's redraw-replaces behavior). A
    /// degenerate ring the kernel refuses to extrude leaves the volume `None` and its buffer empty.
    fn draw_footprint(&mut self, draw: DrawFootprint) -> usize {
        self.footprint = draw.vertices;
        let profile = Path2D::closed(
            self.footprint
                .iter()
                .map(|&(x, y)| TickVec2::new(Tick(x), Tick(y)))
                .collect(),
        );
        self.volume = self.kernel.extrude(
            EntityId(u128::from(VOLUME_ID)),
            profile,
            Plane::xy(TickVec3::ZERO),
            UnitVec3::Z,
            Tick(DEFAULT_HEIGHT),
        );
        self.rewrite_space_buffers();
        self.buffer.len()
    }

    /// Push/pull the current volume's top cap by a signed tick distance. Rejects a non-top
    /// `face_index` before touching the kernel; on a kernel `None` (non-top face, or height would
    /// go <= 0) leaves state unchanged. On success rewrites the volume buffer.
    fn push_pull(&mut self, op: PushPull) -> usize {
        if op.face_index != TOP_FACE {
            return self.buffer.len();
        }
        let Some(volume) = self.volume.as_mut() else {
            return self.buffer.len();
        };
        let (mode, magnitude) = if op.distance >= 0 {
            (PushPullMode::Extrude, op.distance)
        } else {
            (PushPullMode::Inset, -op.distance)
        };
        let applied = self.kernel.apply_push_pull(
            volume,
            PushPullOp {
                target_face_volume: EntityId(u128::from(op.volume_id)),
                target_face_index: op.face_index,
                distance: Tick(magnitude),
                mode,
            },
        );
        if applied.is_some() {
            self.rewrite_space_buffers();
        }
        self.buffer.len()
    }

    /// Rewrite the footprint + volume buffers from the current `footprint` ring and `volume`.
    fn rewrite_space_buffers(&mut self) {
        self.footprint_buffer.clear();
        self.volume_buffer.clear();
        let Some(volume) = self.volume.as_ref() else {
            return;
        };
        for &(x, y) in &self.footprint {
            self.footprint_buffer.push(FootprintRow {
                x,
                y,
                space_id: SPACE_ID,
            });
        }
        self.volume_buffer.push(VolumeRow {
            volume_id: VOLUME_ID,
            space_id: SPACE_ID,
            height: volume.height.raw(),
        });
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

    fn square_ring() -> DrawFootprint {
        let ft = 384; // 1ft in ticks
        DrawFootprint {
            vertices: vec![(0, 0), (10 * ft, 0), (10 * ft, 12 * ft), (0, 12 * ft)],
        }
    }

    fn read_i32(bytes: &[u8], off: usize, i: usize) -> i32 {
        i32::from_le_bytes(bytes[off + i * 4..off + i * 4 + 4].try_into().unwrap())
    }
    fn read_u32(bytes: &[u8], off: usize, i: usize) -> u32 {
        u32::from_le_bytes(bytes[off + i * 4..off + i * 4 + 4].try_into().unwrap())
    }

    #[test]
    fn draw_footprint_populates_footprint_and_volume() {
        use crate::layout::{footprint as fp, volume as vol};
        let mut s = Session::new();
        s.apply(Command::DrawFootprint(square_ring()));

        // footprint: 4 live vertex rows, decoded straight from the bytes.
        assert_eq!(s.footprint_count(), 4);
        let fbytes = s.footprint_bytes();
        assert_eq!(fbytes.len(), fp::BUFFER_BYTES);
        assert_eq!(read_i32(fbytes, fp::X_OFFSET, 0), 0);
        assert_eq!(read_i32(fbytes, fp::Y_OFFSET, 0), 0);
        assert_eq!(read_i32(fbytes, fp::X_OFFSET, 1), 10 * 384);
        assert_eq!(read_i32(fbytes, fp::Y_OFFSET, 2), 12 * 384);
        for i in 0..4 {
            assert_eq!(read_u32(fbytes, fp::SPACE_ID_OFFSET, i), 1);
        }

        // volume: exactly one row with the default height, volumeId 1, spaceId 1.
        assert_eq!(s.volume_count(), 1);
        let vbytes = s.volume_bytes();
        assert_eq!(vbytes.len(), vol::BUFFER_BYTES);
        assert_eq!(read_u32(vbytes, vol::VOLUME_ID_OFFSET, 0), 1);
        assert_eq!(read_u32(vbytes, vol::SPACE_ID_OFFSET, 0), 1);
        assert_eq!(read_i32(vbytes, vol::HEIGHT_OFFSET, 0), DEFAULT_HEIGHT);
    }

    #[test]
    fn push_pull_extrude_increases_height() {
        use crate::layout::volume as vol;
        let mut s = Session::new();
        s.apply(Command::DrawFootprint(square_ring()));
        let n = 2 * 384; // +2ft
        s.apply(Command::PushPull(PushPull {
            volume_id: 1,
            face_index: TOP_FACE,
            distance: n,
        }));
        assert_eq!(
            read_i32(s.volume_bytes(), vol::HEIGHT_OFFSET, 0),
            DEFAULT_HEIGHT + n
        );
    }

    #[test]
    fn push_pull_inset_and_floor() {
        use crate::layout::volume as vol;
        let mut s = Session::new();
        s.apply(Command::DrawFootprint(square_ring()));

        // A negative distance lowers the height.
        s.apply(Command::PushPull(PushPull {
            volume_id: 1,
            face_index: TOP_FACE,
            distance: -384, // -1ft
        }));
        assert_eq!(
            read_i32(s.volume_bytes(), vol::HEIGHT_OFFSET, 0),
            DEFAULT_HEIGHT - 384
        );
        let after_inset = read_i32(s.volume_bytes(), vol::HEIGHT_OFFSET, 0);

        // A distance that would drive height <= 0 is refused by the kernel; state unchanged.
        s.apply(Command::PushPull(PushPull {
            volume_id: 1,
            face_index: TOP_FACE,
            distance: -(after_inset + 1),
        }));
        assert_eq!(
            read_i32(s.volume_bytes(), vol::HEIGHT_OFFSET, 0),
            after_inset
        );
    }

    #[test]
    fn push_pull_rejects_non_top_face() {
        use crate::layout::volume as vol;
        use geometry_kernel::BASE_FACE;
        let mut s = Session::new();
        s.apply(Command::DrawFootprint(square_ring()));
        s.apply(Command::PushPull(PushPull {
            volume_id: 1,
            face_index: BASE_FACE, // not the top cap
            distance: 5 * 384,
        }));
        assert_eq!(
            read_i32(s.volume_bytes(), vol::HEIGHT_OFFSET, 0),
            DEFAULT_HEIGHT
        );
    }

    #[test]
    fn redraw_footprint_replaces() {
        let mut s = Session::new();
        s.apply(Command::DrawFootprint(square_ring()));
        assert_eq!(s.footprint_count(), 4);

        // A triangle (3 vertices) must replace, not append, the prior 4-vertex square.
        let ft = 384;
        s.apply(Command::DrawFootprint(DrawFootprint {
            vertices: vec![(0, 0), (8 * ft, 0), (4 * ft, 8 * ft)],
        }));
        assert_eq!(s.footprint_count(), 3);
        assert_eq!(s.volume_count(), 1);
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
