// SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
// SPDX-License-Identifier: MPL-2.0

//! Structured records belonging to collections.

use thiserror::Error;

use super::collection::{Collection, CollectionId};
use super::field::{Field, FieldDef, FieldType};
use super::note::NoteId;
use super::uuidv7_id;

uuidv7_id!(RecordId, "A unique record identifier.");

/// An error creating or changing a record.
#[derive(Debug, Error)]
pub enum RecordError {
    /// The record label is empty after trimming.
    #[error("Record label must not be empty")]
    EmptyLabel,
}

/// A set of field values owned by a collection.
#[derive(Debug, Clone)]
pub struct Record {
    id: RecordId,
    collection_id: CollectionId,
    label: Box<str>,
    source_note_id: Option<NoteId>,
}

impl Record {
    /// Creates a labeled record with a new ID and no fields.
    ///
    /// # Errors
    ///
    /// Returns an error if the label is empty after trimming.
    #[allow(clippy::new_without_default)]
    pub(crate) fn new(collection: &Collection, label: &str) -> Result<Self, RecordError> {
        let label = valid_label(label)?;
        Ok(Self {
            id: RecordId::new(),
            collection_id: collection.id(),
            label: label.into(),
            source_note_id: None,
        })
    }

    /// Returns the record ID.
    #[must_use]
    pub const fn id(&self) -> RecordId {
        self.id
    }

    /// Returns the record's collection ID.
    #[must_use]
    pub const fn collection_id(&self) -> CollectionId {
        self.collection_id
    }

    /// Returns the record's display label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Sets the record's display label.
    ///
    /// # Errors
    ///
    /// Returns an error if the label is empty after trimming.
    pub fn set_label(&mut self, label: &str) -> Result<(), RecordError> {
        self.label = valid_label(label)?.into();
        Ok(())
    }

    /// Returns the note this record was structured from, if any.
    #[must_use]
    pub const fn source_note_id(&self) -> Option<NoteId> {
        self.source_note_id
    }

    /// Sets the note this record was structured from.
    pub const fn set_source_note_id(&mut self, source_note_id: Option<NoteId>) {
        self.source_note_id = source_note_id;
    }

    /// Creates a field for this record using the given definition and value.
    #[must_use]
    pub fn new_field<T: FieldType>(&self, def: &FieldDef<T>, value: T::Value) -> Field<T> {
        Field::new(def, self, value)
    }

    pub(crate) fn restore(
        id: RecordId,
        collection_id: CollectionId,
        label: &str,
        source_note_id: Option<NoteId>,
    ) -> Result<Self, RecordError> {
        Ok(Self {
            id,
            collection_id,
            label: valid_label(label)?.into(),
            source_note_id,
        })
    }
}

fn valid_label(label: &str) -> Result<&str, RecordError> {
    let label = label.trim();
    if label.is_empty() {
        Err(RecordError::EmptyLabel)
    } else {
        Ok(label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Text;

    #[test]
    fn fields_inherit_record_and_definition_identity() {
        let collection = Collection::new("Hosts");
        let record = collection.new_record("Alpha").unwrap();
        let def = Text::def("Hostname");
        let field = record.new_field(&def, String::from("alpha-01"));

        assert_eq!(field.record_id(), record.id());
        assert_eq!(field.def_id(), def.id());
        assert_eq!(field.val(), "alpha-01");
    }

    #[test]
    fn restore_preserves_record_identity_and_owner() {
        let collection_id = CollectionId::new();
        let record_id = RecordId::new();
        let source_note_id = NoteId::new();
        let record =
            Record::restore(record_id, collection_id, "Alpha", Some(source_note_id)).unwrap();

        assert_eq!(record.collection_id(), collection_id);
        assert_eq!(record.id(), record_id);
        assert_eq!(record.label(), "Alpha");
        assert_eq!(record.source_note_id(), Some(source_note_id));
    }

    #[test]
    fn labels_are_trimmed_and_required() {
        let collection = Collection::new("Hosts");
        let mut record = collection.new_record(" Alpha ").unwrap();

        assert_eq!(record.label(), "Alpha");
        assert!(matches!(
            collection.new_record("  "),
            Err(RecordError::EmptyLabel)
        ));
        assert!(record.set_label("\n").is_err());
    }
}
