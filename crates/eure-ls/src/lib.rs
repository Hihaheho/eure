//! Eure Language Server - LSP implementation for the Eure data format.
//!
//! This crate provides both a native binary (`eurels`) and a WASM module
//! for use in VS Code web extensions.

mod capabilities;
pub mod queries;
pub mod types;
mod uri_utils;

// Native-specific module (non-WASM)
#[cfg(not(target_arch = "wasm32"))]
pub mod native;

// WASM-specific module
#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
pub use wasm::WasmCore;

// Public exports for shared functionality
pub use capabilities::server_capabilities;
pub use queries::{
    LspDiagnostics, LspFileDiagnostics, LspSemanticTokens, lsp_completion, lsp_definition,
    lsp_hover, position_to_offset,
};
pub use types::{CoreRequestId, Effect, LspError, LspOutput};

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use eure::query::{
    CollectDiagnosticTargets, Glob, GlobResult, OpenDocuments, OpenDocumentsList, TextFile,
    TextFileContent, Workspace, WorkspaceId, build_runtime,
};
use lsp_types::InitializeParams;
use query_flow::{DurabilityLevel, QueryRuntime};

use crate::types::{
    CommandQuery, CommandResult, CompletionRequest, DefinitionRequest, FileDiagnosticsSubscription,
    HoverRequest, PendingRequest,
};
use crate::uri_utils::uri_to_text_file;

use lsp_types::{
    CompletionParams, CompletionResponse, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, HoverParams, InitializeResult, PublishDiagnosticsParams,
    SemanticTokensParams,
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument,
        Notification as LspNotification, PublishDiagnostics,
    },
    request::{
        Completion, HoverRequest as HoverLspRequest, Initialize, Request as LspRequest,
        SemanticTokensFullRequest, Shutdown,
    },
};

use crate::uri_utils::text_file_to_uri;
use query_flow::{Db, QueryError};
use serde_json::Value;

// Cross-platform logging
#[cfg(not(target_arch = "wasm32"))]
use tracing::{debug, error};

#[cfg(target_arch = "wasm32")]
macro_rules! debug {
    ($($arg:tt)*) => { web_sys::console::debug_1(&format!($($arg)*).into()) };
}
#[cfg(target_arch = "wasm32")]
macro_rules! error {
    ($($arg:tt)*) => { web_sys::console::error_1(&format!($($arg)*).into()) };
}

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

/// The headless LSP core state machine.
///
/// This struct contains all the state and logic for the language server,
/// independent of the platform-specific event loop. Both native and WASM
/// implementations use this core.
pub struct LspCore {
    /// The query runtime for executing LSP queries.
    runtime: QueryRuntime,
    /// Pending requests waiting for assets to be resolved.
    pending_requests: HashMap<CoreRequestId, PendingRequest>,
    /// Files that have been requested but not yet resolved.
    pending_assets: HashSet<TextFile>,
    /// Glob patterns that have been requested but not yet resolved.
    pending_globs: HashMap<String, Glob>,
    /// Per-file diagnostics subscriptions with revision tracking.
    diagnostics_subscriptions: HashMap<TextFile, FileDiagnosticsSubscription>,
    /// URIs we've published diagnostics to (for stale clearing).
    published_uris: HashSet<String>,
    /// Cached content of open documents (keyed by URI string).
    documents: HashMap<String, String>,
    /// Whether the server has been initialized.
    initialized: bool,
    definition_link_support: bool,
}

impl LspCore {
    /// Create a new LspCore instance.
    pub fn new() -> Self {
        let runtime = build_runtime();

        Self {
            runtime,
            pending_requests: HashMap::new(),
            pending_assets: HashSet::new(),
            pending_globs: HashMap::new(),
            diagnostics_subscriptions: HashMap::new(),
            published_uris: HashSet::new(),
            documents: HashMap::new(),
            initialized: false,
            definition_link_support: false,
        }
    }

    /// Get a mutable reference to the query runtime.
    ///
    /// This is useful for registering workspaces during initialization.
    pub fn runtime_mut(&mut self) -> &mut QueryRuntime {
        &mut self.runtime
    }

    /// Check if the server has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Apply capabilities for both the native and WASM initialization paths.
    pub fn configure_client(&mut self, capabilities: &lsp_types::ClientCapabilities) {
        self.definition_link_support = capabilities
            .text_document
            .as_ref()
            .and_then(|caps| caps.definition.as_ref())
            .and_then(|caps| caps.link_support)
            .unwrap_or(false);
    }

    /// Mark the server as initialized.
    pub fn set_initialized(&mut self) {
        self.initialized = true;
    }

    /// Get pending files that need to be fetched.
    pub fn pending_files(&self) -> impl Iterator<Item = &TextFile> {
        self.pending_assets.iter()
    }

    /// Get pending glob patterns that need to be expanded.
    pub fn pending_globs(&self) -> impl Iterator<Item = (&str, &Glob)> {
        self.pending_globs.iter().map(|(k, v)| (k.as_str(), v))
    }

    // === Document Management ===

    /// Update the OpenDocuments asset with current open documents.
    ///
    /// This should be called whenever documents are opened or closed to ensure
    /// collection queries (`CollectDiagnosticTargets`, `CollectSchemaFiles`) are invalidated.
    fn update_open_documents(&mut self) {
        let files: Vec<TextFile> = self
            .documents
            .keys()
            .filter_map(|uri| uri_to_text_file(uri).ok())
            .collect();

        self.runtime.resolve_asset(
            OpenDocuments,
            OpenDocumentsList(files),
            DurabilityLevel::Volatile,
        );
    }

    /// Open a document and cache its content.
    ///
    /// This should be called when a `textDocument/didOpen` notification is received.
    pub fn open_document(&mut self, uri: &str, content: String) {
        // Update document cache
        self.documents.insert(uri.to_string(), content.clone());

        // Resolve in query runtime
        let Ok(file) = uri_to_text_file(uri) else {
            return; // Invalid URI - skip
        };
        self.runtime
            .resolve_asset(file, TextFileContent(content), DurabilityLevel::Volatile);

        // Update open documents asset
        self.update_open_documents();
    }

    /// Update a document's content.
    ///
    /// This should be called when a `textDocument/didChange` notification is received.
    pub fn change_document(&mut self, uri: &str, content: String) {
        // Same as open - we use full sync mode
        self.open_document(uri, content);
    }

    /// Close a document and clear its cached content.
    ///
    /// This should be called when a `textDocument/didClose` notification is received.
    pub fn close_document(&mut self, uri: &str) {
        // Remove from document cache
        self.documents.remove(uri);

        // Invalidate in query runtime
        if let Ok(file) = uri_to_text_file(uri) {
            // Downloaded sources remain authoritative when their read-only
            // editor closes; reopening must not independently refetch them.
            if file.as_local_path().is_some() {
                self.runtime.invalidate_asset(&file);
            }
        }

        // Update open documents asset - this triggers re-evaluation of diagnostic targets
        self.update_open_documents();
    }

    /// Get the cached content of a document.
    pub fn get_document(&self, uri: &str) -> Option<&String> {
        self.documents.get(uri)
    }

    // === Request Handling ===

    /// Handle an LSP request.
    ///
    /// Returns outputs to send to the client and effects for the platform to perform.
    pub fn handle_request(
        &mut self,
        id: CoreRequestId,
        method: &str,
        params: Value,
    ) -> (Vec<LspOutput>, Vec<Effect>) {
        let mut outputs = Vec::new();
        let mut effects = Vec::new();

        match method {
            Initialize::METHOD => {
                let init_params: InitializeParams = match serde_json::from_value(params) {
                    Ok(p) => p,
                    Err(e) => {
                        outputs.push(LspOutput::Response {
                            id,
                            result: Err(LspError::invalid_params(format!("Invalid params: {}", e))),
                        });
                        return (outputs, effects);
                    }
                };

                self.configure_client(&init_params.capabilities);

                // Register workspaces from initialization
                register_workspaces_from_init(&mut self.runtime, &init_params);

                let result = InitializeResult {
                    capabilities: server_capabilities(),
                    server_info: Some(lsp_types::ServerInfo {
                        name: "eure-ls".to_string(),
                        version: Some(env!("CARGO_PKG_VERSION").to_string()),
                    }),
                };

                self.initialized = true;
                outputs.push(LspOutput::Response {
                    id,
                    result: Ok(serde_json::to_value(result).unwrap()),
                });
            }
            Shutdown::METHOD => {
                outputs.push(LspOutput::Response {
                    id,
                    result: Ok(Value::Null),
                });
            }
            SemanticTokensFullRequest::METHOD => {
                let params: SemanticTokensParams = match serde_json::from_value(params) {
                    Ok(p) => p,
                    Err(e) => {
                        outputs.push(LspOutput::Response {
                            id,
                            result: Err(LspError::invalid_params(format!("Invalid params: {}", e))),
                        });
                        return (outputs, effects);
                    }
                };

                let uri = params.text_document.uri;
                let uri_str = uri.as_str();
                let file = match uri_to_text_file(uri_str) {
                    Ok(f) => f,
                    Err(e) => {
                        outputs.push(LspOutput::Response {
                            id,
                            result: Err(LspError::invalid_params(format!("Invalid URI: {}", e))),
                        });
                        return (outputs, effects);
                    }
                };
                let source = self.documents.get(uri_str).cloned().unwrap_or_default();

                let query = LspSemanticTokens::new(file, source.clone());
                let command = CommandQuery::SemanticTokensFull(query);
                let (cmd_outputs, cmd_effects) = self.run_command(id, command);
                outputs.extend(cmd_outputs);
                effects.extend(cmd_effects);
            }
            Completion::METHOD => {
                let params: CompletionParams = match serde_json::from_value(params) {
                    Ok(p) => p,
                    Err(e) => {
                        outputs.push(LspOutput::Response {
                            id,
                            result: Err(LspError::invalid_params(format!("Invalid params: {}", e))),
                        });
                        return (outputs, effects);
                    }
                };

                let position = params.text_document_position;
                let uri_str = position.text_document.uri.as_str();
                let file = match uri_to_text_file(uri_str) {
                    Ok(f) => f,
                    Err(e) => {
                        outputs.push(LspOutput::Response {
                            id,
                            result: Err(LspError::invalid_params(format!("Invalid URI: {}", e))),
                        });
                        return (outputs, effects);
                    }
                };
                // The cursor is converted against the text we last received for
                // this document, which is exactly what the queries parse.
                let source = self.documents.get(uri_str).cloned().unwrap_or_default();
                let offset = position_to_offset(&source, position.position) as u32;

                let command = CommandQuery::Completion(CompletionRequest { file, offset });
                let (cmd_outputs, cmd_effects) = self.run_command(id, command);
                outputs.extend(cmd_outputs);
                effects.extend(cmd_effects);
            }
            HoverLspRequest::METHOD => {
                let params: HoverParams = match serde_json::from_value(params) {
                    Ok(p) => p,
                    Err(e) => {
                        outputs.push(LspOutput::Response {
                            id,
                            result: Err(LspError::invalid_params(format!("Invalid params: {}", e))),
                        });
                        return (outputs, effects);
                    }
                };

                let position = params.text_document_position_params;
                let uri_str = position.text_document.uri.as_str();
                let file = match uri_to_text_file(uri_str) {
                    Ok(f) => f,
                    Err(e) => {
                        outputs.push(LspOutput::Response {
                            id,
                            result: Err(LspError::invalid_params(format!("Invalid URI: {}", e))),
                        });
                        return (outputs, effects);
                    }
                };
                let source = self.documents.get(uri_str).cloned().unwrap_or_default();
                let offset = position_to_offset(&source, position.position) as u32;

                let command = CommandQuery::Hover(HoverRequest { file, offset });
                let (cmd_outputs, cmd_effects) = self.run_command(id, command);
                outputs.extend(cmd_outputs);
                effects.extend(cmd_effects);
            }
            "textDocument/definition" => {
                let params: lsp_types::GotoDefinitionParams = match serde_json::from_value(params) {
                    Ok(p) => p,
                    Err(e) => {
                        outputs.push(LspOutput::Response {
                            id,
                            result: Err(LspError::invalid_params(format!("Invalid params: {}", e))),
                        });
                        return (outputs, effects);
                    }
                };

                let position = params.text_document_position_params;
                let uri_str = position.text_document.uri.as_str();
                let file = match uri_to_text_file(uri_str) {
                    Ok(f) => f,
                    Err(e) => {
                        outputs.push(LspOutput::Response {
                            id,
                            result: Err(LspError::invalid_params(format!("Invalid URI: {}", e))),
                        });
                        return (outputs, effects);
                    }
                };
                let command = CommandQuery::Definition(DefinitionRequest {
                    file,
                    position: position.position,
                });
                let (cmd_outputs, cmd_effects) = self.run_command(id, command);
                outputs.extend(cmd_outputs);
                effects.extend(cmd_effects);
            }
            "eure/schemaContent" => {
                let params: lsp_types::TextDocumentIdentifier = match serde_json::from_value(params)
                {
                    Ok(params) => params,
                    Err(error) => {
                        outputs.push(LspOutput::Response {
                            id,
                            result: Err(LspError::invalid_params(error.to_string())),
                        });
                        return (outputs, effects);
                    }
                };
                let file = match uri_to_text_file(params.uri.as_str()) {
                    Ok(file) if file.as_url().is_some() => file,
                    _ => {
                        outputs.push(LspOutput::Response {
                            id,
                            result: Err(LspError::invalid_params("Expected an HTTPS schema URI")),
                        });
                        return (outputs, effects);
                    }
                };
                let (cmd_outputs, cmd_effects) =
                    self.run_command(id, CommandQuery::SchemaContent(file));
                outputs.extend(cmd_outputs);
                effects.extend(cmd_effects);
            }
            _ => {
                outputs.push(LspOutput::Response {
                    id,
                    result: Err(LspError::method_not_found(method)),
                });
            }
        }

        (outputs, effects)
    }

    /// Execute a command query for request `id`, or park it until the assets
    /// it suspended on are resolved.
    fn run_command(
        &mut self,
        id: CoreRequestId,
        command: CommandQuery,
    ) -> (Vec<LspOutput>, Vec<Effect>) {
        let mut outputs = Vec::new();
        let mut effects = Vec::new();

        match self.try_execute(&command) {
            Ok(result) => {
                let json = self.result_to_value(result);
                outputs.push(LspOutput::Response {
                    id,
                    result: Ok(json),
                });
            }
            Err(QueryError::Suspend { .. }) => {
                // Query is pending - collect effects and store request
                let (new_effects, waiting_for) = self.collect_pending_assets();
                effects.extend(new_effects);

                self.pending_requests.insert(
                    id.clone(),
                    PendingRequest {
                        id,
                        command,
                        waiting_for,
                    },
                );
            }
            Err(e) => {
                if let Some(lsp_err) = Self::handle_query_error(command.name(), e) {
                    outputs.push(LspOutput::Response {
                        id,
                        result: Err(lsp_err),
                    });
                }
            }
        }

        (outputs, effects)
    }

    /// Cancel a pending request.
    pub fn cancel_request(&mut self, id: &CoreRequestId) {
        self.pending_requests.remove(id);
    }

    // === Notification Handling ===

    /// Handle an LSP notification.
    ///
    /// Returns outputs to send to the client and effects for the platform to perform.
    pub fn handle_notification(
        &mut self,
        method: &str,
        params: Value,
    ) -> (Vec<LspOutput>, Vec<Effect>) {
        let mut outputs = Vec::new();
        let mut effects = Vec::new();

        match method {
            DidOpenTextDocument::METHOD => {
                if let Ok(params) = serde_json::from_value::<DidOpenTextDocumentParams>(params) {
                    let uri = params.text_document.uri;
                    let content = params.text_document.text;

                    // Open document in core
                    self.open_document(uri.as_str(), content);

                    // Refresh diagnostics for all targets
                    let (diag_outputs, diag_effects) = self.refresh_diagnostics();
                    outputs.extend(diag_outputs);
                    effects.extend(diag_effects);
                }
            }
            DidChangeTextDocument::METHOD => {
                if let Ok(params) = serde_json::from_value::<DidChangeTextDocumentParams>(params) {
                    let uri = params.text_document.uri;
                    // We use FULL sync, so there's only one change with the full content
                    if let Some(change) = params.content_changes.into_iter().next() {
                        let content = change.text;

                        // Change document in core
                        self.change_document(uri.as_str(), content);

                        // Refresh diagnostics for all targets
                        let (diag_outputs, diag_effects) = self.refresh_diagnostics();
                        outputs.extend(diag_outputs);
                        effects.extend(diag_effects);
                    }
                }
            }
            DidCloseTextDocument::METHOD => {
                if let Ok(params) = serde_json::from_value::<DidCloseTextDocumentParams>(params) {
                    let uri = params.text_document.uri;
                    let uri_str = uri.as_str();

                    // Close document in core
                    self.close_document(uri_str);

                    // Also remove any pending requests for this document
                    self.pending_requests
                        .retain(|_, pending| text_file_to_uri(pending.command.file()) != uri_str);

                    // Refresh diagnostics - stale files will be cleared automatically
                    let (diag_outputs, diag_effects) = self.refresh_diagnostics();
                    outputs.extend(diag_outputs);
                    effects.extend(diag_effects);
                }
            }
            "$/cancelRequest" => {
                if let Some(id) = params.get("id") {
                    let core_id = CoreRequestId::from(id);
                    self.cancel_request(&core_id);
                }
            }
            "initialized" | "exit" => {
                // Ignore
            }
            _ => {
                // Unknown notification - ignore
            }
        }

        (outputs, effects)
    }

    /// Refresh diagnostics for all diagnostic targets.
    ///
    /// Uses `CollectDiagnosticTargets` to discover all files needing diagnostics,
    /// then polls `LspFileDiagnostics` for each file with per-file revision tracking.
    ///
    /// Returns notifications for all changed files and any effects needed.
    fn refresh_diagnostics(&mut self) -> (Vec<LspOutput>, Vec<Effect>) {
        let mut outputs = Vec::new();
        let mut effects = Vec::new();

        debug!("[LspCore] refresh_diagnostics");

        // 1. Collect all files to diagnose (includes open docs + schema files)
        let all_files = match self.runtime.poll(CollectDiagnosticTargets::new()) {
            Ok(polled) => match polled.value {
                Ok(files) => files,
                Err(e) => {
                    error!("CollectDiagnosticTargets error: {}", e);
                    return (outputs, effects);
                }
            },
            Err(QueryError::Suspend { .. }) => {
                debug!("[LspCore] CollectDiagnosticTargets suspended");
                let (new_effects, _) = self.collect_pending_assets();
                effects.extend(new_effects);
                return (outputs, effects);
            }
            Err(e) => {
                Self::handle_query_error("CollectDiagnosticTargets", e);
                return (outputs, effects);
            }
        };

        debug!("[LspCore] diagnostic targets: {} files", all_files.len());

        // 2. Poll LspFileDiagnostics for each file
        let mut current_uris = HashSet::new();
        for file in all_files.iter() {
            let query = LspFileDiagnostics::new(file.clone());

            // Get or create subscription
            let last_revision = self
                .diagnostics_subscriptions
                .get(file)
                .map(|s| s.last_revision)
                .unwrap_or_default();

            match self.runtime.poll(query.clone()) {
                Ok(polled) => {
                    let uri = text_file_to_uri(file);
                    current_uris.insert(uri.clone());

                    // Only publish if revision changed
                    if polled.revision != last_revision {
                        // Update subscription
                        self.diagnostics_subscriptions.insert(
                            file.clone(),
                            FileDiagnosticsSubscription {
                                file: file.clone(),
                                query,
                                last_revision: polled.revision,
                            },
                        );

                        match polled.value {
                            Ok(diagnostics) => {
                                debug!(
                                    "[LspCore] sending {} diagnostics for {}",
                                    diagnostics.len(),
                                    uri
                                );
                                if let Ok(parsed_uri) = uri.parse::<lsp_types::Uri>() {
                                    let params = PublishDiagnosticsParams {
                                        uri: parsed_uri,
                                        diagnostics: diagnostics.as_ref().clone(),
                                        version: None,
                                    };
                                    outputs.push(LspOutput::Notification {
                                        method: PublishDiagnostics::METHOD.to_string(),
                                        params: serde_json::to_value(params).unwrap(),
                                    });
                                }
                            }
                            Err(e) => {
                                error!("Diagnostics query error for {}: {}", uri, e);
                                if let Ok(parsed_uri) = uri.parse::<lsp_types::Uri>() {
                                    let params = PublishDiagnosticsParams {
                                        uri: parsed_uri,
                                        diagnostics: vec![],
                                        version: None,
                                    };
                                    outputs.push(LspOutput::Notification {
                                        method: PublishDiagnostics::METHOD.to_string(),
                                        params: serde_json::to_value(params).unwrap(),
                                    });
                                }
                            }
                        }
                    }
                }
                Err(QueryError::Suspend { .. }) => {
                    debug!("[LspCore] diagnostics for {:?} suspended", file);
                    // Store subscription for retry
                    self.diagnostics_subscriptions.insert(
                        file.clone(),
                        FileDiagnosticsSubscription {
                            file: file.clone(),
                            query,
                            last_revision,
                        },
                    );
                    let (new_effects, _) = self.collect_pending_assets();
                    effects.extend(new_effects);
                }
                Err(e) => {
                    Self::handle_query_error(&format!("LspFileDiagnostics({:?})", file), e);
                }
            }
        }

        // 3. Clear stale diagnostics for files no longer in target set
        let stale: Vec<_> = self
            .published_uris
            .difference(&current_uris)
            .cloned()
            .collect();
        for uri in stale {
            debug!("[LspCore] clearing stale diagnostics for {}", uri);
            if let Ok(parsed_uri) = uri.parse::<lsp_types::Uri>() {
                let params = PublishDiagnosticsParams {
                    uri: parsed_uri,
                    diagnostics: vec![],
                    version: None,
                };
                outputs.push(LspOutput::Notification {
                    method: PublishDiagnostics::METHOD.to_string(),
                    params: serde_json::to_value(params).unwrap(),
                });
            }
        }
        self.published_uris = current_uris;

        // 4. Remove subscriptions for files no longer tracked
        self.diagnostics_subscriptions
            .retain(|f, _| all_files.contains(f));

        (outputs, effects)
    }

    // === Asset Resolution ===

    /// Resolve a file asset with its content.
    ///
    /// Returns outputs (responses, notifications) and effects for any newly pending assets.
    pub fn resolve_file(
        &mut self,
        file: TextFile,
        content: Result<String, String>,
    ) -> (Vec<LspOutput>, Vec<Effect>) {
        // Resolve in runtime
        match content {
            Ok(text) => {
                self.runtime.resolve_asset(
                    file.clone(),
                    TextFileContent(text),
                    DurabilityLevel::Volatile,
                );
            }
            Err(error) => {
                self.runtime.resolve_asset_error::<TextFile>(
                    file.clone(),
                    anyhow::anyhow!("{}", error),
                    DurabilityLevel::Volatile,
                );
            }
        }
        self.pending_assets.remove(&file);

        // Process pending requests and diagnostics
        self.process_after_asset_change()
    }

    /// Resolve a glob pattern with matching files.
    ///
    /// Returns outputs (responses, notifications) and effects for any newly pending assets.
    pub fn resolve_glob(
        &mut self,
        id: &str,
        files: Vec<TextFile>,
    ) -> (Vec<LspOutput>, Vec<Effect>) {
        if let Some(glob_key) = self.pending_globs.remove(id) {
            self.runtime
                .resolve_asset(glob_key, GlobResult(files), DurabilityLevel::Volatile);
        }

        // Process pending requests and diagnostics
        self.process_after_asset_change()
    }

    /// Process pending requests and diagnostics after an asset is resolved.
    fn process_after_asset_change(&mut self) -> (Vec<LspOutput>, Vec<Effect>) {
        let mut outputs = Vec::new();
        let mut effects = Vec::new();

        // Retry pending requests
        let (req_outputs, req_effects) = self.retry_pending_requests();
        outputs.extend(req_outputs);
        effects.extend(req_effects);

        // Check diagnostics subscriptions
        let (diag_outputs, diag_effects) = self.check_diagnostics_subscriptions();
        outputs.extend(diag_outputs);
        effects.extend(diag_effects);

        (outputs, effects)
    }

    /// Retry pending requests after an asset was resolved.
    fn retry_pending_requests(&mut self) -> (Vec<LspOutput>, Vec<Effect>) {
        let mut outputs = Vec::new();
        let mut effects = Vec::new();

        let request_ids: Vec<CoreRequestId> = self.pending_requests.keys().cloned().collect();
        let mut completed_ids = Vec::new();

        for id in request_ids {
            if let Some(pending) = self.pending_requests.get(&id) {
                let command = pending.command.clone();

                match self.try_execute(&command) {
                    Ok(result) => {
                        let json = self.result_to_value(result);
                        outputs.push(LspOutput::Response {
                            id: id.clone(),
                            result: Ok(json),
                        });
                        completed_ids.push(id);
                    }
                    Err(QueryError::Suspend { .. }) => {
                        // Still waiting - collect more effects
                        let (new_effects, _) = self.collect_pending_assets();
                        effects.extend(new_effects);
                    }
                    Err(e) => {
                        if let Some(lsp_err) = Self::handle_query_error("RetryQuery", e) {
                            outputs.push(LspOutput::Response {
                                id: id.clone(),
                                result: Err(lsp_err),
                            });
                            completed_ids.push(id);
                        }
                    }
                }
            }
        }

        for id in completed_ids {
            self.pending_requests.remove(&id);
        }

        (outputs, effects)
    }

    /// Check diagnostics subscriptions and send updates.
    ///
    /// This simply calls `refresh_diagnostics` to re-poll all targets.
    fn check_diagnostics_subscriptions(&mut self) -> (Vec<LspOutput>, Vec<Effect>) {
        self.refresh_diagnostics()
    }

    // === Internal Helpers ===

    /// Log a QueryError and convert it to an LspError.
    /// Returns None for Suspend (should be handled separately).
    fn handle_query_error(context: &str, err: QueryError) -> Option<LspError> {
        match err {
            QueryError::Suspend { .. } => None,
            QueryError::Cancelled => {
                error!("{}: query unexpectedly cancelled", context);
                Some(LspError::internal_error("Query cancelled"))
            }
            QueryError::DependenciesRemoved { missing_keys } => {
                error!("{}: dependencies removed: {:?}", context, missing_keys);
                Some(LspError::internal_error("Dependencies removed"))
            }
            QueryError::Cycle { path } => {
                error!("{}: query cycle: {:?}", context, path);
                Some(LspError::internal_error(format!("Query cycle: {:?}", path)))
            }
            QueryError::InconsistentAssetResolution => {
                unreachable!("InconsistentAssetResolution should not occur")
            }
            QueryError::UserError(e) => {
                error!("{}: unexpected user error: {}", context, e);
                Some(LspError::internal_error(e.to_string()))
            }
        }
    }

    /// Try to execute a command query.
    fn try_execute(&mut self, command: &CommandQuery) -> Result<CommandResult, QueryError> {
        match command {
            CommandQuery::SemanticTokensFull(query) => {
                let result = self.runtime.query(query.clone())?;
                Ok(CommandResult::SemanticTokens(Some((*result).clone())))
            }
            CommandQuery::Completion(request) => {
                let items = lsp_completion(&self.runtime, &request.file, request.offset)?;
                Ok(CommandResult::Completion(items))
            }
            CommandQuery::Definition(request) => {
                let source = self.runtime.asset(request.file.clone())?;
                let offset = position_to_offset(source.get(), request.position) as u32;
                Ok(CommandResult::Definition(lsp_definition(
                    &self.runtime,
                    &request.file,
                    offset,
                )?))
            }
            CommandQuery::SchemaContent(file) => Ok(CommandResult::SchemaContent(
                self.runtime.asset(file.clone())?.get().to_string(),
            )),
            CommandQuery::Hover(request) => {
                let hover = lsp_hover(&self.runtime, &request.file, request.offset)?;
                Ok(CommandResult::Hover(hover))
            }
        }
    }

    /// Convert a command result to a JSON value.
    fn result_to_value(&self, result: CommandResult) -> Value {
        match result {
            CommandResult::SemanticTokens(tokens) => {
                serde_json::to_value(tokens).unwrap_or(Value::Null)
            }
            CommandResult::Completion(items) => {
                serde_json::to_value(CompletionResponse::Array(items)).unwrap_or(Value::Null)
            }
            CommandResult::SchemaContent(content) => Value::String(content),
            CommandResult::Definition(links) => {
                let response = if self.definition_link_support {
                    lsp_types::GotoDefinitionResponse::Link(links)
                } else {
                    lsp_types::GotoDefinitionResponse::Array(
                        links
                            .into_iter()
                            .map(|link| lsp_types::Location {
                                uri: link.target_uri,
                                range: link.target_selection_range,
                            })
                            .collect(),
                    )
                };
                serde_json::to_value(response).expect("definition locations are JSON serializable")
            }
            CommandResult::Hover(hover) => serde_json::to_value(hover).unwrap_or(Value::Null),
        }
    }

    /// Collect pending assets and return effects for the platform to handle.
    fn collect_pending_assets(&mut self) -> (Vec<Effect>, HashSet<TextFile>) {
        let mut effects = Vec::new();
        let mut waiting_for = HashSet::new();

        for pending in self.runtime.pending_assets() {
            if let Some(file) = pending.key::<TextFile>() {
                if !self.pending_assets.contains(file) {
                    self.pending_assets.insert(file.clone());
                    effects.push(Effect::FetchFile(file.clone()));
                }
                waiting_for.insert(file.clone());
            } else if let Some(glob_key) = pending.key::<Glob>() {
                // Generate a unique ID for this glob request
                let id = format!(
                    "{}:{}",
                    glob_key.base_dir.to_string_lossy(),
                    glob_key.pattern
                );
                if !self.pending_globs.contains_key(&id) {
                    self.pending_globs.insert(id.clone(), glob_key.clone());
                    effects.push(Effect::ExpandGlob {
                        id,
                        glob: glob_key.clone(),
                    });
                }
            }
        }

        (effects, waiting_for)
    }
}

impl Default for LspCore {
    fn default() -> Self {
        Self::new()
    }
}
