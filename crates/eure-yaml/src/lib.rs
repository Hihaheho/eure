#![doc = include_str!("../README.md")]

mod config;
mod convert;
mod error;
mod format;

pub use config::Config;
pub use convert::{
    value_to_yaml, value_to_yaml_with_config, yaml_to_value, yaml_to_value_with_config,
};
pub use error::Error;
pub use eure_value::value::VariantRepr;
pub use format::{format_eure, format_eure_bindings};

#[cfg(test)]
mod tests;

/// Result type for eure-yaml operations
pub type Result<T> = std::result::Result<T, Error>;
