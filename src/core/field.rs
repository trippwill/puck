use std::marker::PhantomData;

use super::record::prelude::*;
use super::uuidv7_id;

mod sealed {
    pub trait Sealed {}
}

pub mod prelude {
    pub use super::{
        AnyField,
        AnyFieldConvert,
        AnyFieldDef,
        AnyFieldDefConvert,
        Boolean,
        Date,
        Field,
        FieldDef,
        FieldDefId,
        FieldType,
        Integer,
        Text,
        Time,
        Timestamp,
    };
}

uuidv7_id!(FieldDefId);

/// A built-in field type and its Rust value type.
pub trait FieldType: self::sealed::Sealed + Sized + Clone {
    /// The Rust value type for this field type.
    type Value: Clone;

    /// Creates a typed field definition with a new ID.
    #[must_use]
    fn def(name: &str) -> FieldDef<Self> {
        FieldDef {
            id: FieldDefId::new(),
            name: name.into(),
            _marker: PhantomData,
        }
    }
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
/// A unix epoch timestamp in seconds and nanoseconds since 1970-01-01T00:00:00Z.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Timestamp;

/// A typed field definition and its associated value.
#[derive(Debug, Clone)]
pub struct Field<T: FieldType> {
    def_id: FieldDefId,
    record_id: RecordId,
    value: T::Value,
}

impl<T: FieldType> Field<T> {
    pub(crate) fn new(def: &FieldDef<T>, record: &Record, value: T::Value) -> Self {
        Self {
            def_id: def.id(),
            record_id: record.id(),
            value,
        }
    }

    /// Returns the field's definition ID.
    pub(crate) const fn def_id(&self) -> FieldDefId {
        self.def_id
    }

    /// Returns the field's record ID.
    pub(crate) const fn record_id(&self) -> RecordId {
        self.record_id
    }

    /// Returns a reference to the field's value.
    pub fn val(&self) -> &T::Value {
        &self.value
    }

    /// Restores a field from the given definition ID, record ID, and value.
    /// It is the caller's responsibility to ensure the [`FieldDefId`] is valid for the given value.
    #[must_use]
    pub(crate) fn restore(def_id: FieldDefId, record_id: RecordId, value: T::Value) -> Self {
        Self {
            def_id,
            record_id,
            value,
        }
    }
}

/// A definition that supports values of field type `T`.
#[derive(Debug, Clone)]
pub struct FieldDef<T: FieldType> {
    id: FieldDefId,
    name: Box<str>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: FieldType> FieldDef<T> {
    /// Returns the ID.
    #[must_use]
    pub(crate) const fn id(&self) -> FieldDefId {
        self.id
    }

    /// Returns the display name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the display name.
    pub fn set_name(&mut self, name: &str) {
        self.name = name.into();
    }

    #[must_use]
    pub(crate) fn restore(id: FieldDefId, name: &str) -> Self {
        Self {
            id,
            name: name.into(),
            _marker: PhantomData,
        }
    }
}

/// A field value of any supported type.
#[derive(Debug, Clone)]
pub enum AnyField {
    Text(Field<Text>),
    Boolean(Field<Boolean>),
    Integer(Field<Integer>),
    Date(Field<Date>),
    Time(Field<Time>),
    Timestamp(Field<Timestamp>),
}

/// A field definition of any supported type.
#[derive(Debug, Clone)]
pub enum AnyFieldDef {
    Text(FieldDef<Text>),
    Boolean(FieldDef<Boolean>),
    Integer(FieldDef<Integer>),
    Date(FieldDef<Date>),
    Time(FieldDef<Time>),
    Timestamp(FieldDef<Timestamp>),
}

impl self::sealed::Sealed for Text {}
impl self::sealed::Sealed for Boolean {}
impl self::sealed::Sealed for Integer {}
impl self::sealed::Sealed for Date {}
impl self::sealed::Sealed for Time {}
impl self::sealed::Sealed for Timestamp {}

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
impl FieldType for Timestamp {
    type Value = time::Timestamp;
}

pub trait AnyFieldConvert {
    fn into(self) -> AnyField;
    fn from(field: &AnyField) -> Option<&Self>
    where
        Self: Sized;
}

pub trait AnyFieldDefConvert {
    fn into(self) -> AnyFieldDef;
    fn from(field: &AnyFieldDef) -> Option<&Self>
    where
        Self: Sized;
}

impl AnyFieldConvert for Field<Text> {
    fn into(self) -> AnyField {
        AnyField::Text(self)
    }

    fn from(field: &AnyField) -> Option<&Self> {
        match field {
            AnyField::Text(f) => Some(f),
            _ => None,
        }
    }
}
impl AnyFieldConvert for Field<Boolean> {
    fn into(self) -> AnyField {
        AnyField::Boolean(self)
    }

    fn from(field: &AnyField) -> Option<&Self> {
        match field {
            AnyField::Boolean(f) => Some(f),
            _ => None,
        }
    }
}
impl AnyFieldConvert for Field<Integer> {
    fn into(self) -> AnyField {
        AnyField::Integer(self)
    }

    fn from(field: &AnyField) -> Option<&Self> {
        match field {
            AnyField::Integer(f) => Some(f),
            _ => None,
        }
    }
}
impl AnyFieldConvert for Field<Date> {
    fn into(self) -> AnyField {
        AnyField::Date(self)
    }

    fn from(field: &AnyField) -> Option<&Self> {
        match field {
            AnyField::Date(f) => Some(f),
            _ => None,
        }
    }
}
impl AnyFieldConvert for Field<Time> {
    fn into(self) -> AnyField {
        AnyField::Time(self)
    }

    fn from(field: &AnyField) -> Option<&Self> {
        match field {
            AnyField::Time(f) => Some(f),
            _ => None,
        }
    }
}
impl AnyFieldConvert for Field<Timestamp> {
    fn into(self) -> AnyField {
        AnyField::Timestamp(self)
    }

    fn from(field: &AnyField) -> Option<&Self> {
        match field {
            AnyField::Timestamp(f) => Some(f),
            _ => None,
        }
    }
}

impl AnyFieldDefConvert for FieldDef<Text> {
    fn into(self) -> AnyFieldDef {
        AnyFieldDef::Text(self)
    }

    fn from(field: &AnyFieldDef) -> Option<&Self> {
        match field {
            AnyFieldDef::Text(f) => Some(f),
            _ => None,
        }
    }
}
impl AnyFieldDefConvert for FieldDef<Boolean> {
    fn into(self) -> AnyFieldDef {
        AnyFieldDef::Boolean(self)
    }

    fn from(field: &AnyFieldDef) -> Option<&Self> {
        match field {
            AnyFieldDef::Boolean(f) => Some(f),
            _ => None,
        }
    }
}
impl AnyFieldDefConvert for FieldDef<Integer> {
    fn into(self) -> AnyFieldDef {
        AnyFieldDef::Integer(self)
    }

    fn from(field: &AnyFieldDef) -> Option<&Self> {
        match field {
            AnyFieldDef::Integer(f) => Some(f),
            _ => None,
        }
    }
}
impl AnyFieldDefConvert for FieldDef<Date> {
    fn into(self) -> AnyFieldDef {
        AnyFieldDef::Date(self)
    }

    fn from(field: &AnyFieldDef) -> Option<&Self> {
        match field {
            AnyFieldDef::Date(f) => Some(f),
            _ => None,
        }
    }
}
impl AnyFieldDefConvert for FieldDef<Time> {
    fn into(self) -> AnyFieldDef {
        AnyFieldDef::Time(self)
    }

    fn from(field: &AnyFieldDef) -> Option<&Self> {
        match field {
            AnyFieldDef::Time(f) => Some(f),
            _ => None,
        }
    }
}
impl AnyFieldDefConvert for FieldDef<Timestamp> {
    fn into(self) -> AnyFieldDef {
        AnyFieldDef::Timestamp(self)
    }

    fn from(field: &AnyFieldDef) -> Option<&Self> {
        match field {
            AnyFieldDef::Timestamp(f) => Some(f),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use time::Month;
    use time::macros::timestamp;

    use super::*;
    use crate::core::Collection;

    #[test]
    fn definitions_have_unique_identity_and_mutable_names() {
        let mut first = Text::def("");
        let second = Text::def("");

        assert_ne!(first.id(), second.id());
        assert_eq!(first.name(), "");

        first.set_name("Hostname");
        assert_eq!(first.name(), "Hostname");
    }

    #[test]
    fn restore_preserves_definition_and_field_data() {
        let collection = Collection::new("Hosts");
        let record = collection.new_record();
        let def_id = FieldDefId::new();
        let def = FieldDef::<Text>::restore(def_id, "Hostname");
        let field = Field::<Text>::restore(def_id, record.id(), String::from("alpha-01"));

        assert_eq!(def.id(), def_id);
        assert_eq!(def.name(), "Hostname");
        assert_eq!(field.def_id(), def_id);
        assert_eq!(field.record_id(), record.id());
        assert_eq!(field.val(), "alpha-01");
    }

    #[test]
    fn all_field_types_erase_and_recover() {
        let collection = Collection::new("Values");
        let record = collection.new_record();
        let date = time::Date::from_calendar_date(2026, Month::August, 17).unwrap();
        let time = time::Time::from_hms(21, 55, 0).unwrap();

        macro_rules! check {
            ($field_type:ty, $value:expr, $variant:ident) => {{
                let def = <$field_type as FieldType>::def(stringify!($variant));
                let erased_def = <FieldDef<$field_type> as AnyFieldDefConvert>::into(def.clone());
                assert!(matches!(erased_def, AnyFieldDef::$variant(_)));
                assert!(<FieldDef<$field_type> as AnyFieldDefConvert>::from(&erased_def).is_some());

                let erased =
                    <Field<$field_type> as AnyFieldConvert>::into(record.new_field(&def, $value));
                assert!(matches!(erased, AnyField::$variant(_)));
                assert!(<Field<$field_type> as AnyFieldConvert>::from(&erased).is_some());
            }};
        }

        check!(Text, String::from("alpha-01"), Text);
        check!(Boolean, true, Boolean);
        check!(Integer, 3, Integer);
        check!(Date, date, Date);
        check!(Time, time, Time);
        check!(Timestamp, timestamp!(1_787_001_300), Timestamp);
    }
}
