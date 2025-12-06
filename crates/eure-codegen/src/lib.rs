//! Code generation from Eure schemas.
//!
//! This crate provides types for configuring code generation from Eure schemas.
//!
//! # Types
//!
//! ## Configuration
//!
//! - [`GenerationConfig`] - Runtime configuration for the code generator
//! - [`Visibility`] - Visibility of generated types
//!
//! ## Schema-Defined Codegen Settings
//!
//! These types implement [`ParseDocument`](eure_value::parse::ParseDocument) for
//! parsing codegen configuration from Eure schema files:
//!
//! - [`RootCodegen`] - Root-level `$codegen` extension
//! - [`CodegenDefaults`] - Root-level `$codegen-defaults` extension
//! - [`UnionCodegen`] - Codegen settings for union types
//! - [`RecordCodegen`] - Codegen settings for record types
//! - [`FieldCodegen`] - Codegen settings for individual record fields
//! - [`CascadeExtTypeCodegen`] - Codegen settings for cascade-ext-types
//! - [`CodegenStruct`] - Field grouping configuration

mod config;
mod parse;

pub use config::*;
pub use parse::*;
