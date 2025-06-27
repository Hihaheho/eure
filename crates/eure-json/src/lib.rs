#![doc = include_str!("../README.md")]

mod config;
mod convert;
mod error;
mod format;

pub use config::Config;
pub use convert::{
    json_to_value, json_to_value_with_config, value_to_json, value_to_json_with_config,
};
pub use error::Error;
pub use eure_value::value::VariantRepr;
pub use format::{format_eure, format_eure_bindings};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod test_format;

/// Result type for eure-json operations
pub type Result<T> = std::result::Result<T, Error>;
