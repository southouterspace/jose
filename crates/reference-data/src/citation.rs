//! Edition-aware pointers into the reference library.

/// An opaque, edition-aware pointer into the reference library — the shared target for
/// every `sourceRef` / `citation` across the schema.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct CitationKey {
    /// Library book identifier (required).
    pub book: String,
    /// Anchor within the book (section, table, figure).
    pub anchor: Option<String>,
    /// Edition the citation is pinned to.
    pub edition: Option<String>,
    /// Free-text note.
    pub note: Option<String>,
}

impl CitationKey {
    /// A bare citation to a book with no anchor/edition/note.
    pub fn book(book: impl Into<String>) -> CitationKey {
        CitationKey {
            book: book.into(),
            anchor: None,
            edition: None,
            note: None,
        }
    }

    /// Set the anchor (builder style).
    pub fn at(mut self, anchor: impl Into<String>) -> CitationKey {
        self.anchor = Some(anchor.into());
        self
    }

    /// Set the edition (builder style).
    pub fn edition(mut self, edition: impl Into<String>) -> CitationKey {
        self.edition = Some(edition.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_sets_fields() {
        let c = CitationKey::book("NDS").at("Table 4A").edition("2018");
        assert_eq!(c.book, "NDS");
        assert_eq!(c.anchor.as_deref(), Some("Table 4A"));
        assert_eq!(c.edition.as_deref(), Some("2018"));
        assert_eq!(c.note, None);
    }
}
