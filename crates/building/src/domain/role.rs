//! [`FramingRole`] — the closed vocabulary of framing-member roles.
//!
//! The role a member plays (stud, plate, header, …) is the connective tissue between three
//! contexts: the [`FramingSolver`](crate::FramingSolver) *mints* it, the render mirror *encodes* it
//! into the SoA `roleId` column, and the loads layer *maps* it to a structural limit-state class.
//! Modeling it as a closed enum — instead of a bare string matched ad hoc in each consumer — means
//! adding a role is one edit the compiler chases to every use site, the buffer id is derived here
//! rather than looked up with a silent fallback, and the loads classifier can no longer reference a
//! string the framer never emits.
//!
//! The discriminant **is** the `roleId` stored in the generated `MemberPlacement` buffer: the
//! variant order matches `schema/model/buffer-layouts.json`'s `roles` array exactly. A `bim-core`
//! guard test asserts the enum and the generated table never drift.

/// One framing member's role. The variant order is the canonical `roleId` order — keep it in lock
/// step with the `roles` array in `schema/model/buffer-layouts.json` (the guard test enforces it).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(u32)]
pub enum FramingRole {
    /// Top/bottom plate or sill plate — runs flat along the wall.
    Plate = 0,
    /// Full-height stud on the OC grid.
    Stud = 1,
    /// King stud at an opening edge (full height).
    King = 2,
    /// Jack (trimmer) stud carrying a header.
    Jack = 3,
    /// Cripple stud above/below an opening.
    Cripple = 4,
    /// Header spanning an opening.
    Header = 5,
    /// Window sill plate.
    Sill = 6,
    /// Shared corner/intersection post.
    Post = 7,
    /// Discrete blocking.
    Block = 8,
    /// Floor/ceiling joist.
    Joist = 9,
    /// Roof rafter.
    Rafter = 10,
    /// Truss chord.
    Chord = 11,
    /// Sheathing panel.
    Panel = 12,
}

impl FramingRole {
    /// Every role, in `roleId` order — the single list the framer, the buffer vocabulary, and the
    /// loads classifier all agree on.
    pub const ALL: [FramingRole; 13] = [
        FramingRole::Plate,
        FramingRole::Stud,
        FramingRole::King,
        FramingRole::Jack,
        FramingRole::Cripple,
        FramingRole::Header,
        FramingRole::Sill,
        FramingRole::Post,
        FramingRole::Block,
        FramingRole::Joist,
        FramingRole::Rafter,
        FramingRole::Chord,
        FramingRole::Panel,
    ];

    /// The stable id stored in the SoA `roleId` column — the enum discriminant, no lookup and no
    /// fallback. Matches the generated `roleId` vocabulary by construction.
    pub fn id(self) -> u32 {
        self as u32
    }

    /// The lowercase wire string (buffer vocabulary, drawings, debug).
    pub fn as_str(self) -> &'static str {
        match self {
            FramingRole::Plate => "plate",
            FramingRole::Stud => "stud",
            FramingRole::King => "king",
            FramingRole::Jack => "jack",
            FramingRole::Cripple => "cripple",
            FramingRole::Header => "header",
            FramingRole::Sill => "sill",
            FramingRole::Post => "post",
            FramingRole::Block => "block",
            FramingRole::Joist => "joist",
            FramingRole::Rafter => "rafter",
            FramingRole::Chord => "chord",
            FramingRole::Panel => "panel",
        }
    }

    /// Whether this member runs vertically (extends up the wall in +Z). Studs and the post family
    /// stand; plates, headers, sills, and sheathing lie flat. The render mirror derives a member's
    /// segment end from this.
    pub fn is_vertical(self) -> bool {
        matches!(
            self,
            FramingRole::Stud
                | FramingRole::King
                | FramingRole::Jack
                | FramingRole::Cripple
                | FramingRole::Post
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_is_the_discriminant_and_indexes_all() {
        for (i, role) in FramingRole::ALL.iter().enumerate() {
            assert_eq!(role.id() as usize, i);
        }
    }

    #[test]
    fn vertical_roles_stand_others_lie_flat() {
        assert!(FramingRole::Stud.is_vertical());
        assert!(FramingRole::Post.is_vertical());
        assert!(!FramingRole::Plate.is_vertical());
        assert!(!FramingRole::Header.is_vertical());
    }
}
