//! [`Sheet`] + [`DrawingSet`] — the composed, permittable deliverable.
//!
//! A sheet places projected [`DrawingView`]s + [`Dimension`]s + a [`TitleBlock`] at a paper size; a
//! drawing set is the ordered collection of sheets organized per the National CAD Standard. These
//! are the only two entities in the layer.

use crate::domain::annotation::{Dimension, TitleBlock};
use crate::domain::view::DrawingView;
use crate::keys::{DrawingSetId, SheetId};
use reference_data::CitationKey;

/// Standard paper sizes (a subset of ANSI / Arch series).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PaperSize {
    /// ANSI D (22×34).
    AnsiD,
    /// Arch D (24×36).
    ArchD,
    /// Arch E (36×48).
    ArchE,
}

/// One composed drawing sheet: placed views + dimensions + a title block at a paper size.
#[derive(Clone, PartialEq, Debug)]
pub struct Sheet {
    /// Stable identity.
    pub id: SheetId,
    /// Paper size; `None` until laid out.
    pub size: Option<PaperSize>,
    /// Placed views.
    pub views: Vec<DrawingView>,
    /// Dimensions annotating the views.
    pub dims: Vec<Dimension>,
    /// The sheet metadata block.
    pub title_block: TitleBlock,
}

impl Sheet {
    /// An empty sheet with a title block, no views/dims/size yet.
    pub fn new(id: SheetId, title_block: TitleBlock) -> Sheet {
        Sheet {
            id,
            size: None,
            views: Vec::new(),
            dims: Vec::new(),
            title_block,
        }
    }
}

/// The full permittable set: ordered sheets, organized per NCS. The deliverable artifact.
#[derive(Clone, PartialEq, Debug)]
pub struct DrawingSet {
    /// Stable identity.
    pub id: DrawingSetId,
    /// Ordered sheets.
    pub sheets: Vec<Sheet>,
    /// → the National CAD Standard the set conforms to.
    pub standard: Option<CitationKey>,
}

impl DrawingSet {
    /// An empty drawing set.
    pub fn new(id: DrawingSetId) -> DrawingSet {
        DrawingSet {
            id,
            sheets: Vec::new(),
            standard: None,
        }
    }

    /// Number of sheets in the set.
    pub fn sheet_count(&self) -> usize {
        self.sheets.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::ProjectRef;

    #[test]
    fn a_sheet_holds_a_title_block() {
        let sheet = Sheet::new(SheetId(1), TitleBlock::new("A-101", ProjectRef(1)));
        assert_eq!(sheet.title_block.sheet_no, "A-101");
        assert!(sheet.views.is_empty());

        let mut set = DrawingSet::new(DrawingSetId(1));
        set.sheets.push(sheet);
        assert_eq!(set.sheet_count(), 1);
    }
}
