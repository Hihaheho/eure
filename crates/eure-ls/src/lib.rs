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

use std::path::PathBuf;

use eure::query::{Workspace, WorkspaceId};
use lsp_types::InitializeParams;
use query_flow::{DurabilityLevel, QueryRuntime};

/// Register workspaces from LSP initialization parameters.
pub fn register_workspaces_from_init(runtime: &mut QueryRuntime, params: &InitializeParams) {
    if let Some(folders) = &params.workspace_folders {
        for folder in folders {
            let workspace_path = PathBuf::from(folder.uri.path().as_str());
            let config_path = workspace_path.join("Eure.eure");

            runtime.resolve_asset(
                WorkspaceId(workspace_path.to_string_lossy().into_owned()),
                Workspace {
                    path: workspace_path,
                    config_path,
                },
                DurabilityLevel::Static,
            );
        }
    } else if let Some(root_uri) = {
        #[allow(
            deprecated,
            reason = "fallback for clients without workspace_folders support"
        )]
        &params.root_uri
    } {
        let workspace_path = PathBuf::from(root_uri.path().as_str());
        let config_path = workspace_path.join("Eure.eure");

        runtime.resolve_asset(
            WorkspaceId(workspace_path.to_string_lossy().into_owned()),
            Workspace {
                path: workspace_path,
                config_path,
            },
            DurabilityLevel::Static,
        );
    }
}
