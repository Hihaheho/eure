//! Eure Language Server - LSP implementation for the Eure data format.
//!
//! This crate provides both a native binary (`eurels`) and a WASM module
//! for use in VS Code web extensions.

mod capabilities;
pub mod queries;
mod uri_utils;

// WASM-specific module
#[cfg(target_arch = "wasm32")]
mod wasm_api;
#[cfg(target_arch = "wasm32")]
pub use wasm_api::WasmCore;

// Public exports for shared functionality
pub use capabilities::server_capabilities;
pub use queries::{LspDiagnostics, LspSemanticTokens};
