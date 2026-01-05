//! WASM API for the Eure Language Server.
//!
//! This module provides a WasmCore struct that can be used from JavaScript/TypeScript.
//! It uses an inbox/outbox pattern for LSP message handling.

use std::collections::{HashMap, HashSet};

use eure::query::{
    Glob, GlobResult, TextFile, TextFileContent, build_runtime, error::EureQueryError,
};
use js_sys::Array;
use lsp_types::{
    Diagnostic, DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    InitializeParams, InitializeResult, PublishDiagnosticsParams, SemanticTokensParams, Uri,
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification,
    },
    request::{Initialize, Request, SemanticTokensFullRequest, Shutdown},
};
use query_flow::{DurabilityLevel, QueryError, QueryRuntime, RevisionCounter};
use serde::Serialize;
use serde_json::Value;
use wasm_bindgen::prelude::*;

use crate::capabilities::server_capabilities;
use crate::queries::{LspDiagnostics, LspSemanticTokens};
use crate::uri_utils::{text_file_to_uri, uri_to_text_file as uri_to_text_file_from_str};

// =========================================================================
// WASM-exported types for TypeScript
// =========================================================================

/// Cache key information derived from a URL.
#[wasm_bindgen(getter_with_clone)]
pub struct CacheKeyInfo {
    pub url: String,
    pub hash: String,
    pub host: String,
    pub filename: String,
    pub cache_path: String,
}

impl From<eure_env::cache::CacheKeyInfo> for CacheKeyInfo {
    fn from(info: eure_env::cache::CacheKeyInfo) -> Self {
        Self {
            url: info.url,
            hash: info.hash,
            host: info.host,
            filename: info.filename,
            cache_path: info.cache_path,
        }
    }
}

/// Conditional headers for cache revalidation.
#[derive(Clone)]
#[wasm_bindgen(getter_with_clone)]
pub struct ConditionalHeaders {
    pub if_none_match: Option<String>,
    pub if_modified_since: Option<String>,
}

impl From<eure_env::cache::ConditionalHeaders> for ConditionalHeaders {
    fn from(headers: eure_env::cache::ConditionalHeaders) -> Self {
        Self {
            if_none_match: headers.if_none_match,
            if_modified_since: headers.if_modified_since,
        }
    }
}

/// Cache action result.
#[wasm_bindgen]
pub struct CacheAction {
    action: CacheActionKind,
    headers: Option<ConditionalHeaders>,
}

#[derive(Clone, Copy)]
#[wasm_bindgen]
pub enum CacheActionKind {
    Fetch,
    UseCached,
    Revalidate,
}

#[wasm_bindgen]
impl CacheAction {
    #[wasm_bindgen(getter)]
    pub fn action(&self) -> CacheActionKind {
        self.action
    }

    #[wasm_bindgen(getter)]
    pub fn headers(&self) -> Option<ConditionalHeaders> {
        self.headers.clone()
    }
}

impl From<eure_env::cache::CacheAction> for CacheAction {
    fn from(action: eure_env::cache::CacheAction) -> Self {
        match action {
            eure_env::cache::CacheAction::Fetch => Self {
                action: CacheActionKind::Fetch,
                headers: None,
            },
            eure_env::cache::CacheAction::UseCached => Self {
                action: CacheActionKind::UseCached,
                headers: None,
            },
            eure_env::cache::CacheAction::Revalidate { headers } => Self {
                action: CacheActionKind::Revalidate,
                headers: Some(headers.into()),
            },
        }
    }
}

// =========================================================================
// Internal types
// =========================================================================

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
    pending_globs: HashMap<String, Glob>,
    diagnostics_subscriptions: HashMap<String, DiagnosticsSubscription>,
    documents: HashMap<String, String>,
    initialized: bool,
}

#[wasm_bindgen]
impl WasmCore {
    /// Create a new WasmCore instance.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        // Set up panic hook to display panic messages in the console
        console_error_panic_hook::set_once();

        let runtime = build_runtime();

        Self {
            runtime,
            outbox: Vec::new(),
            pending_requests: HashMap::new(),
            pending_assets: HashSet::new(),
            pending_globs: HashMap::new(),
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

    /// Get pending text file URIs that need to be fetched.
    ///
    /// Returns a JavaScript array of URI strings.
    /// - Local files return file:// URIs
    /// - Remote files return https:// URLs
    #[wasm_bindgen]
    pub fn get_pending_text_files(&self) -> Array {
        self.pending_assets
            .iter()
            .map(|file| JsValue::from_str(&text_file_to_uri(file)))
            .collect()
    }

    /// Get pending glob patterns that need to be expanded.
    ///
    /// Returns a JavaScript array of objects with:
    /// - `id`: Unique identifier for this glob request
    /// - `base_dir`: Base directory for the pattern
    /// - `pattern`: Glob pattern relative to base_dir
    #[wasm_bindgen]
    pub fn get_pending_globs(&self) -> Array {
        self.pending_globs
            .iter()
            .map(|(id, glob)| {
                let obj = js_sys::Object::new();
                js_sys::Reflect::set(&obj, &"id".into(), &JsValue::from_str(id)).unwrap();
                js_sys::Reflect::set(
                    &obj,
                    &"base_dir".into(),
                    &JsValue::from_str(&glob.base_dir.to_string_lossy()),
                )
                .unwrap();
                js_sys::Reflect::set(&obj, &"pattern".into(), &JsValue::from_str(&glob.pattern))
                    .unwrap();
                JsValue::from(obj)
            })
            .collect()
    }

    /// Resolve a glob pattern with matching file paths.
    ///
    /// - `id`: The glob request ID from `get_pending_globs()`
    /// - `files`: Array of file URIs (file://) matching the pattern
    #[wasm_bindgen]
    pub fn resolve_glob(&mut self, id: &str, files: Array) {
        if let Some(glob_key) = self.pending_globs.remove(id) {
            let text_files: Vec<TextFile> = files
                .iter()
                .filter_map(|v| v.as_string())
                .map(|uri| uri_to_text_file_from_str(&uri))
                .collect();

            self.runtime
                .resolve_asset(glob_key, GlobResult(text_files), DurabilityLevel::Volatile);

            // Try to complete pending requests
            self.retry_pending_requests();

            // Check diagnostics subscriptions
            self.check_diagnostics_subscriptions();
        }
    }

    /// Resolve a text file content.
    ///
    /// - `uri`: The file URI (file://) or URL (https://)
    /// - `content`: The file content, or undefined/null if the file doesn't exist
    /// - `error`: Optional error message if content is null due to an error
    #[wasm_bindgen]
    pub fn resolve_text_file(&mut self, uri: &str, content: Option<String>, error: Option<String>) {
        // Parse URI to get TextFile
        let file = uri_to_text_file_from_str(uri);

        // Handle error case
        if let Some(error_msg) = error {
            // Log the error and fail pending requests
            self.handle_asset_error(&file, error_msg);
            return;
        }

        // Resolve in runtime
        match content {
            Some(s) => {
                self.runtime.resolve_asset(
                    file.clone(),
                    TextFileContent(s),
                    DurabilityLevel::Volatile,
                );
            }
            None => {
                self.runtime.resolve_asset_error::<TextFile>(
                    file.clone(),
                    EureQueryError::ContentNotFound(file.clone()),
                    DurabilityLevel::Volatile,
                );
            }
        }
        self.pending_assets.remove(&file);

        // Try to complete pending requests
        self.retry_pending_requests();

        // Check diagnostics subscriptions
        self.check_diagnostics_subscriptions();
    }

    /// Handle an asset fetch error.
    fn handle_asset_error(&mut self, file: &TextFile, error_msg: String) {
        self.pending_assets.remove(file);

        self.runtime.resolve_asset_error::<TextFile>(
            file.clone(),
            anyhow::anyhow!("{}", error_msg),
            DurabilityLevel::Volatile,
        );

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

    // =========================================================================
    // Cache Helper Functions (for WASM host to implement caching)
    // =========================================================================

    /// Compute cache key information from a URL.
    ///
    /// Returns cache key info with URL, hash, host, filename, and cache_path.
    /// Returns None if the URL is invalid.
    #[wasm_bindgen]
    pub fn compute_cache_key(&self, url_str: &str) -> Option<CacheKeyInfo> {
        use eure_env::cache::compute_cache_key;

        let url = url::Url::parse(url_str).ok()?;
        Some(compute_cache_key(&url).into())
    }

    /// Check cache status and determine what action to take.
    ///
    /// - `meta_json`: Optional JSON string of cached metadata (from .meta file)
    /// - `max_age_secs`: Maximum age in seconds before revalidation
    ///
    /// Returns CacheAction with action kind and optional conditional headers.
    #[wasm_bindgen]
    pub fn check_cache_status(&self, meta_json: Option<String>, max_age_secs: u32) -> CacheAction {
        use eure_env::cache::CacheMeta;

        let Some(meta_json) = meta_json else {
            return eure_env::cache::CacheAction::Fetch.into();
        };

        let Ok(meta) = serde_json::from_str::<CacheMeta>(&meta_json) else {
            return eure_env::cache::CacheAction::Fetch.into();
        };

        meta.check_freshness(max_age_secs).into()
    }

    /// Build cache metadata from fetch response.
    ///
    /// Arguments:
    /// - `url`: The fetched URL
    /// - `etag`: ETag header from response (optional)
    /// - `last_modified`: Last-Modified header from response (optional)
    /// - `content_hash`: SHA256 hash of content (computed by host)
    /// - `size_bytes`: Content size in bytes
    ///
    /// Returns a JSON string suitable for storing in .meta file.
    #[wasm_bindgen]
    pub fn build_cache_meta(
        &self,
        url: &str,
        etag: Option<String>,
        last_modified: Option<String>,
        content_hash: &str,
        size_bytes: u32,
    ) -> String {
        use eure_env::cache::CacheMeta;

        let meta = CacheMeta::new(
            url.to_string(),
            etag,
            last_modified,
            content_hash.to_string(),
            size_bytes as u64,
        );

        serde_json::to_string_pretty(&meta).unwrap_or_default()
    }

    /// Compute SHA256 hash of content.
    ///
    /// Returns the hash as a hex string.
    #[wasm_bindgen]
    pub fn compute_content_hash(&self, content: &str) -> String {
        eure_env::cache::compute_content_hash(content)
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
                    self.runtime.resolve_asset(
                        file,
                        TextFileContent(content.clone()),
                        DurabilityLevel::Volatile,
                    );

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
                        self.runtime.resolve_asset(
                            file,
                            TextFileContent(content.clone()),
                            DurabilityLevel::Volatile,
                        );

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
                                // Keep if URI doesn't match
                                let pending_uri = text_file_to_uri(&q.file);
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
                Ok(CommandResult::SemanticTokens(Some((*result).clone())))
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
            } else if let Some(glob_key) = pending.key::<Glob>() {
                // Generate a unique ID for this glob request
                let id = format!(
                    "{}:{}",
                    glob_key.base_dir.to_string_lossy(),
                    glob_key.pattern
                );
                self.pending_globs
                    .entry(id)
                    .or_insert_with(|| glob_key.clone());
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
                    CommandQuery::SemanticTokensFull(q) => text_file_to_uri(&q.file),
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
    uri_to_text_file_from_str(uri.as_str())
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
