//! Eure Language Server - LSP implementation for the Eure data format.
//!
//! This crate provides both a native binary (`eurels`) and a WASM module
//! for use in VS Code web extensions.

mod capabilities;
mod queries;

// Native-only modules (require threading, crossbeam, etc.)
#[cfg(not(target_arch = "wasm32"))]
pub mod asset_locator;
#[cfg(not(target_arch = "wasm32"))]
pub mod executor;
#[cfg(not(target_arch = "wasm32"))]
pub mod io_pool;
#[cfg(not(target_arch = "wasm32"))]
pub mod types;

// WASM-specific module
#[cfg(target_arch = "wasm32")]
mod wasm_api;
#[cfg(target_arch = "wasm32")]
pub use wasm_api::WasmCore;

// Public exports for shared functionality
pub use capabilities::server_capabilities;
pub use queries::{LspDiagnostics, LspSemanticTokens};
