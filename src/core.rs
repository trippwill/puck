//! Domain types for dynamically structured records and pile notes.
//!
//! Field descriptions create typed field values. A [`RecordSchema`] selects the descriptions
//! allowed in a [`Record`], while a [`Collection`] groups records that share a schema. [`PileNote`]
//! provides a separate free-form note model.
//!
//! # Example
//!
//! ```
//! use indexmap::indexset;
//! use puck::core::{
//!     BooleanFieldDescription, Collection, NoteSummary, PileNote, RecordField, RecordSchema,
//!     TextFieldDescription,
//! };
//!
//! let hostname = TextFieldDescription::new("Hostname");
//! let active = BooleanFieldDescription::new("Active");
//! let machines = RecordSchema::new(
//!     "Machine",
//!     Some(String::from("A physical or virtual machine")),
//!     indexset! { hostname.id(), active.id() },
//! );
//!
//! let machine = machines
//!     .record([hostname.value(String::from("puck.local")).into(), active.value(true).into()])?;
//! let RecordField::Text(hostname_value) =
//!     machine.field_by_description(hostname.id()).expect("hostname is present")
//! else {
//!     unreachable!("hostname values are text");
//! };
//! assert_eq!(hostname_value.value(), "puck.local");
//!
//! let mut inventory = Collection::new("Inventory", machines.id());
//! inventory.insert(&machine)?;
//! assert!(inventory.contains(&machine));
//!
//! let note = PileNote::create("Provision puck.local");
//! let summary = NoteSummary::from(&note);
//! assert_eq!(summary.preview, "Provision puck.local");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod collection;
mod field;
mod note;
mod record;

macro_rules! uuidv7_id {
    ($name:ident) => {
        #[doc = concat!("A UUID v7-backed `", stringify!($name), "`.")]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(uuid::Uuid);

        impl $name {
            /// Creates a new time-ordered identifier.
            #[must_use]
            #[allow(clippy::new_without_default)]
            pub fn new() -> Self {
                Self(uuid::Uuid::now_v7())
            }

            /// Borrows the underlying UUID.
            #[must_use]
            pub const fn as_uuid(&self) -> &uuid::Uuid {
                &self.0
            }

            /// Returns the underlying UUID.
            #[must_use]
            pub const fn into_uuid(self) -> uuid::Uuid {
                self.0
            }

            #[allow(dead_code)]
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

pub use collection::{Collection, CollectionError, CollectionId};
pub use field::{
    BooleanField, BooleanFieldDescription, DateField, DateFieldDescription, FieldDescriptionId,
    FieldId, IntegerField, IntegerFieldDescription, TextField, TextFieldDescription, TimeField,
    TimeFieldDescription,
};
pub use note::{ArchiveNote, MAX_PREVIEW_CHARS, NoteError, NoteId, NoteSummary, PileNote};
pub use record::{Record, RecordError, RecordField, RecordId, RecordSchema, RecordSchemaId};
pub(crate) use uuidv7_id;
