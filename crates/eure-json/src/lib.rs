#![doc = include_str!("../README.md")]

mod error;
mod config;
mod convert;
mod format;

pub use error::Error;
pub use config::Config;
pub use eure_value::value::VariantRepr;
pub use convert::{
    value_to_json,
    value_to_json_with_config,
    json_to_value,
    json_to_value_with_config,
};
pub use format::{format_eure, format_eure_bindings};


#[cfg(test)]
mod tests;

#[cfg(test)]
mod test_format;

/// Result type for eure-json operations
pub type Result<T> = std::result::Result<T, Error>;
