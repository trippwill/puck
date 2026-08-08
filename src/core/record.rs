#![allow(dead_code)]

use thiserror::Error;
use time::OffsetDateTime;

pub struct RecordDescription {
    id: uuid::Uuid,
    name: String,
    fields: Vec<FieldDescription>,
}

impl RecordDescription {
    pub fn new(name: impl Into<String>, fields: Vec<FieldDescription>) -> Self {
        Self { id: uuid::Uuid::now_v7(), name: name.into(), fields }
    }

    pub(crate) fn restore(
        id: uuid::Uuid,
        name: impl Into<String>,
        fields: Vec<FieldDescription>,
    ) -> Self {
        Self { id, name: name.into(), fields }
    }

    pub const fn id(&self) -> uuid::Uuid {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn fields(&self) -> &[FieldDescription] {
        &self.fields
    }
}

struct FieldDescriptionData {
    id: uuid::Uuid,
    name: String,
}

pub struct TextFieldDescription(FieldDescriptionData);
pub struct NumberFieldDescription(FieldDescriptionData);
pub struct BooleanFieldDescription(FieldDescriptionData);
pub struct DateFieldDescription(FieldDescriptionData);

pub enum FieldDescription {
    Text(TextFieldDescription),
    Number(NumberFieldDescription),
    Boolean(BooleanFieldDescription),
    Date(DateFieldDescription),
}

struct FieldValueData<V> {
    id: uuid::Uuid,
    description_id: uuid::Uuid,
    value: V,
}

pub struct TextFieldValue(FieldValueData<String>);
pub struct NumberFieldValue(FieldValueData<f64>);
pub struct BooleanFieldValue(FieldValueData<bool>);
pub struct DateFieldValue(FieldValueData<OffsetDateTime>);

pub enum FieldValue {
    Text(TextFieldValue),
    Number(NumberFieldValue),
    Boolean(BooleanFieldValue),
    Date(DateFieldValue),
}

impl FieldValue {
    pub const fn id(&self) -> uuid::Uuid {
        match self {
            FieldValue::Text(value) => value.id(),
            FieldValue::Number(value) => value.id(),
            FieldValue::Boolean(value) => value.id(),
            FieldValue::Date(value) => value.id(),
        }
    }

    pub const fn description_id(&self) -> uuid::Uuid {
        match self {
            FieldValue::Text(value) => value.description_id(),
            FieldValue::Number(value) => value.description_id(),
            FieldValue::Boolean(value) => value.description_id(),
            FieldValue::Date(value) => value.description_id(),
        }
    }

    fn matches(&self, description: &FieldDescription) -> bool {
        matches!(
            (self, description),
            (Self::Text(_), FieldDescription::Text(_))
                | (Self::Number(_), FieldDescription::Number(_))
                | (Self::Boolean(_), FieldDescription::Boolean(_))
                | (Self::Date(_), FieldDescription::Date(_))
        )
    }
}

impl TextFieldValue {
    pub const fn id(&self) -> uuid::Uuid {
        self.0.id
    }

    pub const fn description_id(&self) -> uuid::Uuid {
        self.0.description_id
    }

    pub fn value(&self) -> &str {
        &self.0.value
    }

    pub(crate) fn restore(
        id: uuid::Uuid,
        description_id: uuid::Uuid,
        value: impl Into<String>,
    ) -> Self {
        Self(FieldValueData { id, description_id, value: value.into() })
    }
}

impl From<TextFieldValue> for FieldValue {
    fn from(value: TextFieldValue) -> Self {
        FieldValue::Text(value)
    }
}

impl NumberFieldValue {
    pub const fn id(&self) -> uuid::Uuid {
        self.0.id
    }

    pub const fn description_id(&self) -> uuid::Uuid {
        self.0.description_id
    }

    pub fn value(&self) -> f64 {
        self.0.value
    }

    pub(crate) fn restore(id: uuid::Uuid, description_id: uuid::Uuid, value: f64) -> Self {
        Self(FieldValueData { id, description_id, value })
    }
}

impl From<NumberFieldValue> for FieldValue {
    fn from(value: NumberFieldValue) -> Self {
        FieldValue::Number(value)
    }
}

impl BooleanFieldValue {
    pub const fn id(&self) -> uuid::Uuid {
        self.0.id
    }

    pub const fn description_id(&self) -> uuid::Uuid {
        self.0.description_id
    }

    pub fn value(&self) -> bool {
        self.0.value
    }

    pub(crate) fn restore(id: uuid::Uuid, description_id: uuid::Uuid, value: bool) -> Self {
        Self(FieldValueData { id, description_id, value })
    }
}

impl From<BooleanFieldValue> for FieldValue {
    fn from(value: BooleanFieldValue) -> Self {
        FieldValue::Boolean(value)
    }
}

impl DateFieldValue {
    pub const fn id(&self) -> uuid::Uuid {
        self.0.id
    }

    pub const fn description_id(&self) -> uuid::Uuid {
        self.0.description_id
    }

    pub fn value(&self) -> OffsetDateTime {
        self.0.value
    }

    pub(crate) fn restore(
        id: uuid::Uuid,
        description_id: uuid::Uuid,
        value: OffsetDateTime,
    ) -> Self {
        Self(FieldValueData { id, description_id, value })
    }
}

impl From<DateFieldValue> for FieldValue {
    fn from(value: DateFieldValue) -> Self {
        FieldValue::Date(value)
    }
}

impl FieldDescription {
    pub fn id(&self) -> uuid::Uuid {
        match self {
            FieldDescription::Text(desc) => desc.id(),
            FieldDescription::Number(desc) => desc.id(),
            FieldDescription::Boolean(desc) => desc.id(),
            FieldDescription::Date(desc) => desc.id(),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            FieldDescription::Text(desc) => desc.name(),
            FieldDescription::Number(desc) => desc.name(),
            FieldDescription::Boolean(desc) => desc.name(),
            FieldDescription::Date(desc) => desc.name(),
        }
    }
}

impl TextFieldDescription {
    pub fn new(name: impl Into<String>) -> Self {
        Self(FieldDescriptionData { id: uuid::Uuid::now_v7(), name: name.into() })
    }

    pub(crate) fn restore(id: uuid::Uuid, name: impl Into<String>) -> Self {
        Self(FieldDescriptionData { id, name: name.into() })
    }

    pub const fn id(&self) -> uuid::Uuid {
        self.0.id
    }

    pub fn name(&self) -> &str {
        &self.0.name
    }

    pub fn value(&self, value: impl Into<String>) -> FieldValue {
        TextFieldValue(FieldValueData {
            id: uuid::Uuid::now_v7(),
            description_id: self.id(),
            value: value.into(),
        })
        .into()
    }
}

impl From<TextFieldDescription> for FieldDescription {
    fn from(desc: TextFieldDescription) -> Self {
        FieldDescription::Text(desc)
    }
}

impl NumberFieldDescription {
    pub fn new(name: impl Into<String>) -> Self {
        Self(FieldDescriptionData { id: uuid::Uuid::now_v7(), name: name.into() })
    }

    pub(crate) fn restore(id: uuid::Uuid, name: impl Into<String>) -> Self {
        Self(FieldDescriptionData { id, name: name.into() })
    }

    pub const fn id(&self) -> uuid::Uuid {
        self.0.id
    }

    pub fn name(&self) -> &str {
        &self.0.name
    }

    pub fn value(&self, value: f64) -> FieldValue {
        NumberFieldValue(FieldValueData {
            id: uuid::Uuid::now_v7(),
            description_id: self.id(),
            value,
        })
        .into()
    }
}

impl From<NumberFieldDescription> for FieldDescription {
    fn from(desc: NumberFieldDescription) -> Self {
        FieldDescription::Number(desc)
    }
}

impl BooleanFieldDescription {
    pub fn new(name: impl Into<String>) -> Self {
        Self(FieldDescriptionData { id: uuid::Uuid::now_v7(), name: name.into() })
    }

    pub(crate) fn restore(id: uuid::Uuid, name: impl Into<String>) -> Self {
        Self(FieldDescriptionData { id, name: name.into() })
    }

    pub const fn id(&self) -> uuid::Uuid {
        self.0.id
    }

    pub fn name(&self) -> &str {
        &self.0.name
    }

    pub fn value(&self, value: bool) -> FieldValue {
        BooleanFieldValue(FieldValueData {
            id: uuid::Uuid::now_v7(),
            description_id: self.id(),
            value,
        })
        .into()
    }
}

impl From<BooleanFieldDescription> for FieldDescription {
    fn from(desc: BooleanFieldDescription) -> Self {
        FieldDescription::Boolean(desc)
    }
}

impl DateFieldDescription {
    pub fn new(name: impl Into<String>) -> Self {
        Self(FieldDescriptionData { id: uuid::Uuid::now_v7(), name: name.into() })
    }

    pub(crate) fn restore(id: uuid::Uuid, name: impl Into<String>) -> Self {
        Self(FieldDescriptionData { id, name: name.into() })
    }

    pub const fn id(&self) -> uuid::Uuid {
        self.0.id
    }

    pub fn name(&self) -> &str {
        &self.0.name
    }

    pub fn value(&self, value: OffsetDateTime) -> FieldValue {
        DateFieldValue(FieldValueData {
            id: uuid::Uuid::now_v7(),
            description_id: self.id(),
            value,
        })
        .into()
    }
}

impl From<DateFieldDescription> for FieldDescription {
    fn from(desc: DateFieldDescription) -> Self {
        FieldDescription::Date(desc)
    }
}

#[derive(Debug, Error)]
pub enum RecordError {
    #[error("Unknown field with ID: {0}")]
    UnknownField(uuid::Uuid),
    #[error("Field type mismatch for field with ID: {0}")]
    FieldTypeMismatch(uuid::Uuid),
}

pub struct Record {
    id: uuid::Uuid,
    description_id: uuid::Uuid,
    values: Vec<FieldValue>,
}

impl Record {
    pub fn new(
        description: &RecordDescription,
        values: Vec<FieldValue>,
    ) -> Result<Self, RecordError> {
        for value in &values {
            let Some(field) =
                description.fields().iter().find(|field| field.id() == value.description_id())
            else {
                return Err(RecordError::UnknownField(value.description_id()));
            };

            if !value.matches(field) {
                return Err(RecordError::FieldTypeMismatch(value.description_id()));
            }
        }
        Ok(Self { id: uuid::Uuid::now_v7(), description_id: description.id(), values })
    }

    pub(crate) fn restore(
        id: uuid::Uuid,
        description_id: uuid::Uuid,
        values: Vec<FieldValue>,
    ) -> Self {
        Self { id, description_id, values }
    }

    pub const fn id(&self) -> uuid::Uuid {
        self.id
    }

    pub const fn description_id(&self) -> uuid::Uuid {
        self.description_id
    }

    pub fn values(&self) -> &[FieldValue] {
        &self.values
    }
}
