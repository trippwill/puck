#![allow(dead_code)]

mod adapter;
mod document;
mod version;

pub use document::{Document, DocumentError};
pub use version::SchemaVersion;
