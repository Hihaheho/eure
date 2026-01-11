//! WASM API for the Eure Language Server.
//!
//! This module provides a WasmCore struct that can be used from JavaScript/TypeScript.
//! It uses an inbox/outbox pattern for LSP message handling.

use eure::query::TextFile;
use js_sys::Array;
use serde::Serialize;
use serde_json::Value;
use wasm_bindgen::prelude::*;

use crate::uri_utils::{text_file_to_uri, uri_to_text_file};
use crate::{CoreRequestId, LspCore, LspOutput};

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
// WasmCore - thin wrapper around LspCore
// =========================================================================

/// WASM-compatible Language Server wrapper.
///
/// This struct wraps LspCore and provides the inbox/outbox pattern
/// for TypeScript integration.
#[wasm_bindgen]
pub struct WasmCore {
    core: LspCore,
    outbox: Vec<Value>,
}

#[wasm_bindgen]
impl WasmCore {
    /// Create a new WasmCore instance.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        // Set up panic hook to display panic messages in the console
        console_error_panic_hook::set_once();

        Self {
            core: LspCore::new(),
            outbox: Vec::new(),
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
                let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
                let params = msg.get("params").cloned().unwrap_or(Value::Null);
                let core_id = CoreRequestId::from(id);

                let (outputs, _effects) = self.core.handle_request(core_id, method, params);
                self.process_outputs(outputs);
                // Effects are handled by TypeScript polling pending_files/pending_globs
            }
            // Response - we don't send requests, so ignore responses
        } else if msg.get("method").is_some() {
            // Notification
            let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
            let params = msg.get("params").cloned().unwrap_or(Value::Null);

            let (outputs, _effects) = self.core.handle_notification(method, params);
            self.process_outputs(outputs);
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
    #[wasm_bindgen]
    pub fn get_pending_text_files(&self) -> Array {
        self.core
            .pending_files()
            .map(|file| JsValue::from_str(&text_file_to_uri(file)))
            .collect()
    }

    /// Get pending glob patterns that need to be expanded.
    ///
    /// Returns a JavaScript array of objects with id, base_dir, pattern.
    #[wasm_bindgen]
    pub fn get_pending_globs(&self) -> Array {
        self.core
            .pending_globs()
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
    #[wasm_bindgen]
    pub fn resolve_glob(&mut self, id: &str, files: Array) {
        let text_files: Vec<TextFile> = files
            .iter()
            .filter_map(|v| v.as_string())
            .map(|uri| uri_to_text_file(&uri))
            .collect();

        let (outputs, _effects) = self.core.resolve_glob(id, text_files);
        self.process_outputs(outputs);
    }

    /// Resolve a text file content.
    #[wasm_bindgen]
    pub fn resolve_text_file(&mut self, uri: &str, content: Option<String>, error: Option<String>) {
        let file = uri_to_text_file(uri);

        let result = match (content, error) {
            (Some(c), _) => Ok(c),
            (_, Some(e)) => Err(e),
            (None, None) => Err("File not found".to_string()),
        };

        let (outputs, _effects) = self.core.resolve_file(file, result);
        self.process_outputs(outputs);
    }

    /// Tick the event loop.
    #[wasm_bindgen]
    pub fn tick(&mut self) {
        // Currently a no-op
    }

    // =========================================================================
    // Cache Helper Functions
    // =========================================================================

    /// Compute cache key information from a URL.
    #[wasm_bindgen]
    pub fn compute_cache_key(&self, url_str: &str) -> Option<CacheKeyInfo> {
        use eure_env::cache::compute_cache_key;

        let url = url::Url::parse(url_str).ok()?;
        Some(compute_cache_key(&url).into())
    }

    /// Check cache status and determine what action to take.
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
    #[wasm_bindgen]
    pub fn compute_content_hash(&self, content: &str) -> String {
        eure_env::cache::compute_content_hash(content)
    }
}

impl WasmCore {
    /// Process LspOutputs and add to outbox.
    fn process_outputs(&mut self, outputs: Vec<LspOutput>) {
        for output in outputs {
            let json = match output {
                LspOutput::Response { id, result } => match result {
                    Ok(value) => serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": value
                    }),
                    Err(err) => serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": err.code,
                            "message": err.message
                        }
                    }),
                },
                LspOutput::Notification { method, params } => serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": method,
                    "params": params
                }),
            };
            self.outbox.push(json);
        }
    }
}

impl Default for WasmCore {
    fn default() -> Self {
        Self::new()
    }
}
