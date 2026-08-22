// SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
// SPDX-License-Identifier: MPL-2.0

//! Typed field definitions and record values.

use std::marker::PhantomData;
use std::str::FromStr;

use thiserror::Error;

use super::record::{Record, RecordId};
use super::uuidv7_id;

mod sealed {
    pub trait Sealed {}
}

uuidv7_id!(FieldDefId, "A unique field-definition identifier.");

/// A built-in field kind.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FieldKind {
    /// Free-form text.
    Text,
    /// A Boolean value.
    Boolean,
    /// A signed integer.
    Integer,
    /// A calendar date.
    Date,
    /// A wall-clock time.
    Time,
    /// A Unix timestamp.
    Timestamp,
}

impl FieldKind {
    /// All supported field kinds.
    pub const ALL: [Self; 6] = [
        Self::Text,
        Self::Boolean,
        Self::Integer,
        Self::Date,
        Self::Time,
        Self::Timestamp,
    ];
}

impl std::fmt::Display for FieldKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Text => "Text",
            Self::Boolean => "Boolean",
            Self::Integer => "Integer",
            Self::Date => "Date",
            Self::Time => "Time",
            Self::Timestamp => "Timestamp",
        })
    }
}

/// An invalid textual field value.
#[derive(Debug, Error)]
#[error("Invalid {kind} value {value:?}: expected {expected}")]
pub struct FieldValueError {
    kind: FieldKind,
    value: String,
    expected: &'static str,
}

/// A built-in field type and its Rust value type.
pub trait FieldType: self::sealed::Sealed + Sized {
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
#[derive(Debug, Clone, Copy)]
pub struct Text;
/// A Boolean value.
#[derive(Debug, Clone, Copy)]
pub struct Boolean;
/// A signed 64-bit integer.
#[derive(Debug, Clone, Copy)]
pub struct Integer;
/// A calendar date without a time or offset.
#[derive(Debug, Clone, Copy)]
pub struct Date;
/// A wall-clock time without a date or offset.
#[derive(Debug, Clone, Copy)]
pub struct Time;
/// A unix epoch timestamp in milliseconds since 1970-01-01T00:00:00Z.
#[derive(Debug, Clone, Copy)]
pub struct Timestamp;

/// A typed value belonging to a record and field definition.
#[derive(Debug, Clone)]
pub struct Field<T: FieldType> {
    def_id: FieldDefId,
    record_id: RecordId,
    value: T::Value,
}

/// A unique key for a [`Field`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldKey(pub RecordId, pub FieldDefId);

impl<T: FieldType> Field<T> {
    pub(crate) fn new(def: &FieldDef<T>, record: &Record, value: T::Value) -> Self {
        Self {
            def_id: def.id(),
            record_id: record.id(),
            value,
        }
    }

    /// Returns the field's definition ID.
    pub const fn def_id(&self) -> FieldDefId {
        self.def_id
    }

    /// Returns the field's record ID.
    pub const fn record_id(&self) -> RecordId {
        self.record_id
    }

    /// Returns the field's record and definition IDs.
    #[must_use]
    pub const fn key(&self) -> FieldKey {
        FieldKey(self.record_id, self.def_id)
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

impl AnyField {
    /// Returns the field's definition ID.
    #[must_use]
    pub const fn def_id(&self) -> FieldDefId {
        match self {
            AnyField::Text(f) => f.def_id(),
            AnyField::Boolean(f) => f.def_id(),
            AnyField::Integer(f) => f.def_id(),
            AnyField::Date(f) => f.def_id(),
            AnyField::Time(f) => f.def_id(),
            AnyField::Timestamp(f) => f.def_id(),
        }
    }

    /// Returns the field's record ID.
    #[must_use]
    pub const fn record_id(&self) -> RecordId {
        match self {
            AnyField::Text(f) => f.record_id(),
            AnyField::Boolean(f) => f.record_id(),
            AnyField::Integer(f) => f.record_id(),
            AnyField::Date(f) => f.record_id(),
            AnyField::Time(f) => f.record_id(),
            AnyField::Timestamp(f) => f.record_id(),
        }
    }

    /// Returns the field's unique key.
    #[must_use]
    pub const fn key(&self) -> FieldKey {
        match self {
            AnyField::Text(f) => f.key(),
            AnyField::Boolean(f) => f.key(),
            AnyField::Integer(f) => f.key(),
            AnyField::Date(f) => f.key(),
            AnyField::Time(f) => f.key(),
            AnyField::Timestamp(f) => f.key(),
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
    pub const fn id(&self) -> FieldDefId {
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
    /// A text value.
    Text(Field<Text>),
    /// A Boolean value.
    Boolean(Field<Boolean>),
    /// A signed integer value.
    Integer(Field<Integer>),
    /// A calendar date.
    Date(Field<Date>),
    /// A wall-clock time.
    Time(Field<Time>),
    /// A Unix timestamp.
    Timestamp(Field<Timestamp>),
}

/// A field definition of any supported type.
#[derive(Debug, Clone)]
pub enum AnyFieldDef {
    /// A text definition.
    Text(FieldDef<Text>),
    /// A Boolean definition.
    Boolean(FieldDef<Boolean>),
    /// A signed integer definition.
    Integer(FieldDef<Integer>),
    /// A definition for calendar dates.
    Date(FieldDef<Date>),
    /// A definition for wall-clock times.
    Time(FieldDef<Time>),
    /// A definition for Unix timestamps.
    Timestamp(FieldDef<Timestamp>),
}

impl AnyFieldDef {
    /// Returns the definition's ID.
    #[must_use]
    pub const fn id(&self) -> FieldDefId {
        match self {
            AnyFieldDef::Text(def) => def.id(),
            AnyFieldDef::Boolean(def) => def.id(),
            AnyFieldDef::Integer(def) => def.id(),
            AnyFieldDef::Date(def) => def.id(),
            AnyFieldDef::Time(def) => def.id(),
            AnyFieldDef::Timestamp(def) => def.id(),
        }
    }

    /// Returns the display name.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            AnyFieldDef::Text(def) => def.name(),
            AnyFieldDef::Boolean(def) => def.name(),
            AnyFieldDef::Integer(def) => def.name(),
            AnyFieldDef::Date(def) => def.name(),
            AnyFieldDef::Time(def) => def.name(),
            AnyFieldDef::Timestamp(def) => def.name(),
        }
    }

    /// Returns the definition's field kind.
    #[must_use]
    pub const fn kind(&self) -> FieldKind {
        match self {
            Self::Text(_) => FieldKind::Text,
            Self::Boolean(_) => FieldKind::Boolean,
            Self::Integer(_) => FieldKind::Integer,
            Self::Date(_) => FieldKind::Date,
            Self::Time(_) => FieldKind::Time,
            Self::Timestamp(_) => FieldKind::Timestamp,
        }
    }

    /// Creates a typed field by parsing a textual value.
    ///
    /// # Errors
    ///
    /// Returns an error when the value does not match this definition's kind.
    ///
    /// # Panics
    ///
    /// Panics only if the built-in date or time format description is invalid.
    pub fn new_field_from_str(
        &self,
        record: &Record,
        value: &str,
    ) -> Result<AnyField, FieldValueError> {
        match self {
            Self::Text(def) => Ok(AnyField::Text(record.new_field(def, value.to_owned()))),
            Self::Boolean(def) => parse(value, self.kind(), "boolean (true or false)")
                .map(|value| AnyField::Boolean(record.new_field(def, value))),
            Self::Integer(def) => parse(value, self.kind(), "integer")
                .map(|value| AnyField::Integer(record.new_field(def, value))),
            Self::Date(def) => {
                let format = time::format_description::parse_borrowed::<2>("[year]-[month]-[day]")
                    .expect("valid date format");
                time::Date::parse(value, &format)
                    .map(|value| AnyField::Date(record.new_field(def, value)))
                    .map_err(|_| invalid_value(value, self.kind(), "date (YYYY-MM-DD)"))
            }
            Self::Time(def) => {
                let format =
                    time::format_description::parse_borrowed::<2>("[hour]:[minute]:[second]")
                        .expect("valid time format");
                time::Time::parse(value, &format)
                    .map(|value| AnyField::Time(record.new_field(def, value)))
                    .map_err(|_| invalid_value(value, self.kind(), "time (HH:MM:SS)"))
            }
            Self::Timestamp(def) => value
                .parse()
                .ok()
                .and_then(|milliseconds| time::Timestamp::from_milliseconds(milliseconds).ok())
                .map(|value| AnyField::Timestamp(record.new_field(def, value)))
                .ok_or_else(|| invalid_value(value, self.kind(), "timestamp (Unix milliseconds)")),
        }
    }
}

fn parse<T: FromStr>(
    value: &str,
    kind: FieldKind,
    expected: &'static str,
) -> Result<T, FieldValueError> {
    value
        .parse()
        .map_err(|_| invalid_value(value, kind, expected))
}

fn invalid_value(value: &str, kind: FieldKind, expected: &'static str) -> FieldValueError {
    FieldValueError {
        kind,
        value: value.to_owned(),
        expected,
    }
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
        let record = collection.new_record("Alpha").unwrap();
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
    fn built_in_field_types_accept_their_values() {
        let record = Collection::new("Values").new_record("Value").unwrap();
        let date = time::Date::from_calendar_date(2026, Month::August, 18).unwrap();
        let time = time::Time::from_hms(15, 51, 0).unwrap();
        let timestamp = timestamp!(1_787_069_460);

        assert_eq!(
            record
                .new_field(&Text::def("Text"), String::from("alpha-01"))
                .val(),
            "alpha-01"
        );
        assert!(*record.new_field(&Boolean::def("Boolean"), true).val());
        assert_eq!(*record.new_field(&Integer::def("Integer"), 3).val(), 3);
        assert_eq!(*record.new_field(&Date::def("Date"), date).val(), date);
        assert_eq!(*record.new_field(&Time::def("Time"), time).val(), time);
        assert_eq!(
            *record
                .new_field(&Timestamp::def("Timestamp"), timestamp)
                .val(),
            timestamp
        );
    }

    #[test]
    fn definitions_parse_their_textual_values() {
        let record = Collection::new("Values").new_record("Value").unwrap();

        assert!(matches!(
            AnyFieldDef::Boolean(Boolean::def("Done"))
                .new_field_from_str(&record, "true"),
            Ok(AnyField::Boolean(field)) if *field.val()
        ));
        assert!(matches!(
            AnyFieldDef::Integer(Integer::def("Count")).new_field_from_str(&record, "not a number"),
            Err(FieldValueError { .. })
        ));
        assert!(matches!(
            AnyFieldDef::Date(Date::def("Due")).new_field_from_str(&record, "2026-08-22"),
            Ok(AnyField::Date(_))
        ));
    }
}
