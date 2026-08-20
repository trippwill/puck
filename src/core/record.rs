// SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
// SPDX-License-Identifier: MPL-2.0

//! Structured records belonging to collections.

use super::collection::prelude::*;
use super::field::prelude::*;
use super::uuidv7_id;

/// Record types.
pub mod prelude {
    pub use super::{Record, RecordId};
}

uuidv7_id!(RecordId, "A unique record identifier.");

/// A set of field values owned by a collection.
#[derive(Debug)]
pub struct Record {
    id: RecordId,
    collection_id: CollectionId,
}

impl Record {
    /// Creates a record with a new ID and no fields.
    #[must_use]
    #[allow(clippy::new_without_default)]
    pub(crate) fn new(collection: &Collection) -> Self {
        Self {
            id: RecordId::new(),
            collection_id: collection.id(),
        }
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

    /// Creates a field for this record using the given definition and value.
    #[must_use]
    pub fn new_field<T: FieldType>(&self, def: &FieldDef<T>, value: T::Value) -> Field<T> {
        Field::new(def, self, value)
    }

    #[must_use]
    pub(crate) fn restore(id: RecordId, collection_id: CollectionId) -> Self {
        Self { id, collection_id }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_inherit_record_and_definition_identity() {
        let collection = Collection::new("Hosts");
        let record = collection.new_record();
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
        let record = Record::restore(record_id, collection_id);

        assert_eq!(record.collection_id(), collection_id);
        assert_eq!(record.id(), record_id);
    }
}
