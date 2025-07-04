// Comment out the module since it's empty
// mod semantic_tokens;

use std::collections::HashMap;
use std::path::PathBuf;

use eure_editor_support::{diagnostics, parser, schema_validation, semantic_tokens};
use eure_tree::Cst;
use lsp_types::notification::{Notification as _, PublishDiagnostics};
use lsp_types::request::{DocumentDiagnosticRequest, SemanticTokensFullRequest};
use lsp_types::{
    Diagnostic, DocumentDiagnosticParams, DocumentDiagnosticReport,
    DocumentDiagnosticReportResult, FullDocumentDiagnosticReport, InitializeParams,
    PublishDiagnosticsParams, RelatedFullDocumentDiagnosticReport, SemanticTokensFullOptions,
    SemanticTokensLegend, SemanticTokensOptions, SemanticTokensResult, ServerCapabilities, Uri,
};

use lsp_server::{
    Connection, ErrorCode, ExtractError, Message, Notification, Request, Response, ResponseError,
};

fn main() -> anyhow::Result<()> {
    let (connection, io_threads) = Connection::stdio();

    // Get the legend from the support crate
    let legend = semantic_tokens::get_legend();

    let server_capabilities = serde_json::to_value(&ServerCapabilities {
        semantic_tokens_provider: Some(
            SemanticTokensOptions {
                work_done_progress_options: Default::default(),
                legend: legend.clone(), // Use the actual legend
                range: Some(false),     // Let's start with full document support only
                full: Some(SemanticTokensFullOptions::Delta { delta: Some(false) }), // No delta support yet
            }
            .into(),
        ),
        // Add textDocumentSync capability if not already present, needed for tracking documents
        text_document_sync: Some(lsp_types::TextDocumentSyncCapability::Kind(
            lsp_types::TextDocumentSyncKind::FULL, // Or INCREMENTAL if handled
        )),
        // Include diagnostic capability
        diagnostic_provider: Some(lsp_types::DiagnosticServerCapabilities::Options(
            lsp_types::DiagnosticOptions {
                identifier: None,
                workspace_diagnostics: false,
                work_done_progress_options: Default::default(),
                inter_file_dependencies: false,
            },
        )),
        ..Default::default()
    })
    .unwrap();
    let params = match connection.initialize(server_capabilities) {
        Ok(it) => it,
        Err(e) => {
            if e.channel_is_disconnected() {
                io_threads.join()?;
            }
            return Err(e.into());
        }
    };
    let params: InitializeParams = serde_json::from_value(params)?;

    let mut context = ServerContext {
        connection,
        params,
        documents: HashMap::new(), // Initialize documents map
        legend,                    // Store legend in context
        schema_manager: schema_validation::SchemaManager::new(),
        diagnostics: HashMap::new(), // Initialize diagnostics map
    };
    context.run()?;

    io_threads.join()?;

    // Shut down gracefully.
    eprintln!("shutting down server");
    Ok(())
}

pub struct ServerContext {
    connection: Connection,
    #[allow(dead_code)]
    params: InitializeParams,
    documents: HashMap<String, (Option<Cst>, String)>, // Store (CST, Content) by document URI
    legend: SemanticTokensLegend,                      // Store the legend
    schema_manager: schema_validation::SchemaManager,  // Schema management
    diagnostics: HashMap<String, Vec<Diagnostic>>,     // Store diagnostics by document URI
}

pub enum Event {
    Request(Request),
    Response(Response),
    Notification(Notification),
}

impl ServerContext {
    fn run(&mut self) -> anyhow::Result<()> {
        loop {
            let msg = self.connection.receiver.recv()?;
            match msg {
                Message::Request(req) => {
                    if self.connection.handle_shutdown(&req)? {
                        return Ok(());
                    }
                    // Handle Semantic Tokens request
                    if self
                        .handle_request::<SemanticTokensFullRequest>(
                            req.clone(), // Clone req as handle_request consumes it
                            Self::handle_semantic_tokens_full,
                        )?
                        .is_some()
                    {
                        continue; // Request was handled
                    }

                    // Handle Document Diagnostic request
                    if self
                        .handle_request::<DocumentDiagnosticRequest>(
                            req.clone(),
                            Self::handle_document_diagnostic,
                        )?
                        .is_some()
                    {
                        continue; // Request was handled
                    }

                    // Placeholder for other request handlers
                    eprintln!("unhandled request: {req:?}");
                    let resp = Response {
                        id: req.id,
                        result: None,
                        error: Some(ResponseError {
                            code: ErrorCode::MethodNotFound as i32,
                            message: format!("method not supported: {}", req.method),
                            data: None,
                        }),
                    };
                    self.send_response(resp)?;
                }
                Message::Response(_resp) => {
                    // Handle response if needed
                }
                Message::Notification(not) => {
                    // Handle notification for document updates
                    if not.method == "textDocument/didOpen" {
                        if let Ok(params) = serde_json::from_value::<
                            lsp_types::DidOpenTextDocumentParams,
                        >(not.params)
                        {
                            let uri = params.text_document.uri.clone();
                            let text = params.text_document.text;
                            let version = params.text_document.version;

                            self.process_document(uri, text, Some(version))?;
                        }
                    } else if not.method == "textDocument/didChange"
                        && let Ok(params) = serde_json::from_value::<
                            lsp_types::DidChangeTextDocumentParams,
                        >(not.params)
                    {
                        let uri = params.text_document.uri.clone();
                        let version = params.text_document.version;

                        // For FULL sync, we just get the full content from the last change
                        if let Some(last_change) = params.content_changes.last() {
                            let text = last_change.text.clone();
                            self.process_document(uri, text, Some(version))?;
                        }
                    }
                }
            }
        }
    }

    // Process a document: parse it, store it, and publish diagnostics
    fn process_document(
        &mut self,
        uri: Uri,
        text: String,
        version: Option<i32>,
    ) -> anyhow::Result<()> {
        // Try to parse the document using eure-editor-support
        let uri_string = uri.to_string();
        let parse_result = parser::parse_document(&text);

        // Prepare diagnostics and store CST based on parse result
        let (cst, mut diagnostics) = match parse_result {
            parser::ParseResult::Ok(cst) => {
                // Success case - store CST and clear diagnostics
                (Some(cst), Vec::new())
            }
            parser::ParseResult::ErrWithCst { cst, error } => {
                // We have both a partial CST and an error
                (Some(cst), diagnostics::error_to_diagnostic(&error))
            }
        };

        // Store document in our map
        let cst_clone = cst.clone();
        self.documents.insert(uri_string.clone(), (cst, text.clone()));

        // If we have a valid CST, perform schema validation
        if let Some(ref cst) = cst_clone {
            // Check if this is a schema file itself
            if uri_string.contains(".schema.eure") {
                // Try to load this as a schema
                if let Err(e) = self.schema_manager.load_schema(&uri_string, &text, cst) {
                    eprintln!("Failed to load schema from {uri_string}: {e}");
                }
            } else {
                // First, extract schema and check for $schema reference
                if let Ok(validation_result) = schema_validation::validate_and_extract_schema(&text, cst) {
                    // Check if document has a $schema reference
                    if let Some(schema_ref) = &validation_result.schema.document_schema.schema_ref {
                    if let Some(doc_path) = uri_to_path(&uri) {
                        let workspace_root = self.get_workspace_root();
                        match schema_validation::resolve_schema_reference(&doc_path, schema_ref, workspace_root.as_deref()) {
                            Ok(schema_path) => {
                                // Load the schema if we haven't already
                                let schema_uri = format!("file://{}", schema_path.display());
                                if self.schema_manager.get_schema(&schema_uri).is_none() {
                                    // Need to parse and load the schema
                                    if let Ok(schema_content) = std::fs::read_to_string(&schema_path)
                                        && let parser::ParseResult::Ok(schema_cst) = parser::parse_document(&schema_content) {
                                            if let Err(e) = self.schema_manager.load_schema(&schema_uri, &schema_content, &schema_cst) {
                                                eprintln!("Failed to load schema from {schema_uri}: {e}");
                                                eprintln!("Schema path: {}", schema_path.display());
                                            } else {
                                                eprintln!("Successfully loaded schema from {schema_uri}");
                                                // Associate document with schema
                                                self.schema_manager.set_document_schema(&uri_string, &schema_uri);
                                                eprintln!("Associated {uri_string} with schema {schema_uri}");
                                            }
                                        }
                                } else {
                                    // Schema already loaded, just associate
                                    self.schema_manager.set_document_schema(&uri_string, &schema_uri);
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to resolve schema reference '{schema_ref}': {e}");
                            }
                        }
                    }
                    }
                } else {
                    // No $schema reference, fall back to convention-based discovery
                    if let Some(path) = uri_to_path(&uri) {
                        let workspace_root = self.get_workspace_root();
                        if let Some(schema_path) = schema_validation::find_schema_for_document(&path, workspace_root.as_deref()) {
                            eprintln!("Found schema by convention for {}: {}", uri_string, schema_path.display());
                            // Load the schema if we haven't already
                            let schema_uri = format!("file://{}", schema_path.display());
                            if self.schema_manager.get_schema(&schema_uri).is_none() {
                                // Need to parse and load the schema
                                if let Ok(schema_content) = std::fs::read_to_string(&schema_path) {
                                    if let parser::ParseResult::Ok(schema_cst) = parser::parse_document(&schema_content) {
                                        if let Err(e) = self.schema_manager.load_schema(&schema_uri, &schema_content, &schema_cst) {
                                            eprintln!("Failed to load schema from {schema_uri}: {e}");
                                        } else {
                                            eprintln!("Successfully loaded schema from {schema_uri}");
                                            // Associate document with schema
                                            self.schema_manager.set_document_schema(&uri_string, &schema_uri);
                                            eprintln!("Associated {uri_string} with schema {schema_uri}");
                                        }
                                    } else {
                                        eprintln!("Failed to parse schema file: {}", schema_path.display());
                                    }
                                } else {
                                    eprintln!("Failed to read schema file: {}", schema_path.display());
                                }
                            } else {
                                eprintln!("Schema already loaded, associating {uri_string} with {schema_uri}");
                                // Schema already loaded, just associate
                                self.schema_manager.set_document_schema(&uri_string, &schema_uri);
                            }
                        } else {
                            eprintln!("No schema found by convention for {uri_string}");
                        }
                    }
                }
                
                // Run schema validation
                let schema_diagnostics = schema_validation::validate_document(
                    &uri_string,
                    &text,
                    cst,
                    &self.schema_manager,
                );
                
                // Merge diagnostics
                diagnostics.extend(schema_diagnostics);
            }
        }

        // Publish diagnostics
        self.publish_diagnostics(uri, diagnostics, version)?;

        Ok(())
    }

    // Publish diagnostics to the client
    fn publish_diagnostics(
        &mut self,
        uri: Uri,
        diagnostics: Vec<Diagnostic>,
        version: Option<i32>,
    ) -> anyhow::Result<()> {
        // Store diagnostics for pull-based requests
        self.diagnostics.insert(uri.to_string(), diagnostics.clone());
        
        let params = PublishDiagnosticsParams {
            uri,
            diagnostics,
            version,
        };

        let notification = Notification {
            method: PublishDiagnostics::METHOD.to_string(),
            params: serde_json::to_value(params)?,
        };

        self.connection
            .sender
            .send(Message::Notification(notification))?;
        Ok(())
    }

    // Handler for textDocument/semanticTokens/full
    fn handle_semantic_tokens_full(
        &mut self,
        params: lsp_types::SemanticTokensParams,
    ) -> anyhow::Result<Option<Option<SemanticTokensResult>>> {
        let uri = params.text_document.uri.to_string();

        // Lookup document in our store
        if let Some((cst_opt, text)) = self.documents.get(&uri) {
            if let Some(cst) = cst_opt {
                // Generate tokens if we have a CST
                match semantic_tokens::semantic_tokens(text, cst, &self.legend) {
                    Some(tokens) => Ok(Some(Some(SemanticTokensResult::Tokens(tokens)))),
                    None => Ok(Some(None)),
                }
            } else {
                eprintln!("Document has no valid CST for {uri}");
                Ok(Some(None))
            }
        } else {
            eprintln!("Document not found in store: {uri}");
            Ok(Some(None))
        }
    }

    // Handler for textDocument/diagnostic
    fn handle_document_diagnostic(
        &mut self,
        params: DocumentDiagnosticParams,
    ) -> anyhow::Result<Option<DocumentDiagnosticReportResult>> {
        let uri = params.text_document.uri.to_string();
        
        // Get stored diagnostics for this document
        let diagnostics = self.diagnostics
            .get(&uri)
            .cloned()
            .unwrap_or_default();
        
        // Create a full diagnostic report
        let report = FullDocumentDiagnosticReport {
            items: diagnostics,
            result_id: None, // We don't support result IDs yet
        };
        
        // Wrap in the required response types
        let result = DocumentDiagnosticReportResult::Report(
            DocumentDiagnosticReport::Full(
                RelatedFullDocumentDiagnosticReport {
                    related_documents: None, // We don't track related documents yet
                    full_document_diagnostic_report: report,
                }
            )
        );
        
        Ok(Some(result))
    }

    fn send_response(&self, resp: Response) -> anyhow::Result<()> {
        Ok(self.connection.sender.send(Message::Response(resp))?)
    }

    // Generic request handler
    fn handle_request<R>(
        &mut self,
        req: Request,
        f: impl FnOnce(&mut Self, R::Params) -> anyhow::Result<Option<R::Result>>,
    ) -> anyhow::Result<Option<()>>
    // Returns Some(()) if handled, None otherwise
    where
        R: lsp_types::request::Request,
        R::Params: serde::de::DeserializeOwned,
        R::Result: serde::ser::Serialize,
    {
        let (id, params) = match req.extract(R::METHOD) {
            Ok(it) => it,
            Err(ExtractError::MethodMismatch(_)) => return Ok(None), // Not this request type
            Err(ExtractError::JsonError { method, error }) => {
                return Err(anyhow::anyhow!(
                    "failed to parse request: {method}: {error}"
                ));
            }
        };

        let result_opt = f(self, params);

        let resp = match result_opt {
            Ok(Some(result)) => Response {
                id,
                result: Some(serde_json::to_value(&result)?),
                error: None,
            },
            Ok(None) => Response {
                id,
                result: Some(serde_json::Value::Null),
                error: None,
            },
            Err(e) => Response {
                id,
                result: None,
                error: Some(ResponseError {
                    code: ErrorCode::InternalError as i32,
                    message: e.to_string(),
                    data: None,
                }),
            },
        };
        self.send_response(resp)?;
        Ok(Some(())) // Signal that the request was handled
    }

    // Get the workspace root from initialization params
    fn get_workspace_root(&self) -> Option<PathBuf> {
        self.params
            .workspace_folders
            .as_ref()
            .and_then(|folders| folders.first())
            .and_then(|folder| uri_to_path(&folder.uri))
            .or_else(|| {
                #[allow(deprecated)]
                self.params
                    .root_uri
                    .as_ref()
                    .and_then(uri_to_path)
            })
    }
}

/// Convert a URI to a file path
fn uri_to_path(uri: &Uri) -> Option<PathBuf> {
    // Check if it's a file:// URI
    let uri_str = uri.as_str();
    if !uri_str.starts_with("file://") {
        return None;
    }
    
    // Remove the file:// prefix and decode the path
    let path_str = &uri_str[7..]; // Skip "file://"
    
    // On Windows, file URIs might have an extra slash (file:///C:/...)
    let path_str = if cfg!(windows) && path_str.starts_with('/') {
        &path_str[1..]
    } else {
        path_str
    };
    
    // Decode percent-encoded characters
    let decoded = percent_decode(path_str).ok()?;
    
    Some(PathBuf::from(decoded))
}

/// Simple percent decoding for file paths
fn percent_decode(s: &str) -> Result<String, ()> {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    
    while let Some(ch) = chars.next() {
        if ch == '%' {
            let hex1 = chars.next().ok_or(())?;
            let hex2 = chars.next().ok_or(())?;
            let byte = u8::from_str_radix(&format!("{hex1}{hex2}"), 16).map_err(|_| ())?;
            result.push(byte as char);
        } else {
            result.push(ch);
        }
    }
    
    Ok(result)
}
