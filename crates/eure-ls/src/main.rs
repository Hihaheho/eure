//! Eure Language Server - LSP implementation for the Eure data format.

pub mod executor;
pub mod io_pool;
pub mod types;

use std::collections::HashMap;
use std::path::PathBuf;

use crate::executor::QueryExecutor;
use crate::io_pool::IoPool;
use crate::types::CommandQuery;
use anyhow::Result;
use crossbeam_channel::select;
use eure::query::TextFile;
use eure_ls::{LspDiagnostics, LspSemanticTokens, server_capabilities};
use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    InitializeParams, PublishDiagnosticsParams, SemanticTokensParams, Uri,
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
        PublishDiagnostics,
    },
    request::{Request as _, SemanticTokensFullRequest},
};
use tracing::{error, info};

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("eure_ls=info".parse().unwrap()),
        )
        .with_writer(std::io::stderr)
        .init();

    info!("Starting Eure Language Server");

    // Create LSP connection
    let (connection, io_threads) = Connection::stdio();

    // Initialize LSP
    let server_capabilities = serde_json::to_value(server_capabilities())?;
    let init_params = connection.initialize(server_capabilities)?;
    let init_params: InitializeParams = serde_json::from_value(init_params)?;

    info!("Eure Language Server initialized");

    // Create components
    let io_pool = IoPool::new(4);
    let mut executor = QueryExecutor::new(io_pool);

    // Register workspaces from initialization
    executor.register_workspaces_from_init(&init_params);

    // Track open documents and their content (keyed by URI string)
    let mut documents: HashMap<String, String> = HashMap::new();

    // Main event loop
    loop {
        select! {
            recv(connection.receiver) -> msg => {
                match msg {
                    Ok(Message::Request(req)) => {
                        if connection.handle_shutdown(&req)? {
                            info!("Shutdown requested");
                            break;
                        }
                        handle_request(req, &connection, &mut executor, &documents);
                    }
                    Ok(Message::Notification(not)) => {
                        handle_notification(not, &connection, &mut executor, &mut documents);
                    }
                    Ok(Message::Response(_)) => {
                        // We don't send requests, so we shouldn't receive responses
                    }
                    Err(e) => {
                        error!("Error receiving message: {}", e);
                        break;
                    }
                }
            }
            recv(executor.io_receiver()) -> response => {
                if let Ok(response) = response {
                    match response.result {
                        Ok(content) => {
                            let (responses, notifications) = executor.on_asset_resolved(
                                response.file,
                                content,
                                &documents,
                            );
                            for resp in responses {
                                connection.sender.send(Message::Response(resp))?;
                            }
                            for notif in notifications {
                                connection.sender.send(Message::Notification(notif))?;
                            }
                        }
                        Err(e) => {
                            let (responses, notifications) =
                                executor.on_asset_error(response.file, e, &documents);
                            for resp in responses {
                                connection.sender.send(Message::Response(resp))?;
                            }
                            for notif in notifications {
                                connection.sender.send(Message::Notification(notif))?;
                            }
                        }
                    }
                }
            }
        }
    }

    io_threads.join()?;
    info!("Eure Language Server stopped");
    Ok(())
}

/// Handle an incoming LSP request.
fn handle_request(
    req: Request,
    connection: &Connection,
    executor: &mut QueryExecutor,
    documents: &HashMap<String, String>,
) {
    if let Some(response) = dispatch_request(req, executor, documents)
        && let Err(e) = connection.sender.send(Message::Response(response))
    {
        error!("Failed to send response: {}", e);
    }
    // If dispatch_request returns None, the request is pending
}

/// Dispatch a request to the appropriate handler.
fn dispatch_request(
    req: Request,
    executor: &mut QueryExecutor,
    documents: &HashMap<String, String>,
) -> Option<Response> {
    match req.method.as_str() {
        SemanticTokensFullRequest::METHOD => {
            let params: SemanticTokensParams = serde_json::from_value(req.params).ok()?;
            let uri = params.text_document.uri;
            let uri_str = uri.as_str();
            let file = uri_to_text_file(&uri);
            let source = documents.get(uri_str).map(String::as_str).unwrap_or("");

            let query = LspSemanticTokens::new(file, source.to_string());
            let command = CommandQuery::SemanticTokensFull(query);

            executor.execute(req.id, command, source)
        }
        _ => {
            // Unknown request - return method not found
            Some(Response::new_err(
                req.id,
                lsp_server::ErrorCode::MethodNotFound as i32,
                format!("Unknown method: {}", req.method),
            ))
        }
    }
}

/// Handle an incoming LSP notification.
fn handle_notification(
    not: Notification,
    connection: &Connection,
    executor: &mut QueryExecutor,
    documents: &mut HashMap<String, String>,
) {
    match not.method.as_str() {
        DidOpenTextDocument::METHOD => {
            if let Ok(params) = serde_json::from_value::<DidOpenTextDocumentParams>(not.params) {
                let uri = params.text_document.uri;
                let content = params.text_document.text;

                // Update document cache
                documents.insert(uri.as_str().to_string(), content.clone());

                // Resolve in query runtime
                let file = uri_to_text_file(&uri);
                executor.resolve_open_document(file.clone(), content.clone());

                // Publish diagnostics
                publish_diagnostics(connection, executor, &uri, &content);
            }
        }
        DidChangeTextDocument::METHOD => {
            if let Ok(params) = serde_json::from_value::<DidChangeTextDocumentParams>(not.params) {
                let uri = params.text_document.uri;
                // We use FULL sync, so there's only one change with the full content
                if let Some(change) = params.content_changes.into_iter().next() {
                    let content = change.text;

                    // Update document cache
                    documents.insert(uri.as_str().to_string(), content.clone());

                    // Resolve in query runtime
                    let file = uri_to_text_file(&uri);
                    executor.resolve_open_document(file.clone(), content.clone());

                    // Publish diagnostics
                    publish_diagnostics(connection, executor, &uri, &content);
                }
            }
        }
        DidCloseTextDocument::METHOD => {
            if let Ok(params) = serde_json::from_value::<DidCloseTextDocumentParams>(not.params) {
                let uri = params.text_document.uri;
                let uri_str = uri.as_str();

                // Remove from document cache
                documents.remove(uri_str);

                // Unsubscribe from diagnostics
                executor.unsubscribe_diagnostics(uri_str);

                // Invalidate in query runtime
                let file = uri_to_text_file(&uri);
                executor.invalidate_file(&file);

                // Clear diagnostics
                let params = PublishDiagnosticsParams {
                    uri,
                    diagnostics: vec![],
                    version: None,
                };
                let notification =
                    Notification::new(PublishDiagnostics::METHOD.to_string(), params);
                if let Err(e) = connection.sender.send(Message::Notification(notification)) {
                    error!("Failed to send diagnostics: {}", e);
                }
            }
        }
        "$/cancelRequest" => {
            if let Ok(params) = serde_json::from_value::<CancelParams>(not.params) {
                executor.cancel(&params.id);
            }
        }
        _ => {
            // Unknown notification - ignore
        }
    }
}

/// Publish diagnostics for a document (and any related files like schemas).
fn publish_diagnostics(
    connection: &Connection,
    executor: &mut QueryExecutor,
    uri: &Uri,
    _source: &str,
) {
    let uri_str = uri.as_str().to_string();
    let file = uri_to_text_file(uri);
    let query = LspDiagnostics::new(file);

    // Execute the diagnostics query - returns notifications for all affected files
    let notifications = executor.get_diagnostics(uri_str, query);

    // Send all notifications (may include diagnostics for schema files, etc.)
    for notification in notifications {
        if let Err(e) = connection.sender.send(Message::Notification(notification)) {
            error!("Failed to send diagnostics: {}", e);
        }
    }
}

/// Convert an LSP URI to a TextFile.
fn uri_to_text_file(uri: &Uri) -> TextFile {
    // Extract path from file:// URI, decoding percent-encoding
    let path_str = uri.path().as_str();
    // Simple percent-decoding for common cases
    let decoded = percent_decode(path_str);
    TextFile::from_path(PathBuf::from(decoded))
}

/// Parameters for the `$/cancelRequest` notification.
#[derive(serde::Deserialize)]
struct CancelParams {
    id: RequestId,
}

/// Percent-decode a URI path to a string.
fn percent_decode(s: &str) -> String {
    percent_encoding::percent_decode_str(s)
        .decode_utf8_lossy()
        .into_owned()
}
