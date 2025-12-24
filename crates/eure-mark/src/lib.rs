//! Eure Markdown document format
//!
//! This crate provides parsing and validation for `.eumd` files.

mod check;
mod document;
mod error;
mod reference;

pub use check::check_references;
pub use document::*;
pub use error::*;
pub use reference::*;
