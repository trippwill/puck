use std::collections::HashMap;

use indexmap::IndexSet;
use thiserror::Error;

use super::field;
use crate::core::uuidv7_id;

uuidv7_id!(RecordSchemaId);
uuidv7_id!(RecordId);

/// An error creating a record from a schema.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RecordError {
    /// A supplied field is not listed in the schema.
    #[error("Field not allowed in this record schema: {0:?}")]
    FieldNotAllowed(field::FieldDescriptionId),
}

/// A named, ordered set of field descriptions accepted by records.
pub struct RecordSchema {
    id: RecordSchemaId,
    name: String,
    description: Option<String>,
    fields: IndexSet<field::FieldDescriptionId>,
}

impl RecordSchema {
    /// Creates a record schema with a new ID.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        description: Option<String>,
        fields: IndexSet<field::FieldDescriptionId>,
    ) -> Self {
        Self { id: RecordSchemaId::new(), name: name.into(), description, fields }
    }

    /// Creates a new record based on this schema.
    ///
    /// If multiple values use the same field description, the first value is retained.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the provided fields are not allowed in this schema.
    pub fn record(
        &self,
        fields: impl IntoIterator<Item = RecordField>,
    ) -> Result<Record, RecordError> {
        let mut values = HashMap::new();
        for field in fields {
            let description_id = field.description_id();
            if !self.fields.contains(&description_id) {
                return Err(RecordError::FieldNotAllowed(description_id));
            }

            values.entry(description_id).or_insert(field);
        }

        Ok(Record::new(self.id, values))
    }

    /// Returns the schema ID.
    #[must_use]
    pub const fn id(&self) -> RecordSchemaId {
        self.id
    }

    /// Returns the schema name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the optional schema description.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Iterates over allowed field descriptions in insertion order.
    pub fn fields(&self) -> impl Iterator<Item = &field::FieldDescriptionId> {
        self.fields.iter()
    }

    #[allow(dead_code)]
    pub(crate) fn restore(
        id: RecordSchemaId,
        name: String,
        description: Option<String>,
        fields: IndexSet<field::FieldDescriptionId>,
    ) -> Self {
        Self { id, name, description, fields }
    }
}

/// A field value stored in a record.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RecordField {
    /// A text value.
    Text(field::TextField),
    /// A Boolean value.
    Boolean(field::BooleanField),
    /// An integer value.
    Integer(field::IntegerField),
    /// A calendar-date value.
    Date(field::DateField),
    /// A wall-clock-time value.
    Time(field::TimeField),
}

impl RecordField {
    /// Returns the field-value ID.
    #[must_use]
    pub fn id(&self) -> field::FieldId {
        match self {
            RecordField::Text(field) => field.id(),
            RecordField::Boolean(field) => field.id(),
            RecordField::Integer(field) => field.id(),
            RecordField::Date(field) => field.id(),
            RecordField::Time(field) => field.id(),
        }
    }

    /// Returns the field-description ID.
    #[must_use]
    pub fn description_id(&self) -> field::FieldDescriptionId {
        match self {
            RecordField::Text(field) => field.description_id(),
            RecordField::Boolean(field) => field.description_id(),
            RecordField::Integer(field) => field.description_id(),
            RecordField::Date(field) => field.description_id(),
            RecordField::Time(field) => field.description_id(),
        }
    }

    /// Formats the contained value for display.
    #[must_use]
    pub fn value_as_string(&self) -> String {
        match self {
            RecordField::Text(field) => field.value().clone(),
            RecordField::Boolean(field) => field.value().to_string(),
            RecordField::Integer(field) => field.value().to_string(),
            RecordField::Date(field) => field.value().to_string(),
            RecordField::Time(field) => field.value().to_string(),
        }
    }
}

/// A set of typed field values created from a [`RecordSchema`].
pub struct Record {
    id: RecordId,
    schema_id: RecordSchemaId,
    fields: HashMap<field::FieldDescriptionId, RecordField>,
}

impl Record {
    /// Returns the record ID.
    #[must_use]
    fn new(
        schema_id: RecordSchemaId,
        fields: HashMap<field::FieldDescriptionId, RecordField>,
    ) -> Self {
        Self { id: RecordId::new(), schema_id, fields }
    }

    /// Returns the schema used to create this record.
    #[must_use]
    pub const fn id(&self) -> RecordId {
        self.id
    }

    /// Finds a field by its description.
    #[must_use]
    pub const fn schema_id(&self) -> RecordSchemaId {
        self.schema_id
    }

    /// Finds a field by its value ID.
    #[must_use]
    pub fn field_by_description(
        &self,
        field_description_id: field::FieldDescriptionId,
    ) -> Option<&RecordField> {
        self.fields.get(&field_description_id)
    }

    #[must_use]
    pub fn field_by_id(&self, field_id: field::FieldId) -> Option<&RecordField> {
        self.fields.values().find(|field| field.id() == field_id)
    }

    /// Iterates over the record's fields in unspecified order.
    pub fn fields(&self) -> impl Iterator<Item = &RecordField> {
        self.fields.values()
    }

    #[allow(dead_code)]
    pub(crate) fn restore(
        id: RecordId,
        schema_id: RecordSchemaId,
        fields: HashMap<field::FieldDescriptionId, RecordField>,
    ) -> Self {
        Self { id, schema_id, fields }
    }
}

impl From<field::TextField> for RecordField {
    fn from(field: field::TextField) -> Self {
        RecordField::Text(field)
    }
}
impl From<field::BooleanField> for RecordField {
    fn from(field: field::BooleanField) -> Self {
        RecordField::Boolean(field)
    }
}
impl From<field::IntegerField> for RecordField {
    fn from(field: field::IntegerField) -> Self {
        RecordField::Integer(field)
    }
}
impl From<field::DateField> for RecordField {
    fn from(field: field::DateField) -> Self {
        RecordField::Date(field)
    }
}
impl From<field::TimeField> for RecordField {
    fn from(field: field::TimeField) -> Self {
        RecordField::Time(field)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use indexmap::indexset;

    use super::*;

    #[test]
    fn schema_and_record_preserve_domain_data() {
        let greeting = field::TextFieldDescription::new("Greeting");
        let active = field::BooleanFieldDescription::new("Active");
        let schema = RecordSchema::new(
            "User",
            Some(String::from("A user record")),
            indexset! {greeting.id(), active.id()},
        );
        assert_eq!(schema.name(), "User");
        assert_eq!(schema.description(), Some("A user record"));
        assert_eq!(schema.fields().copied().collect::<Vec<_>>(), [greeting.id(), active.id()]);

        let greeting_value = greeting.value(String::from("Hello"));
        let greeting_value_id = greeting_value.id();
        let active_value = active.value(true);
        let active_value_id = active_value.id();
        let record = schema.record([greeting_value.into(), active_value.into()]).unwrap();

        assert_eq!(record.schema_id(), schema.id());
        assert_eq!(
            record.field_by_description(greeting.id()).map(RecordField::id),
            Some(greeting_value_id)
        );
        assert_eq!(
            record.field_by_id(active_value_id).map(RecordField::description_id),
            Some(active.id())
        );
        assert_eq!(
            record.fields().map(RecordField::description_id).collect::<HashSet<_>>(),
            HashSet::from([greeting.id(), active.id()])
        );
    }

    #[test]
    fn schema_description_can_be_omitted() {
        let schema = RecordSchema::new("User", None, indexset! {});

        assert_eq!(schema.description(), None);
    }

    #[test]
    fn record_rejects_fields_outside_schema() {
        let allowed = field::TextFieldDescription::new("Allowed");
        let unknown = field::TextFieldDescription::new("Unknown");
        let schema = RecordSchema::new("Schema", None, indexset! {allowed.id()});

        assert_eq!(
            schema.record([unknown.value(String::from("value")).into()]).err(),
            Some(RecordError::FieldNotAllowed(unknown.id()))
        );
    }

    #[test]
    fn duplicate_fields_keep_the_first_value() {
        let greeting = field::TextFieldDescription::new("Greeting");
        let schema = RecordSchema::new("Schema", None, indexset! {greeting.id()});

        let record = schema
            .record([
                greeting.value(String::from("First")).into(),
                greeting.value(String::from("Second")).into(),
            ])
            .unwrap();

        let Some(RecordField::Text(value)) = record.field_by_description(greeting.id()) else {
            panic!("greeting should be a text field");
        };
        assert_eq!(value.value(), "First");
        assert_eq!(record.fields().count(), 1);
    }
}
