// SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
// SPDX-License-Identifier: MPL-2.0

//! Ordered collections of structured records.

use super::record::Record;
use super::uuidv7_id;

uuidv7_id!(CollectionId, "A unique collection identifier.");

/// An ordered set of records.
#[derive(Debug, Clone)]
pub struct Collection {
    id: CollectionId,
    name: Box<str>,
}

impl Collection {
    /// Creates an empty collection with a new ID.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            id: CollectionId::new(),
            name: name.into(),
        }
    }

    /// Returns the collection ID.
    #[must_use]
    pub const fn id(&self) -> CollectionId {
        self.id
    }

    /// Returns the collection name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the collection name.
    pub fn set_name(&mut self, name: &str) {
        self.name = name.into();
    }

    /// Creates a new record owned by this collection.
    ///
    /// # Errors
    ///
    /// Returns an error if the label is empty after trimming.
    pub fn new_record(&self, label: &str) -> Result<Record, super::record::RecordError> {
        Record::new(self, label)
    }

    /// Creates a collection from the given ID and name.
    #[must_use]
    pub(crate) fn restore(id: CollectionId, name: &str) -> Self {
        Self {
            id,
            name: name.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collections_have_unique_identity_and_mutable_names() {
        let mut first = Collection::new("");
        let second = Collection::new("");

        assert_ne!(first.id(), second.id());
        assert_eq!(first.name(), "");

        first.set_name("Hosts");
        assert_eq!(first.name(), "Hosts");
    }

    #[test]
    fn records_inherit_collection_identity() {
        let collection = Collection::new("Hosts");
        let first = collection.new_record("Alpha").unwrap();
        let second = collection.new_record("Beta").unwrap();

        assert_eq!(first.collection_id(), collection.id());
        assert_eq!(second.collection_id(), collection.id());
        assert_ne!(first.id(), second.id());
    }

    #[test]
    fn restore_preserves_collection_data() {
        let id = CollectionId::new();
        let collection = Collection::restore(id, "Hosts");

        assert_eq!(collection.id(), id);
        assert_eq!(collection.name(), "Hosts");
    }
}
