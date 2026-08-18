// SPDX-License-Identifier: MPL-2.0

#![allow(dead_code)]

mod adapter;
mod command;
mod document;
mod query;
mod version;

pub mod prelude {
    pub use super::command::Command;
    pub use super::document::{Document, DocumentError};
    pub use super::query::prelude::*;
    pub use super::version::SchemaVersion;
}

#[doc(inline)]
pub use self::prelude::*;
