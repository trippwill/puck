// SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
// SPDX-License-Identifier: MPL-2.0

//! Persistence types for puck documents.

mod adapter;
mod command;
mod document;
mod migration;
mod query_trait;
mod version;

pub mod query;

/// Commonly used persistence types.
pub mod prelude {
    pub use super::command::Command;
    pub use super::document::{Document, DocumentError};
    pub use super::version::SchemaVersion;
}

#[doc(inline)]
pub use prelude::*;

#[derive(Debug, thiserror::Error)]
pub(crate) enum SqlFieldTypeError {
    #[error("invalid field kind: {0}")]
    InvalidKind(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SqlFieldType {
    Text,
    Boolean,
    Integer,
    Date,
    Time,
    Timestamp,
}

impl AsRef<str> for SqlFieldType {
    fn as_ref(&self) -> &str {
        match self {
            SqlFieldType::Text => "text",
            SqlFieldType::Boolean => "boolean",
            SqlFieldType::Integer => "integer",
            SqlFieldType::Date => "date",
            SqlFieldType::Time => "time",
            SqlFieldType::Timestamp => "timestamp",
        }
    }
}

impl std::str::FromStr for SqlFieldType {
    type Err = SqlFieldTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "text" => Ok(SqlFieldType::Text),
            "boolean" => Ok(SqlFieldType::Boolean),
            "integer" => Ok(SqlFieldType::Integer),
            "date" => Ok(SqlFieldType::Date),
            "time" => Ok(SqlFieldType::Time),
            "timestamp" => Ok(SqlFieldType::Timestamp),
            _ => Err(SqlFieldTypeError::InvalidKind(s.to_string())),
        }
    }
}

impl tokio_rusqlite::ToSql for SqlFieldType {
    fn to_sql(&self) -> tokio_rusqlite::rusqlite::Result<tokio_rusqlite::types::ToSqlOutput<'_>> {
        Ok(tokio_rusqlite::types::ToSqlOutput::from(self.as_ref()))
    }
}

impl tokio_rusqlite::types::FromSql for SqlFieldType {
    fn column_result(
        value: tokio_rusqlite::types::ValueRef<'_>,
    ) -> tokio_rusqlite::types::FromSqlResult<Self> {
        let s = value.as_str()?;
        s.parse()
            .map_err(tokio_rusqlite::types::FromSqlError::other)
    }
}

pub(crate) enum AnyFieldValue {
    Text(String),
    Boolean(bool),
    Integer(i64),
    Date(time::Date),
    Time(time::Time),
    Timestamp(i64),
}

impl tokio_rusqlite::ToSql for AnyFieldValue {
    fn to_sql(&self) -> tokio_rusqlite::rusqlite::Result<tokio_rusqlite::types::ToSqlOutput<'_>> {
        match self {
            AnyFieldValue::Text(s) => s.to_sql(),
            AnyFieldValue::Boolean(b) => b.to_sql(),
            AnyFieldValue::Integer(i) => i.to_sql(),
            AnyFieldValue::Date(d) => d.to_sql(),
            AnyFieldValue::Time(t) => t.to_sql(),
            AnyFieldValue::Timestamp(ts) => ts.to_sql(),
        }
    }
}
