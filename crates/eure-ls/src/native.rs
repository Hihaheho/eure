//! Native platform support for the Eure Language Server.
//!
//! This module provides the event loop and I/O pool for running
//! the language server on native platforms (non-WASM).

mod io_pool;

pub use io_pool::{IoPool, IoRequest, IoResponse};
