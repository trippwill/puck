use indexmap::IndexSet;
use thiserror::Error;

use super::record;
use crate::core::uuidv7_id;

uuidv7_id!(CollectionId);

/// An error changing collection membership.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CollectionError {
    /// A record belongs to a different schema.
    #[error("Record uses schema {actual}, but collection requires schema {expected}")]
    IncompatibleSchema { expected: record::RecordSchemaId, actual: record::RecordSchemaId },
}

/// An ordered set of records sharing one schema.
pub struct Collection {
    id: CollectionId,
    name: String,
    schema_id: record::RecordSchemaId,
    records: IndexSet<record::RecordId>,
}

impl Collection {
    /// Creates an empty collection with a new ID.
    #[must_use]
    pub fn new(name: impl Into<String>, schema_id: record::RecordSchemaId) -> Self {
        Self { id: CollectionId::new(), name: name.into(), schema_id, records: IndexSet::new() }
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

    /// Returns the required record schema.
    #[must_use]
    pub const fn schema_id(&self) -> record::RecordSchemaId {
        self.schema_id
    }

    /// Inserts a record into the collection.
    ///
    /// # Errors
    ///
    /// Returns an error if the record's schema does not match the collection's schema.
    pub fn insert(&mut self, record: &record::Record) -> Result<(), CollectionError> {
        if record.schema_id() != self.schema_id {
            return Err(CollectionError::IncompatibleSchema {
                expected: self.schema_id,
                actual: record.schema_id(),
            });
        }

        self.records.insert(record.id());
        Ok(())
    }

    /// Removes a record, returning whether it was present.
    pub fn remove(&mut self, record: &record::Record) -> bool {
        self.records.shift_remove(&record.id())
    }

    /// Returns whether the record is present.
    #[must_use]
    pub fn contains(&self, record: &record::Record) -> bool {
        self.records.contains(&record.id())
    }

    /// Iterates over record IDs in insertion order.
    pub fn records(&self) -> impl Iterator<Item = &record::RecordId> {
        self.records.iter()
    }

    /// Returns the number of records.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Returns whether the collection contains no records.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Removes all records.
    pub fn clear(&mut self) {
        self.records.clear();
    }

    #[allow(dead_code)]
    pub(crate) fn restore(
        id: CollectionId,
        name: String,
        schema_id: record::RecordSchemaId,
        records: IndexSet<record::RecordId>,
    ) -> Self {
        Self { id, name, schema_id, records }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collection_manages_membership_in_insertion_order() {
        let schema = record::RecordSchema::new("Machine", None, indexmap::indexset! {});
        let first = schema.record(std::iter::empty()).unwrap();
        let second = schema.record(std::iter::empty()).unwrap();
        let mut collection = Collection::new("Machines", schema.id());

        assert!(collection.is_empty());
        collection.insert(&first).unwrap();
        collection.insert(&second).unwrap();
        collection.insert(&first).unwrap();

        assert_eq!(collection.len(), 2);
        assert!(collection.contains(&first));
        assert_eq!(collection.records().copied().collect::<Vec<_>>(), [first.id(), second.id()]);
        assert!(collection.remove(&first));
        assert!(!collection.remove(&first));
        assert!(!collection.contains(&first));

        collection.clear();
        assert!(collection.is_empty());
    }

    #[test]
    fn collection_rejects_an_incompatible_schema_without_mutation() {
        let expected = record::RecordSchema::new("Expected", None, indexmap::indexset! {});
        let actual = record::RecordSchema::new("Actual", None, indexmap::indexset! {});
        let record = actual.record(std::iter::empty()).unwrap();
        let mut collection = Collection::new("Records", expected.id());

        assert_eq!(
            collection.insert(&record).unwrap_err(),
            CollectionError::IncompatibleSchema { expected: expected.id(), actual: actual.id() }
        );
        assert!(collection.is_empty());
    }
}
