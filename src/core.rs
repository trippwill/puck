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

// TODO: Remove
#![allow(dead_code)]
#![deny(missing_docs)]

mod collection;
mod field;
mod note;
mod record;

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

        impl AsRef<uuid::Uuid> for $name {
            fn as_ref(&self) -> &uuid::Uuid {
                self.as_uuid()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

#[doc(inline)]
pub(crate) use uuidv7_id;

#[doc(inline)]
pub use self::prelude::*;

/// Commonly used domain types.
pub mod prelude {
    pub use super::super::core::collection::prelude::*;
    pub use super::super::core::field::prelude::*;
    pub use super::super::core::note::prelude::*;
    pub use super::super::core::record::prelude::*;
}
