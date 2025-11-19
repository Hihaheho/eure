#![doc = include_str!("../README.md")]

mod config;
mod error;

pub use config::Config;
pub use error::Error;
pub use eure::value::VariantRepr;
