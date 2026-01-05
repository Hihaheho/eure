//! Query execution and pending request management.

use std::collections::{HashMap, HashSet};

use crossbeam_channel::Receiver;
use eure::query::{Glob, GlobResult, TextFile, TextFileContent, build_runtime};
use lsp_server::{Notification, RequestId, Response};
use lsp_types::{
    Diagnostic, PublishDiagnosticsParams, SemanticTokens, notification::PublishDiagnostics,
};
use query_flow::{DurabilityLevel, QueryError, QueryRuntime, RevisionCounter};
use tracing::{info, warn};

use crate::io_pool::IoPool;
use crate::types::{CommandQuery, IoResponse, PendingRequest};
use eure_ls::queries::LspDiagnostics;

/// A subscription to a diagnostics query with revision tracking.
#[derive(Clone)]
struct DiagnosticsSubscription {
    query: LspDiagnostics,
    last_revision: RevisionCounter,
}

/// Result of executing a command.
pub enum CommandResult {
    SemanticTokens(Option<SemanticTokens>),
}

/// Query executor that manages the query runtime, pending requests, and IO.
pub struct QueryExecutor {
    runtime: QueryRuntime,
    pending: HashMap<RequestId, PendingRequest>,
    io_pool: IoPool,
    /// Track files that have been requested from IO but not yet resolved.
    pending_io: HashSet<TextFile>,
    /// Diagnostics subscriptions with revision tracking (keyed by URI string).
    diagnostics_subscriptions: HashMap<String, DiagnosticsSubscription>,
}

impl QueryExecutor {
    /// Create a new query executor.
    pub fn new(io_pool: IoPool) -> Self {
        let runtime = build_runtime();

        Self {
            runtime,
            pending: HashMap::new(),
            io_pool,
            pending_io: HashSet::new(),
            diagnostics_subscriptions: HashMap::new(),
        }
    }

    /// Get the receiver for IO responses.
    pub fn io_receiver(&self) -> &Receiver<IoResponse> {
        self.io_pool.receiver()
    }

    /// Try to execute a command.
    ///
    /// Returns `Some(Response)` if the command completed successfully.
    /// Returns `None` if the command is pending (waiting for assets).
    pub fn execute(
        &mut self,
        id: RequestId,
        command: CommandQuery,
        source: &str,
    ) -> Option<Response> {
        match self.try_execute(&command, source) {
            Ok(result) => Some(self.result_to_response(id, result)),
            Err(QueryError::Suspend { .. }) => {
                // Query is waiting for assets - add to pending
                let waiting_for = self.dispatch_pending_assets();
                self.pending.insert(
                    id,
                    PendingRequest {
                        command,
                        waiting_for,
                    },
                );
                None
            }
            Err(e) => Some(Response::new_err(
                id,
                lsp_server::ErrorCode::InternalError as i32,
                e.to_string(),
            )),
        }
    }

    /// Called when an asset (file) is resolved.
    ///
    /// Returns responses for pending requests and notifications for completed queries.
    pub fn on_asset_resolved(
        &mut self,
        file: TextFile,
        content: TextFileContent,
        sources: &HashMap<String, String>,
    ) -> (Vec<Response>, Vec<Notification>) {
        self.runtime
            .resolve_asset(file.clone(), content, DurabilityLevel::Volatile);
        self.pending_io.remove(&file);
        self.process_after_asset_change(&file, sources)
    }

    /// Called when fetching an asset fails.
    ///
    /// Returns responses for pending requests and notifications for completed queries.
    pub fn on_asset_error<E: Into<anyhow::Error>>(
        &mut self,
        file: TextFile,
        error: E,
        sources: &HashMap<String, String>,
    ) -> (Vec<Response>, Vec<Notification>) {
        self.runtime.resolve_asset_error::<TextFile>(
            file.clone(),
            error,
            DurabilityLevel::Volatile,
        );
        self.pending_io.remove(&file);
        self.process_after_asset_change(&file, sources)
    }

    /// Process pending requests and diagnostics after an asset is resolved or errored.
    fn process_after_asset_change(
        &mut self,
        file: &TextFile,
        sources: &HashMap<String, String>,
    ) -> (Vec<Response>, Vec<Notification>) {
        // Collect IDs of requests that were waiting for this file
        let waiting_ids: Vec<RequestId> = self
            .pending
            .iter()
            .filter(|(_, pending)| pending.waiting_for.contains(file))
            .map(|(id, _)| id.clone())
            .collect();

        // Try to complete pending requests
        let mut responses = Vec::new();
        let mut completed_ids = Vec::new();
        let mut needs_dispatch = false;

        for id in waiting_ids {
            if let Some(pending) = self.pending.get(&id) {
                let uri_str = file_to_uri(&pending.command);
                let source = sources.get(&uri_str).map(String::as_str).unwrap_or("");
                let command = pending.command.clone();

                match self.try_execute(&command, source) {
                    Ok(result) => {
                        responses.push(self.result_to_response(id.clone(), result));
                        completed_ids.push(id);
                    }
                    Err(QueryError::Suspend { .. }) => {
                        needs_dispatch = true;
                    }
                    Err(e) => {
                        responses.push(Response::new_err(
                            id.clone(),
                            lsp_server::ErrorCode::InternalError as i32,
                            e.to_string(),
                        ));
                        completed_ids.push(id);
                    }
                }
            }
        }

        for id in completed_ids {
            self.pending.remove(&id);
        }

        // Poll diagnostics subscriptions and notify on changes
        let subscription_uris: Vec<String> =
            self.diagnostics_subscriptions.keys().cloned().collect();
        let mut notifications = Vec::new();

        for uri_str in subscription_uris {
            if let Some(sub) = self.diagnostics_subscriptions.get(&uri_str).cloned() {
                match self.runtime.poll(sub.query.clone()) {
                    Ok(polled) => {
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
                                Err(e) => {
                                    warn!("Diagnostics error for {}: {}", uri_str, e);
                                    vec![]
                                }
                            };
                            if let Some(notif) =
                                make_diagnostics_notification(&uri_str, diagnostics)
                            {
                                notifications.push(notif);
                            }
                        }
                    }
                    Err(QueryError::Suspend { .. }) => {
                        needs_dispatch = true;
                    }
                    Err(e) => {
                        warn!("Diagnostics poll error for {}: {}", uri_str, e);
                        self.diagnostics_subscriptions.remove(&uri_str);
                    }
                }
            }
        }

        if needs_dispatch {
            self.dispatch_pending_assets();
        }

        (responses, notifications)
    }

    /// Cancel a pending request.
    pub fn cancel(&mut self, id: &RequestId) {
        if self.pending.remove(id).is_some() {
            info!("Cancelled pending request: {:?}", id);
        }
    }

    /// Execute a diagnostics query and return the diagnostics if changed.
    ///
    /// Returns `Some(diagnostics)` if the query completed and the result changed.
    /// Returns `None` if suspended or if the result hasn't changed since last poll.
    /// When suspended, the query is tracked and retried when assets resolve.
    pub fn get_diagnostics(
        &mut self,
        uri_str: String,
        query: LspDiagnostics,
    ) -> Option<Vec<Diagnostic>> {
        match self.runtime.poll(query.clone()) {
            Ok(polled) => {
                let last_revision = self
                    .diagnostics_subscriptions
                    .get(&uri_str)
                    .map(|s| s.last_revision);

                // Only return diagnostics if revision changed
                let changed = last_revision.is_none() || last_revision != Some(polled.revision);

                // Update subscription with new revision
                self.diagnostics_subscriptions.insert(
                    uri_str.clone(),
                    DiagnosticsSubscription {
                        query,
                        last_revision: polled.revision,
                    },
                );

                if changed {
                    match polled.value {
                        Ok(result) => Some((*result).clone()),
                        Err(e) => {
                            warn!("Diagnostics query error for {}: {}", uri_str, e);
                            Some(vec![])
                        }
                    }
                } else {
                    None
                }
            }
            Err(QueryError::Suspend { .. }) => {
                // Store subscription for retry when assets resolve
                let last_revision = self
                    .diagnostics_subscriptions
                    .get(&uri_str)
                    .map(|s| s.last_revision)
                    .unwrap_or_default();
                self.diagnostics_subscriptions.insert(
                    uri_str.clone(),
                    DiagnosticsSubscription {
                        query,
                        last_revision,
                    },
                );
                self.dispatch_pending_assets();
                None
            }
            Err(e) => {
                warn!("Diagnostics poll error for {}: {}", uri_str, e);
                None
            }
        }
    }

    /// Resolve a file directly (for open documents).
    pub fn resolve_open_document(&mut self, file: TextFile, content: String) {
        self.runtime
            .resolve_asset(file, TextFileContent(content), DurabilityLevel::Volatile);
    }

    /// Invalidate a file (e.g., when it's closed or changed externally).
    pub fn invalidate_file(&mut self, file: &TextFile) {
        self.runtime.invalidate_asset(file);
    }

    /// Unsubscribe from diagnostics for a document.
    pub fn unsubscribe_diagnostics(&mut self, uri_str: &str) {
        self.diagnostics_subscriptions.remove(uri_str);
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

    /// Dispatch any pending assets to the IO pool.
    ///
    /// Returns the set of files being waited for.
    fn dispatch_pending_assets(&mut self) -> HashSet<TextFile> {
        let mut waiting_for = HashSet::new();

        for pending in self.runtime.pending_assets() {
            if let Some(file) = pending.key::<TextFile>() {
                if !self.pending_io.contains(file) {
                    self.io_pool.request_file(file.clone());
                    self.pending_io.insert(file.clone());
                }
                waiting_for.insert(file.clone());
            } else if let Some(glob_key) = pending.key::<Glob>() {
                // Expand glob pattern on filesystem
                let pattern = glob_key.full_pattern();
                let pattern_str = pattern.to_string_lossy();
                let files: Vec<TextFile> = glob::glob(&pattern_str)
                    .into_iter()
                    .flat_map(|paths| paths.flatten().map(TextFile::from_path))
                    .collect();
                self.runtime.resolve_asset(
                    glob_key.clone(),
                    GlobResult(files),
                    DurabilityLevel::Volatile,
                );
            }
        }

        waiting_for
    }

    /// Convert a command result to an LSP response.
    fn result_to_response(&self, id: RequestId, result: CommandResult) -> Response {
        match result {
            CommandResult::SemanticTokens(tokens) => Response::new_ok(id, tokens),
        }
    }
}

/// Extract the file URI string from a command query.
fn file_to_uri(command: &CommandQuery) -> String {
    let file = match command {
        CommandQuery::SemanticTokensFull(query) => &query.file,
    };
    match file {
        TextFile::Local(path) => format!("file://{}", path.display()),
        TextFile::Remote(url) => url.to_string(),
    }
}

/// Create a diagnostics notification from URI string and diagnostics.
fn make_diagnostics_notification(
    uri_str: &str,
    diagnostics: Vec<Diagnostic>,
) -> Option<Notification> {
    use lsp_types::notification::Notification as _;

    let uri = uri_str.parse().ok()?;
    let params = PublishDiagnosticsParams {
        uri,
        diagnostics,
        version: None,
    };
    Some(Notification::new(
        PublishDiagnostics::METHOD.to_string(),
        params,
    ))
}
