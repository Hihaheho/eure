//! JSON Schema to EURE Schema conversion
//!
//! This crate provides functionality to convert between JSON Schema (Draft-07) and EURE Schema formats.

pub mod eure_to_json_schema;
pub mod json_schema;

pub use eure_to_json_schema::{eure_to_json_schema, ConversionError};
pub use json_schema::JsonSchema;
