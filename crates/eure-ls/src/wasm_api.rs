//! WASM API for the Eure Language Server.
//!
//! This module provides a WasmCore struct that can be used from JavaScript/TypeScript.
//! It uses an inbox/outbox pattern for LSP message handling.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use eure_editor_support::assets::{TextFile, TextFileContent};
use eure_editor_support::error_reports_comparator;
use js_sys::Array;
use lsp_types::{
    Diagnostic, DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    InitializeParams, InitializeResult, PublishDiagnosticsParams, SemanticTokensParams, Uri,
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification,
    },
    request::{Initialize, Request, SemanticTokensFullRequest, Shutdown},
};
use query_flow::{QueryError, QueryRuntime, QueryRuntimeBuilder, RevisionCounter};
use serde::Serialize;
use serde_json::Value;
use wasm_bindgen::prelude::*;

use crate::capabilities::server_capabilities;
use crate::queries::{LspDiagnostics, LspSemanticTokens};

/// Subscription for diagnostics with revision tracking.
#[derive(Clone)]
struct DiagnosticsSubscription {
    query: LspDiagnostics,
    last_revision: RevisionCounter,
}

/// Result of executing a command query.
enum CommandResult {
    SemanticTokens(Option<lsp_types::SemanticTokens>),
}

/// Command query for LSP requests.
#[derive(Clone)]
enum CommandQuery {
    SemanticTokensFull(LspSemanticTokens),
}

/// A pending request waiting for assets.
struct PendingRequest {
    id: String,
    command: CommandQuery,
    #[allow(dead_code)]
    waiting_for: HashSet<TextFile>,
}

/// WASM-compatible Language Server core.
///
/// This struct handles LSP messages using an inbox/outbox pattern,
/// allowing the TypeScript event loop to drive I/O.
#[wasm_bindgen]
pub struct WasmCore {
    runtime: QueryRuntime,
    outbox: Vec<Value>,
    pending_requests: HashMap<String, PendingRequest>,
    pending_assets: HashSet<TextFile>,
    diagnostics_subscriptions: HashMap<String, DiagnosticsSubscription>,
    documents: HashMap<String, String>,
    initialized: bool,
}

#[wasm_bindgen]
impl WasmCore {
    /// Create a new WasmCore instance.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let runtime = QueryRuntimeBuilder::new()
            .error_comparator(error_reports_comparator)
            .build();

        Self {
            runtime,
            outbox: Vec::new(),
            pending_requests: HashMap::new(),
            pending_assets: HashSet::new(),
            diagnostics_subscriptions: HashMap::new(),
            documents: HashMap::new(),
            initialized: false,
        }
    }

    /// Handle an incoming LSP message.
    ///
    /// The message should be a JSON-RPC message (request, notification, or response).
    #[wasm_bindgen]
    pub fn handle_message(&mut self, msg: JsValue) {
        let msg: Value = match serde_wasm_bindgen::from_value(msg) {
            Ok(v) => v,
            Err(_) => return,
        };

        // Determine message type
        if let Some(id) = msg.get("id") {
            if msg.get("method").is_some() {
                // Request
                self.handle_request(&msg, id);
            }
            // Response - we don't send requests, so ignore responses
        } else if msg.get("method").is_some() {
            // Notification
            self.handle_notification(&msg);
        }
    }

    /// Drain the outbox and return all pending outgoing messages.
    ///
    /// Returns a JavaScript array of JSON-RPC messages.
    #[wasm_bindgen]
    pub fn drain_outbox(&mut self) -> Array {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        let messages: Vec<JsValue> = self
            .outbox
            .drain(..)
            .filter_map(|v| v.serialize(&serializer).ok())
            .collect();
        messages.into_iter().collect()
    }

    /// Get pending asset URIs that need to be fetched.
    ///
    /// Returns a JavaScript array of URI strings.
    #[wasm_bindgen]
    pub fn get_pending_assets(&self) -> Array {
        self.pending_assets
            .iter()
            .filter_map(|file| {
                // Convert TextFile path to file:// URI
                let path = file.path.display().to_string();
                let uri = if path.starts_with('/') {
                    format!("file://{}", path)
                } else {
                    format!("file:///{}", path)
                };
                Some(JsValue::from_str(&uri))
            })
            .collect()
    }

    /// Resolve an asset (file content).
    ///
    /// - `uri`: The file URI (file://)
    /// - `content`: The file content, or undefined/null if the file doesn't exist
    #[wasm_bindgen]
    pub fn resolve_asset(&mut self, uri: &str, content: Option<String>) {
        // Parse URI to get path
        let path = uri_to_path(uri);
        let file = TextFile::from_path(PathBuf::from(path));

        // Resolve in runtime
        let content = match content {
            Some(s) => TextFileContent::Content(s),
            None => TextFileContent::NotFound,
        };
        self.runtime.resolve_asset(file.clone(), content);
        self.pending_assets.remove(&file);

        // Try to complete pending requests
        self.retry_pending_requests();

        // Check diagnostics subscriptions
        self.check_diagnostics_subscriptions();
    }

    /// Tick the event loop.
    ///
    /// This should be called periodically to process any internal state changes.
    #[wasm_bindgen]
    pub fn tick(&mut self) {
        // Currently a no-op, but could be used for periodic tasks
        // like garbage collection or timeout handling
    }
}

impl WasmCore {
    /// Handle an LSP request.
    fn handle_request(&mut self, msg: &Value, id: &Value) {
        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(Value::Null);
        let id_str = normalize_request_id(id);

        match method {
            Initialize::METHOD => {
                let _params: InitializeParams = match serde_json::from_value(params) {
                    Ok(p) => p,
                    Err(e) => {
                        self.send_error(id, -32602, &format!("Invalid params: {}", e));
                        return;
                    }
                };

                let result = InitializeResult {
                    capabilities: server_capabilities(),
                    server_info: Some(lsp_types::ServerInfo {
                        name: "eure-ls".to_string(),
                        version: Some(env!("CARGO_PKG_VERSION").to_string()),
                    }),
                };

                self.initialized = true;
                self.send_response(id, serde_json::to_value(result).unwrap());
            }
            Shutdown::METHOD => {
                self.send_response(id, Value::Null);
            }
            SemanticTokensFullRequest::METHOD => {
                let params: SemanticTokensParams = match serde_json::from_value(params) {
                    Ok(p) => p,
                    Err(e) => {
                        self.send_error(id, -32602, &format!("Invalid params: {}", e));
                        return;
                    }
                };

                let uri = params.text_document.uri;
                let uri_str = uri.as_str();
                let file = uri_to_text_file(&uri);
                let source = self.documents.get(uri_str).cloned().unwrap_or_default();

                let query = LspSemanticTokens::new(file, source.clone());
                let command = CommandQuery::SemanticTokensFull(query);

                match self.try_execute(&command, &source) {
                    Ok(result) => {
                        let response = self.result_to_value(result);
                        self.send_response(id, response);
                    }
                    Err(QueryError::Suspend { .. }) => {
                        // Query is pending - add to pending requests
                        let waiting_for = self.collect_pending_assets();
                        self.pending_requests.insert(
                            id_str,
                            PendingRequest {
                                id: id.to_string(),
                                command,
                                waiting_for,
                            },
                        );
                    }
                    Err(e) => {
                        self.send_error(id, -32603, &e.to_string());
                    }
                }
            }
            _ => {
                self.send_error(id, -32601, &format!("Method not found: {}", method));
            }
        }
    }

    /// Handle an LSP notification.
    fn handle_notification(&mut self, msg: &Value) {
        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(Value::Null);

        match method {
            DidOpenTextDocument::METHOD => {
                if let Ok(params) = serde_json::from_value::<DidOpenTextDocumentParams>(params) {
                    let uri = params.text_document.uri;
                    let content = params.text_document.text;

                    // Update document cache
                    self.documents
                        .insert(uri.as_str().to_string(), content.clone());

                    // Resolve in query runtime
                    let file = uri_to_text_file(&uri);
                    self.runtime
                        .resolve_asset(file, TextFileContent::Content(content.clone()));

                    // Publish diagnostics
                    self.publish_diagnostics(&uri, &content);
                }
            }
            DidChangeTextDocument::METHOD => {
                if let Ok(params) = serde_json::from_value::<DidChangeTextDocumentParams>(params) {
                    let uri = params.text_document.uri;
                    if let Some(change) = params.content_changes.into_iter().next() {
                        let content = change.text;

                        // Update document cache
                        self.documents
                            .insert(uri.as_str().to_string(), content.clone());

                        // Resolve in query runtime
                        let file = uri_to_text_file(&uri);
                        self.runtime
                            .resolve_asset(file, TextFileContent::Content(content.clone()));

                        // Publish diagnostics
                        self.publish_diagnostics(&uri, &content);
                    }
                }
            }
            DidCloseTextDocument::METHOD => {
                if let Ok(params) = serde_json::from_value::<DidCloseTextDocumentParams>(params) {
                    let uri = params.text_document.uri;
                    let uri_str = uri.as_str();

                    // Remove from document cache
                    self.documents.remove(uri_str);

                    // Unsubscribe from diagnostics
                    self.diagnostics_subscriptions.remove(uri_str);

                    // Also remove any cancelled requests for this document
                    self.pending_requests.retain(|_, pending| {
                        match &pending.command {
                            CommandQuery::SemanticTokensFull(q) => {
                                // Keep if path doesn't match
                                let pending_uri = format!("file://{}", q.file.path.display());
                                pending_uri != uri_str
                            }
                        }
                    });

                    // Invalidate in query runtime
                    let file = uri_to_text_file(&uri);
                    self.runtime.invalidate_asset(&file);

                    // Clear diagnostics
                    self.send_diagnostics(&uri, vec![]);
                }
            }
            "$/cancelRequest" => {
                if let Some(id) = params.get("id") {
                    let id_str = normalize_request_id(id);
                    self.pending_requests.remove(&id_str);
                }
            }
            "initialized" | "exit" => {
                // Ignore
            }
            _ => {
                // Unknown notification - ignore
            }
        }
    }

    /// Try to execute a command query.
    fn try_execute(
        &mut self,
        command: &CommandQuery,
        _source: &str,
    ) -> Result<CommandResult, QueryError> {
        match command {
            CommandQuery::SemanticTokensFull(query) => {
                let result = self.runtime.query(query.clone())?;
                Ok(CommandResult::SemanticTokens((*result).clone()))
            }
        }
    }

    /// Convert a command result to JSON value.
    fn result_to_value(&self, result: CommandResult) -> Value {
        match result {
            CommandResult::SemanticTokens(tokens) => {
                serde_json::to_value(tokens).unwrap_or(Value::Null)
            }
        }
    }

    /// Collect pending assets and request them.
    fn collect_pending_assets(&mut self) -> HashSet<TextFile> {
        let mut waiting_for = HashSet::new();

        for pending in self.runtime.pending_assets() {
            if let Some(file) = pending.key::<TextFile>() {
                if !self.pending_assets.contains(file) {
                    self.pending_assets.insert(file.clone());
                }
                waiting_for.insert(file.clone());
            }
        }

        waiting_for
    }

    /// Retry pending requests after an asset was resolved.
    fn retry_pending_requests(&mut self) {
        let request_ids: Vec<String> = self.pending_requests.keys().cloned().collect();
        let mut completed_ids = Vec::new();

        for id_str in request_ids {
            if let Some(pending) = self.pending_requests.get(&id_str) {
                let uri_str = match &pending.command {
                    CommandQuery::SemanticTokensFull(q) => {
                        format!("file://{}", q.file.path.display())
                    }
                };
                let source = self.documents.get(&uri_str).cloned().unwrap_or_default();
                let command = pending.command.clone();
                let id: Value = serde_json::from_str(&pending.id).unwrap_or(Value::Null);

                match self.try_execute(&command, &source) {
                    Ok(result) => {
                        let response = self.result_to_value(result);
                        self.send_response(&id, response);
                        completed_ids.push(id_str);
                    }
                    Err(QueryError::Suspend { .. }) => {
                        // Still waiting - update waiting_for
                        self.collect_pending_assets();
                    }
                    Err(e) => {
                        self.send_error(&id, -32603, &e.to_string());
                        completed_ids.push(id_str);
                    }
                }
            }
        }

        for id in completed_ids {
            self.pending_requests.remove(&id);
        }
    }

    /// Check diagnostics subscriptions and send updates.
    fn check_diagnostics_subscriptions(&mut self) {
        let subscription_uris: Vec<String> =
            self.diagnostics_subscriptions.keys().cloned().collect();

        for uri_str in subscription_uris {
            if let Some(sub) = self.diagnostics_subscriptions.get(&uri_str).cloned() {
                match self.runtime.poll(sub.query.clone()) {
                    Ok(polled) => {
                        // Only send if revision changed
                        if polled.revision != sub.last_revision {
                            self.diagnostics_subscriptions.insert(
                                uri_str.clone(),
                                DiagnosticsSubscription {
                                    query: sub.query,
                                    last_revision: polled.revision,
                                },
                            );
                            let diagnostics = match polled.value {
                                Ok(result) => (*result).clone(),
                                Err(_) => vec![],
                            };
                            if let Ok(uri) = uri_str.parse() {
                                self.send_diagnostics(&uri, diagnostics);
                            }
                        }
                    }
                    Err(QueryError::Suspend { .. }) => {
                        self.collect_pending_assets();
                    }
                    Err(_) => {
                        self.diagnostics_subscriptions.remove(&uri_str);
                    }
                }
            }
        }
    }

    /// Publish diagnostics for a document.
    fn publish_diagnostics(&mut self, uri: &Uri, source: &str) {
        let uri_str = uri.as_str().to_string();
        let file = uri_to_text_file(uri);
        let query = LspDiagnostics::new(file, source.to_string());

        match self.runtime.poll(query.clone()) {
            Ok(polled) => {
                let last_revision = self
                    .diagnostics_subscriptions
                    .get(&uri_str)
                    .map(|s| s.last_revision);

                let changed = last_revision.is_none() || last_revision != Some(polled.revision);

                // Update subscription
                self.diagnostics_subscriptions.insert(
                    uri_str.clone(),
                    DiagnosticsSubscription {
                        query,
                        last_revision: polled.revision,
                    },
                );

                if changed {
                    let diagnostics = match polled.value {
                        Ok(result) => (*result).clone(),
                        Err(_) => vec![],
                    };
                    self.send_diagnostics(uri, diagnostics);
                }
            }
            Err(QueryError::Suspend { .. }) => {
                // Store subscription for retry
                let last_revision = self
                    .diagnostics_subscriptions
                    .get(&uri_str)
                    .map(|s| s.last_revision)
                    .unwrap_or_default();
                self.diagnostics_subscriptions.insert(
                    uri_str,
                    DiagnosticsSubscription {
                        query,
                        last_revision,
                    },
                );
                self.collect_pending_assets();
            }
            Err(_) => {
                // Error - ignore
            }
        }
    }

    /// Send a diagnostics notification.
    fn send_diagnostics(&mut self, uri: &Uri, diagnostics: Vec<Diagnostic>) {
        let params = PublishDiagnosticsParams {
            uri: uri.clone(),
            diagnostics,
            version: None,
        };

        self.send_notification(
            "textDocument/publishDiagnostics",
            serde_json::to_value(params).unwrap(),
        );
    }

    /// Send a JSON-RPC response.
    fn send_response(&mut self, id: &Value, result: Value) {
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        });
        self.outbox.push(response);
    }

    /// Send a JSON-RPC error response.
    fn send_error(&mut self, id: &Value, code: i32, message: &str) {
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": code,
                "message": message
            }
        });
        self.outbox.push(response);
    }

    /// Send a JSON-RPC notification.
    fn send_notification(&mut self, method: &str, params: Value) {
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });
        self.outbox.push(notification);
    }
}

impl Default for WasmCore {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert an LSP URI to a TextFile.
fn uri_to_text_file(uri: &Uri) -> TextFile {
    let path_str = uri.path().as_str();
    let decoded = percent_decode(path_str);
    TextFile::from_path(PathBuf::from(decoded))
}

/// Extract path from a file:// URI.
fn uri_to_path(uri: &str) -> String {
    let path = if let Some(stripped) = uri.strip_prefix("file:///") {
        // Windows-style: file:///C:/path
        stripped.to_string()
    } else if let Some(stripped) = uri.strip_prefix("file://") {
        // Unix-style: file:///path -> /path
        stripped.to_string()
    } else {
        uri.to_string()
    };
    percent_decode(&path)
}

/// Percent-decode a URI path.
fn percent_decode(s: &str) -> String {
    percent_encoding::percent_decode_str(s)
        .decode_utf8_lossy()
        .into_owned()
}

/// Normalize request ID to string for HashMap keys.
///
/// LSP allows request IDs to be either numbers or strings.
fn normalize_request_id(id: &Value) -> String {
    match id {
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        _ => id.to_string(),
    }
}
