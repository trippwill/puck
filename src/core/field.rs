use crate::core::uuidv7_id;

uuidv7_id!(FieldDescriptionId);
uuidv7_id!(FieldId);

mod sealed {
    pub trait Sealed {}
}

/// A built-in field type and its Rust value type.
///
/// This trait is sealed because [`crate::core::RecordField`] supports a fixed set of variants.
pub trait FieldType: sealed::Sealed {
    type Value;
}

/// Text stored as a [`String`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Text;
/// A Boolean value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Boolean;
/// A signed 64-bit integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Integer;
/// A calendar date without a time or offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Date;
/// A wall-clock time without a date or offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Time;

impl sealed::Sealed for Text {}
impl sealed::Sealed for Boolean {}
impl sealed::Sealed for Integer {}
impl sealed::Sealed for Date {}
impl sealed::Sealed for Time {}

impl FieldType for Text {
    type Value = String;
}

impl FieldType for Boolean {
    type Value = bool;
}

impl FieldType for Integer {
    type Value = i64;
}

impl FieldType for Date {
    type Value = time::Date;
}

impl FieldType for Time {
    type Value = time::Time;
}

/// A text field description.
pub type TextFieldDescription = FieldDescription<Text>;
/// A Boolean field description.
pub type BooleanFieldDescription = FieldDescription<Boolean>;
/// An integer field description.
pub type IntegerFieldDescription = FieldDescription<Integer>;
/// A calendar-date field description.
pub type DateFieldDescription = FieldDescription<Date>;
/// A wall-clock-time field description.
pub type TimeFieldDescription = FieldDescription<Time>;

/// A named definition that creates values of field type `T`.
#[derive(Debug, Clone)]
pub struct FieldDescription<T: FieldType> {
    id: FieldDescriptionId,
    name: String,
    _marker: std::marker::PhantomData<T>,
}

impl<T: FieldType> FieldDescription<T> {
    /// Creates a field description with a new ID.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self { id: FieldDescriptionId::new(), name: name.into(), _marker: std::marker::PhantomData }
    }

    /// Creates a typed field value belonging to this description.
    #[must_use]
    pub fn value(&self, value: T::Value) -> Field<T> {
        Field::new(self.id, value)
    }

    /// Returns the field-description ID.
    #[must_use]
    pub const fn id(&self) -> FieldDescriptionId {
        self.id
    }

    /// Returns the display name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[allow(dead_code)]
    pub(crate) fn restore(id: FieldDescriptionId, name: String) -> Self {
        Self { id, name, _marker: std::marker::PhantomData }
    }
}

/// A text field value.
pub type TextField = Field<Text>;
/// A Boolean field value.
pub type BooleanField = Field<Boolean>;
/// An integer field value.
pub type IntegerField = Field<Integer>;
/// A calendar-date field value.
pub type DateField = Field<Date>;
/// A wall-clock-time field value.
pub type TimeField = Field<Time>;

/// A typed value created by a [`FieldDescription`].
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Field<T: FieldType> {
    id: FieldId,
    description_id: FieldDescriptionId,
    value: T::Value,
}

impl<T: FieldType> Clone for Field<T>
where
    T::Value: Clone,
{
    fn clone(&self) -> Self {
        Self { id: self.id, description_id: self.description_id, value: self.value.clone() }
    }
}

impl<T: FieldType> Field<T> {
    #[must_use]
    fn new(description_id: FieldDescriptionId, value: T::Value) -> Self {
        Self { id: FieldId::new(), description_id, value }
    }

    /// Returns the field-value ID.
    #[must_use]
    pub const fn id(&self) -> FieldId {
        self.id
    }

    /// Returns the description that created this field.
    #[must_use]
    pub const fn description_id(&self) -> FieldDescriptionId {
        self.description_id
    }

    /// Borrows the typed value.
    #[must_use]
    pub fn value(&self) -> &T::Value {
        &self.value
    }

    #[allow(dead_code)]
    pub(crate) fn restore(
        id: FieldId,
        description_id: FieldDescriptionId,
        value: T::Value,
    ) -> Self {
        Self { id, description_id, value }
    }
}

#[cfg(test)]
mod tests {
    use time::Month;

    use super::*;

    #[test]
    fn descriptions_create_typed_values() {
        let text = TextFieldDescription::new("Greeting");
        let boolean = BooleanFieldDescription::new("Active");
        let integer = IntegerFieldDescription::new("Priority");
        let date = DateFieldDescription::new("Due");
        let time = TimeFieldDescription::new("Starts");
        let due = time::Date::from_calendar_date(2026, Month::August, 9).unwrap();
        let starts = time::Time::from_hms(9, 30, 0).unwrap();

        let text_value = text.value(String::from("Hello"));
        let second_text_value = text.value(String::from("Hey"));
        let boolean_value = boolean.value(true);
        let integer_value = integer.value(3);
        let date_value = date.value(due);
        let time_value = time.value(starts);

        assert_eq!(text.name(), "Greeting");
        assert_eq!(text_value.description_id(), text.id());
        assert_ne!(text_value.id(), second_text_value.id());
        assert_eq!(text_value.value(), "Hello");
        assert!(*boolean_value.value());
        assert_eq!(*integer_value.value(), 3);
        assert_eq!(*date_value.value(), due);
        assert_eq!(*time_value.value(), starts);
    }
}
