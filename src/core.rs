// SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
// SPDX-License-Identifier: MPL-2.0

//! Domain types for notes and dynamically structured records.
//!
//! Create a free-form note:
//!
//! ```
//! use puck::core::PileNote;
//!
//! let note = PileNote::create("alpha-01 is 192.168.1.10");
//! assert_eq!(note.revision(), 1);
//! ```
//!
//! Create a record with a typed field:
//!
//! ```
//! use puck::core::{Collection, FieldType, Text};
//!
//! let hosts = Collection::new("Hosts");
//! let hostname = Text::def("Hostname");
//! let host = hosts.new_record();
//! let field = host.new_field(&hostname, String::from("alpha-01"));
//!
//! assert_eq!(field.val(), "alpha-01");
//! ```

#![deny(missing_docs)]

mod collection;
mod field;
mod note;
mod record;

/// Commonly used domain types.
pub mod prelude {
    pub use super::collection::{Collection, CollectionId};
    pub use super::field::{
        AnyField,
        AnyFieldDef,
        Boolean,
        Date,
        Field,
        FieldDef,
        FieldDefId,
        FieldKey,
        FieldType,
        Integer,
        Text,
        Time,
        Timestamp,
    };
    pub use super::note::{
        Archive,
        ArchiveNote,
        MAX_PREVIEW_CHARS,
        Note,
        NoteError,
        NoteId,
        NoteState,
        NoteSummary,
        Pile,
        PileNote,
    };
    pub use super::record::{Record, RecordId};
}

#[doc(inline)]
pub use prelude::*;

macro_rules! uuidv7_id {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(uuid::Uuid);

        impl $name {
            /// Creates a new time-ordered identifier.
            #[must_use]
            #[allow(clippy::new_without_default)]
            pub(crate) fn new() -> Self {
                Self(uuid::Uuid::now_v7())
            }

            /// Borrows the underlying UUID.
            #[must_use]
            pub(crate) const fn as_uuid(&self) -> &uuid::Uuid {
                &self.0
            }

            #[must_use]
            pub(crate) const fn restore(value: uuid::Uuid) -> Self {
                Self(value)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl std::str::FromStr for $name {
            type Err = uuid::Error;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                value.parse().map(Self::restore)
            }
        }
    };
}
pub(crate) use uuidv7_id;
