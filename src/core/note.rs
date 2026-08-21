// SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
// SPDX-License-Identifier: MPL-2.0

//! Immutable revisions of free-form notes.

use thiserror::Error;
use time::Timestamp;

/// Note types.
pub mod prelude {
    pub use super::{
        Archive,
        ArchiveNote,
        MAX_PREVIEW_CHARS,
        Note,
        NoteError,
        NoteId,
        NoteState,
        NoteSummary,
        Pile,
        PileNote,
    };
}

use crate::core::uuidv7_id;

/// The maximum number of characters in a pile-note preview.
pub const MAX_PREVIEW_CHARS: usize = 72;

uuidv7_id!(NoteId, "A unique note identifier.");

/// An error creating, restoring, or editing a pile note.
#[derive(Debug, Error)]
pub enum NoteError {
    /// A restored revision is zero.
    #[error("Note revision must be greater than zero")]
    InvalidRevision,

    /// Editing would overflow the revision counter.
    #[error("Note revision counter overflow")]
    RevisionOverflow,

    /// The updated timestamp precedes the creation timestamp.
    #[error("Invalid timestamp")]
    InvalidTimestamp,
}

mod sealed {
    pub trait Sealed {}
}

/// A note available in the active pile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pile;

/// A note retained outside the active pile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Archive;

/// A sealed lifecycle state for a note.
pub trait NoteState: self::sealed::Sealed + Clone + PartialEq + Eq {}

impl NoteState for Pile {}
impl NoteState for Archive {}
impl self::sealed::Sealed for Pile {}
impl self::sealed::Sealed for Archive {}

/// An immutable revision of a free-form note in the pile.
pub type PileNote = Note<Pile>;
/// An immutable revision of a free-form note in the archive.
pub type ArchiveNote = Note<Archive>;

/// An immutable revision of a free-form note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note<T: NoteState> {
    id: NoteId,
    body: String,
    revision: u32,
    created_at: Timestamp,
    updated_at: Timestamp,
    _marker: std::marker::PhantomData<T>,
}

impl Note<Pile> {
    /// Creates a new note with the given body.
    #[must_use]
    pub fn create(body: impl Into<String>) -> Self {
        let now = now();
        Self {
            id: NoteId::new(),
            body: body.into(),
            revision: 1,
            created_at: now,
            updated_at: now,
            _marker: std::marker::PhantomData,
        }
    }

    /// Edits the note with a new body.
    ///
    /// # Errors
    ///
    /// Returns an error if the revision counter overflows.
    /// See [`PileNote::recover_revision_overflow`] for a way to recover from this error.
    pub fn edit(&self, body: impl Into<String>) -> Result<Self, NoteError> {
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(NoteError::RevisionOverflow)?;
        let updated_at = now();
        Ok(Self {
            id: self.id,
            body: body.into(),
            revision,
            created_at: self.created_at,
            updated_at,
            _marker: std::marker::PhantomData,
        })
    }

    /// Recovers an uneditable note as a new identity at revision one.
    #[must_use]
    pub fn recover_revision_overflow(&self) -> Self {
        let updated_at = now();
        Self {
            id: NoteId::new(),
            body: self.body.clone(),
            revision: 1,
            created_at: self.created_at,
            updated_at,
            _marker: std::marker::PhantomData,
        }
    }

    /// Moves the note out of the active pile.
    #[must_use]
    pub fn archive(self) -> ArchiveNote {
        Note {
            id: self.id,
            body: self.body.clone(),
            revision: self.revision,
            created_at: self.created_at,
            updated_at: self.updated_at,
            _marker: std::marker::PhantomData,
        }
    }
}

impl Note<Archive> {
    /// Returns the note to the active pile.
    #[must_use]
    pub fn unarchive(self) -> PileNote {
        Note {
            id: self.id,
            body: self.body.clone(),
            revision: self.revision,
            created_at: self.created_at,
            updated_at: self.updated_at,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T: NoteState> Note<T> {
    /// Restores a note from persisted data.
    ///
    /// # Errors
    ///
    /// Returns an error if the revision is zero, or the timestamps are invalid
    /// (e.g., `updated_at` is before `created_at`).
    #[allow(dead_code)]
    pub(crate) fn restore(
        id: NoteId,
        body: String,
        revision: u32,
        created_at: Timestamp,
        updated_at: Timestamp,
    ) -> Result<Self, NoteError> {
        if updated_at < created_at {
            return Err(NoteError::InvalidTimestamp);
        }
        match revision {
            0 => Err(NoteError::InvalidRevision),
            _ => Ok(Self {
                id,
                body,
                revision,
                created_at,
                updated_at,
                _marker: std::marker::PhantomData,
            }),
        }
    }

    /// Returns the note ID.
    #[must_use]
    pub fn id(&self) -> NoteId {
        self.id
    }

    /// Returns the full note body.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }

    /// Returns the revision number.
    #[must_use]
    pub fn revision(&self) -> u32 {
        self.revision
    }

    /// Returns when the note was created.
    #[must_use]
    pub fn created_at(&self) -> Timestamp {
        self.created_at
    }

    /// Returns when this revision was created.
    #[must_use]
    pub fn updated_at(&self) -> Timestamp {
        self.updated_at
    }
}

/// A compact note projection for lists and search results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteSummary {
    /// The source note ID.
    pub id: NoteId,
    /// The first line, truncated to [`MAX_PREVIEW_CHARS`] characters.
    pub preview: String,
    /// The source note revision.
    pub revision: u32,
    /// The source note's update timestamp.
    pub updated_at: Timestamp,
}

impl<T: NoteState> From<&Note<T>> for NoteSummary {
    fn from(note: &Note<T>) -> Self {
        let first_line = note.body().lines().next().unwrap_or_default();
        let mut preview: String = first_line.chars().take(MAX_PREVIEW_CHARS).collect();

        if first_line.chars().count() > MAX_PREVIEW_CHARS {
            preview.push('…');
        }

        Self {
            id: note.id(),
            preview,
            revision: note.revision(),
            updated_at: note.updated_at(),
        }
    }
}

fn now() -> Timestamp {
    let now = Timestamp::now();
    Timestamp::from_milliseconds(now.as_milliseconds())
        .expect("the current timestamp remains valid at millisecond precision")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Collection, CollectionId, FieldDefId, FieldType, RecordId, Text};

    #[test]
    fn edit_should_increment_revision() {
        let note = Note::create("Initial body");
        let updated_note = note.edit("Updated body").expect("valid edit");
        assert_eq!(updated_note.revision(), note.revision() + 1);
        assert_eq!(updated_note.body(), "Updated body");
        assert!(updated_note.updated_at() >= note.updated_at());
        assert_eq!(updated_note.created_at(), note.created_at());
        assert_eq!(updated_note.id(), note.id());
    }

    #[test]
    fn edit_revision_overflow_should_return_error() {
        let note = Note::create("Initial body");
        let mut updated_note = note.clone();
        updated_note.revision = u32::MAX;
        let result = updated_note.edit("Updated body");
        assert!(matches!(result, Err(NoteError::RevisionOverflow)));
    }

    #[test]
    fn restore_revision_zero_should_return_error() {
        let body = "Some body".to_string();
        let created_at = now();
        let updated_at = created_at;
        let result = PileNote::restore(NoteId::new(), body, 0, created_at, updated_at);
        assert!(matches!(result, Err(NoteError::InvalidRevision)));
    }

    #[test]
    fn restore_validates_timestamps_and_preserves_data() {
        let id = NoteId::new();
        let created_at = now();
        let updated_at = created_at + time::Duration::SECOND;

        assert!(matches!(
            ArchiveNote::restore(id, String::from("Body"), 2, updated_at, created_at),
            Err(NoteError::InvalidTimestamp)
        ));

        let note =
            ArchiveNote::restore(id, String::from("Body"), 2, created_at, updated_at).unwrap();
        assert_eq!(note.id(), id);
        assert_eq!(note.body(), "Body");
        assert_eq!(note.revision(), 2);
        assert_eq!(note.created_at(), created_at);
        assert_eq!(note.updated_at(), updated_at);
    }

    #[test]
    fn summary_uses_and_truncates_only_the_first_line() {
        assert!(NoteSummary::from(&Note::create("")).preview.is_empty());

        let short = Note::create(
            r#"Short title
            This second line is deliberately much longer than the preview limit and must not affect truncation"#,
        );
        assert_eq!(NoteSummary::from(&short).preview, "Short title");

        let exact = "x".repeat(MAX_PREVIEW_CHARS);
        assert_eq!(
            NoteSummary::from(&Note::create(exact.clone())).preview,
            exact
        );

        let body = "🦀".repeat(MAX_PREVIEW_CHARS + 1);
        let note = Note::create(body);
        let summary = NoteSummary::from(&note);
        assert_eq!(
            summary.preview,
            format!("{}…", "🦀".repeat(MAX_PREVIEW_CHARS))
        );
        assert_eq!(summary.preview.chars().count(), MAX_PREVIEW_CHARS + 1);
    }

    #[test]
    fn archive_and_unarchive_preserve_data() {
        let note = Note::create("Some body");
        let note_clone = note.clone();
        let archived = note.archive();
        assert_eq!(archived.id(), note_clone.id());
        assert_eq!(archived.body(), note_clone.body());
        assert_eq!(archived.revision(), note_clone.revision());
        assert_eq!(archived.created_at(), note_clone.created_at());
        assert_eq!(archived.updated_at(), note_clone.updated_at());

        let unarchived = archived.unarchive();
        assert_eq!(unarchived.id(), note_clone.id());
        assert_eq!(unarchived.body(), note_clone.body());
        assert_eq!(unarchived.revision(), note_clone.revision());
        assert_eq!(unarchived.created_at(), note_clone.created_at());
        assert_eq!(unarchived.updated_at(), note_clone.updated_at());
    }

    #[test]
    fn recover_revision_overflow_resets_revision() {
        let note = Note::create("Some body");
        let mut overflowed_note = note.clone();
        overflowed_note.revision = u32::MAX;
        let recovered_note = overflowed_note.recover_revision_overflow();
        assert_ne!(recovered_note.id(), overflowed_note.id());
        assert_eq!(recovered_note.revision(), 1);
        assert_eq!(recovered_note.body(), overflowed_note.body());
        assert_eq!(recovered_note.created_at(), overflowed_note.created_at());
        assert!(recovered_note.updated_at() >= overflowed_note.updated_at());
    }

    #[test]
    fn domain_ids_parse_from_their_display_form() {
        let note = Note::create("Some body");
        let collection = Collection::new("Hosts");
        let record = collection.new_record();
        let field_def = Text::def("Hostname");

        assert_eq!(note.id().to_string().parse::<NoteId>().unwrap(), note.id());
        assert_eq!(
            collection.id().to_string().parse::<CollectionId>().unwrap(),
            collection.id()
        );
        assert_eq!(
            record.id().to_string().parse::<RecordId>().unwrap(),
            record.id()
        );
        assert_eq!(
            field_def.id().to_string().parse::<FieldDefId>().unwrap(),
            field_def.id()
        );
    }
}
