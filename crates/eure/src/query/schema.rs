//! Schema conversion and validation queries.

use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use eure_document::value::ObjectKey;
use eure_schema::SchemaDocument;
use eure_schema::convert::{
    ConversionError, SchemaSourceMap, loaded_schema_set_to_schema_with_layout,
};
use eure_schema::parse::{ParsedImports, parse_root_imports};
use eure_schema::resolver::{LoadedSchemaSet, ResolvedSchemaUri, ResolverError};
use eure_schema::type_path_trace::LayoutStrategies;
use eure_schema::validate::{ValidationError, validate};
use eure_tree::prelude::Cst;
use eure_tree::tree::InputSpan;
use indexmap::IndexMap;
use query_flow::{Db, QueryError, query};
use url::Url;

use crate::document::OriginMap;

use crate::report::{
    ErrorReport, ErrorReports, Origin, format_error_reports, report_schema_validation_errors,
};

use super::assets::TextFile;
use super::config::ResolveConfig;
use super::error::FileError;
use super::parse::{ParseCst, ParseDocument, ParsedDocument};

/// Validated schema with the SchemaDocument and source map.
#[derive(Clone, PartialEq)]
pub struct ValidatedSchema {
    pub schema: Arc<SchemaDocument>,
    pub layout: Arc<LayoutStrategies>,
    pub source_map: Arc<SchemaSourceMap>,
    pub parsed: ParsedDocument,
    pub source_files: Arc<IndexMap<ResolvedSchemaUri, TextFile>>,
    pub parsed_by_uri: Arc<IndexMap<ResolvedSchemaUri, ParsedDocument>>,
}

/// Resolved $schema extension value with origin for error reporting.
#[derive(Clone, PartialEq)]
pub struct ResolvedSchemaExtension {
    /// The schema path string from $schema extension.
    pub path: String,
    /// Origin of the $schema value (for span in diagnostics).
    pub origin: Origin,
}

/// Resolved schema reference with origin for error reporting.
#[derive(Clone, PartialEq)]
pub struct ResolvedSchema {
    /// The resolved schema file.
    pub file: TextFile,
    /// Origin of the schema reference (None for heuristics like *.schema.eure).
    pub origin: Option<Origin>,
}

fn schema_base_uri(file: &TextFile) -> ResolvedSchemaUri {
    match file.as_local_path() {
        Some(path) => ResolvedSchemaUri::Local(normalize_path_lexically(path)),
        None => ResolvedSchemaUri::Inline(file.to_string()),
    }
}

fn normalize_path_lexically(path: &Path) -> PathBuf {
    let mut prefix: Option<OsString> = None;
    let mut has_root = false;
    let mut parts: Vec<OsString> = Vec::new();

    for component in path.components() {
        match component {
            Component::Prefix(value) => {
                prefix = Some(value.as_os_str().to_os_string());
                parts.clear();
            }
            Component::RootDir => {
                has_root = true;
                parts.clear();
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if parts.pop().is_none() && !has_root {
                    parts.push(OsString::from(".."));
                }
            }
            Component::Normal(value) => parts.push(value.to_os_string()),
        }
    }

    let mut normalized = PathBuf::new();
    if let Some(prefix) = prefix {
        normalized.push(prefix);
    }
    if has_root {
        normalized.push(std::path::MAIN_SEPARATOR.to_string());
    }
    for part in parts {
        normalized.push(part);
    }
    normalized
}

fn resolve_schema_import_text_file(
    importer: &TextFile,
    raw_path: &str,
    boundary: Option<&Path>,
) -> Result<TextFile, ResolverError> {
    if raw_path.starts_with("https://") {
        let url = Url::parse(raw_path).map_err(|error| ResolverError::InvalidUrl {
            raw_url: raw_path.to_string(),
            reason: error.to_string(),
        })?;
        return Ok(TextFile::from_url(url));
    }

    if let Some((scheme, _)) = raw_path.split_once("://") {
        return Err(ResolverError::UnsupportedScheme {
            scheme: scheme.to_string(),
        });
    }

    let raw = Path::new(raw_path);
    if raw.is_absolute() {
        return Err(ResolverError::AbsolutePathUnsupported {
            raw_path: raw_path.to_string(),
        });
    }

    let Some(base_path) = importer.as_local_path() else {
        return Err(ResolverError::NonLocalBase {
            raw_path: raw_path.to_string(),
            base: ResolvedSchemaUri::Inline(importer.to_string()),
        });
    };

    let base_dir = base_path.parent().unwrap_or(Path::new(""));
    let target = normalize_path_lexically(&base_dir.join(raw));

    if let Some(boundary) = boundary {
        let boundary = normalize_path_lexically(boundary);
        if !boundary.as_os_str().is_empty() && !target.starts_with(&boundary) {
            return Err(ResolverError::EscapesWorkspaceRoot {
                resolved: target,
                workspace_root: boundary,
            });
        }
    }

    Ok(TextFile::from_path(target))
}

struct QueryLoadedSchemaSet {
    loaded: LoadedSchemaSet,
    source_files: IndexMap<ResolvedSchemaUri, TextFile>,
    parsed_by_uri: IndexMap<ResolvedSchemaUri, ParsedDocument>,
    import_boundary: Option<PathBuf>,
}

fn collect_schema_import_graph(
    db: &impl Db,
    root_file: TextFile,
    root_parsed: ParsedDocument,
) -> Result<QueryLoadedSchemaSet, QueryError> {
    let root_uri = schema_base_uri(&root_file);
    let import_boundary = import_boundary(db, &root_file)?;
    let mut graph = QueryLoadedSchemaSet {
        loaded: LoadedSchemaSet::new(root_uri.clone(), root_parsed.doc.as_ref().clone()),
        source_files: IndexMap::new(),
        parsed_by_uri: IndexMap::new(),
        import_boundary,
    };
    let mut stack = Vec::new();
    collect_schema_import_graph_inner(
        db,
        root_uri,
        root_file,
        root_parsed,
        &mut graph,
        &mut stack,
    )?;
    Ok(graph)
}

fn collect_schema_import_graph_inner(
    db: &impl Db,
    uri: ResolvedSchemaUri,
    file: TextFile,
    parsed: ParsedDocument,
    graph: &mut QueryLoadedSchemaSet,
    stack: &mut Vec<ResolvedSchemaUri>,
) -> Result<(), QueryError> {
    if let Some(start) = stack.iter().position(|existing| existing == &uri) {
        return Err(FileError {
            file,
            kind: ConversionError::ImportCycle {
                cycle: stack[start..].to_vec(),
                attempted: uri,
            },
        }
        .into());
    }

    if graph.parsed_by_uri.contains_key(&uri) {
        return Ok(());
    }

    graph
        .loaded
        .insert_document(uri.clone(), parsed.doc.as_ref().clone());
    graph.source_files.insert(uri.clone(), file.clone());
    graph.parsed_by_uri.insert(uri.clone(), parsed.clone());
    stack.push(uri.clone());

    let root_ctx = parsed.doc.parse_context(parsed.doc.get_root_id());
    let imports: ParsedImports = parse_root_imports(&root_ctx).map_err(|kind| FileError {
        file: file.clone(),
        kind: ConversionError::ParseError(kind),
    })?;

    for (alias, entry) in imports.entries {
        let target_file = resolve_schema_import_text_file(
            &file,
            &entry.raw_path,
            graph.import_boundary.as_deref(),
        )
        .map_err(|source| FileError {
            file: file.clone(),
            kind: ConversionError::ImportResolverFailed {
                alias: alias.to_string(),
                raw_path: entry.raw_path.clone(),
                source,
            },
        })?;
        let target_uri = schema_base_uri(&target_file);

        graph
            .loaded
            .insert_import(uri.clone(), alias, target_uri.clone());

        if let Some(start) = stack.iter().position(|existing| existing == &target_uri) {
            return Err(FileError {
                file: file.clone(),
                kind: ConversionError::ImportCycle {
                    cycle: stack[start..].to_vec(),
                    attempted: target_uri,
                },
            }
            .into());
        }

        let target_parsed = db.query(ParseDocument::new(target_file.clone()))?;
        collect_schema_import_graph_inner(
            db,
            target_uri,
            target_file,
            target_parsed.as_ref().clone(),
            graph,
            stack,
        )?;
    }

    stack.pop();
    Ok(())
}

fn import_boundary(db: &impl Db, root_file: &TextFile) -> Result<Option<PathBuf>, QueryError> {
    let Some(root_path) = root_file.as_local_path() else {
        return Ok(None);
    };
    let root_path = normalize_path_lexically(root_path);

    for workspace_id in db.list_asset_keys::<super::assets::WorkspaceId>() {
        let workspace = db.asset(workspace_id)?;
        let workspace_path = normalize_path_lexically(&workspace.path);
        if root_path.starts_with(&workspace_path) {
            return Ok(Some(workspace_path));
        }
    }

    Ok(root_path.parent().and_then(|path| {
        let path = normalize_path_lexically(path);
        (!path.as_os_str().is_empty()).then_some(path)
    }))
}

/// Convert document to SchemaDocument.
///
/// Returns `None` if parsing failed.
/// Returns `UserError(SchemaConversionError)` if schema conversion fails.
/// The `SchemaConversionError` contains the file information for proper error reporting.
#[query(debug = "{Self}({file})")]
pub fn document_to_schema_query(
    db: &impl Db,
    file: TextFile,
) -> Result<ValidatedSchema, QueryError> {
    let parsed = db.query(ParseDocument::new(file.clone()))?;
    let loaded = collect_schema_import_graph(db, file.clone(), parsed.as_ref().clone())?;

    let (schema, layout, source_map) = loaded_schema_set_to_schema_with_layout(&loaded.loaded)
        .map_err(|kind| FileError {
            file: file.clone(),
            kind,
        })?;
    Ok(ValidatedSchema {
        schema: Arc::new(schema),
        layout: Arc::new(layout),
        source_map: Arc::new(source_map),
        parsed: parsed.as_ref().clone(),
        source_files: Arc::new(loaded.source_files),
        parsed_by_uri: Arc::new(loaded.parsed_by_uri),
    })
}

/// Validate document against schema.
///
/// Resolves the schema internally from the document's $schema extension,
/// workspace config, or file name heuristics.
///
/// Returns empty reports if no schema is found or parsing failed.
/// Returns error report with proper origin if schema file is not found.
#[query(debug = "{Self}({doc_file})")]
pub fn validate_against_schema(
    db: &impl Db,
    doc_file: TextFile,
) -> Result<ErrorReports, QueryError> {
    // Resolve schema internally
    let Some(schema) = db
        .query(ResolveSchema::new(doc_file.clone()))?
        .as_ref()
        .clone()
    else {
        return Ok(ErrorReports::new());
    };

    // Parse document
    let doc_result = db.query(ParseDocument::new(doc_file.clone()))?;
    let doc_parsed = doc_result.as_ref().clone();

    // Load and convert schema - handle errors appropriately
    let schema_result = match db.query(DocumentToSchemaQuery::new(schema.file.clone())) {
        Ok(result) => result,
        Err(QueryError::UserError(e)) => {
            // Schema conversion errors are returned as ErrorReports with proper location
            if let Some(reports) = e.downcast_ref::<ErrorReports>() {
                return Ok(reports.clone());
            }
            // Other errors (file not found, network, etc.) should be reported at $schema origin
            if let Some(origin) = &schema.origin {
                return Ok(ErrorReports::from(vec![ErrorReport::error(
                    format!("Failed to load schema: {}", e),
                    origin.clone(),
                )]));
            }
            return Err(QueryError::UserError(e));
        }
        Err(other) => return Err(other),
    };

    let result = validate(&doc_parsed.doc, &schema_result.schema);

    report_schema_validation_errors(db, doc_file, schema.file, &result.errors)
}

/// Validate document against an explicitly provided schema file.
///
/// Use this when you have a specific schema file to validate against
/// (e.g., from workspace config). For automatic schema resolution,
/// use `validate_against_schema` instead.
///
/// Returns empty reports if either document or schema parsing failed.
#[query(debug = "{Self}({doc_file}, {schema_file})")]
pub fn validate_against_explicit_schema(
    db: &impl Db,
    doc_file: TextFile,
    schema_file: TextFile,
) -> Result<ErrorReports, QueryError> {
    let doc_result = db.query(ParseDocument::new(doc_file.clone()))?;
    let doc_parsed = doc_result.as_ref().clone();

    let schema_result = db.query(DocumentToSchemaQuery::new(schema_file.clone()))?;

    let result = validate(&doc_parsed.doc, &schema_result.schema);

    report_schema_validation_errors(db, doc_file, schema_file, &result.errors)
}

/// Validate document against an explicitly provided schema and return formatted error strings.
///
/// Use this when you have a specific schema file to validate against.
///
/// Returns empty vec if either document or schema parsing failed.
/// Returns formatted error messages suitable for display.
#[query(debug = "{Self}({doc_file}, {schema_file})")]
pub fn get_validation_errors_formatted_explicit(
    db: &impl Db,
    doc_file: TextFile,
    schema_file: TextFile,
) -> Result<Vec<String>, QueryError> {
    let reports = db.query(ValidateAgainstExplicitSchema::new(doc_file, schema_file))?;

    // Format each error report individually
    let mut formatted = Vec::new();
    for report in reports.iter() {
        let single_report = ErrorReports::from(vec![report.clone()]);
        formatted.push(format_error_reports(db, &single_report, false)?);
    }

    Ok(formatted)
}

/// Validate document against schema and return formatted error strings.
///
/// Resolves the schema internally from the document's $schema extension,
/// workspace config, or file name heuristics.
///
/// Returns empty vec if no schema is found or parsing failed.
/// Returns formatted error messages suitable for display.
#[query(debug = "{Self}({doc_file})")]
pub fn get_validation_errors_formatted(
    db: &impl Db,
    doc_file: TextFile,
) -> Result<Vec<String>, QueryError> {
    let reports = db.query(ValidateAgainstSchema::new(doc_file))?;

    // Format each error report individually
    let mut formatted = Vec::new();
    for report in reports.iter() {
        let single_report = ErrorReports::from(vec![report.clone()]);
        formatted.push(format_error_reports(db, &single_report, false)?);
    }

    Ok(formatted)
}

// =============================================================================
// Schema Resolution Queries
// =============================================================================

/// Extract the `$schema` extension value from a document's root node.
///
/// Returns `None` if:
/// - The file cannot be parsed
/// - The document has no `$schema` extension
/// - The `$schema` value is not a valid string
///
/// Returns `ResolvedSchemaExtension` with the path and origin for error reporting.
#[query(debug = "{Self}({file})")]
pub fn get_schema_extension(
    db: &impl Db,
    file: TextFile,
) -> Result<Option<ResolvedSchemaExtension>, QueryError> {
    let parsed = match db.query(ParseDocument::new(file.clone())) {
        Ok(parsed) => parsed,
        // The document itself is broken (typically: it is being edited).
        // Its parse error is reported separately; here we still want the
        // schema reference so editor features keep working, so read the
        // `$schema` binding syntactically from the partial CST.
        Err(QueryError::UserError(_)) => return schema_extension_from_cst(db, file),
        Err(error) => return Err(error),
    };

    let root_id = parsed.doc.get_root_id();
    let root_ctx = parsed.doc.parse_context(root_id);

    // Check if $schema extension exists
    let Some(schema_ctx) = root_ctx.ext_optional("schema") else {
        return Ok(None);
    };

    // Try to get $schema extension as a string
    let Ok(Some(schema_path)) = root_ctx.parse_ext_optional::<String>("schema") else {
        return Ok(None); // Invalid type, diagnostics handled by get_schema_extension_diagnostics
    };

    // Get the span for the $schema value
    let node_id = schema_ctx.node_id();
    let cst = db.query(ParseCst::new(file.clone()))?;
    let span = parsed
        .origins
        .get_value_span(node_id, &cst.cst)
        .unwrap_or(InputSpan::EMPTY);

    let origin = Origin::new(file, span);

    Ok(Some(ResolvedSchemaExtension {
        path: schema_path,
        origin,
    }))
}

/// Read a root-level `$schema = "..."` (or `$schema: ...`) binding from the
/// tolerant CST of a document that does not parse.
fn schema_extension_from_cst(
    db: &impl Db,
    file: TextFile,
) -> Result<Option<ResolvedSchemaExtension>, QueryError> {
    use crate::tree::scan;
    use eure_document::path::PathSegment;
    use eure_tree::prelude::NonTerminalKind;
    use eure_tree::tree::CstFacade as _;

    let parsed = db.query(ParseCst::new(file.clone()))?;
    let source = db.asset(file.clone())?;
    let input = source.get();
    let cst = &parsed.cst;

    let Some(eure) = scan::child_of_kind(cst, cst.root(), NonTerminalKind::Eure) else {
        return Ok(None);
    };
    let bindings = scan::list_items(
        cst,
        scan::child_of_kind(cst, eure, NonTerminalKind::EureList),
        NonTerminalKind::Binding,
    );
    for binding in bindings {
        let Some(keys) = scan::child_of_kind(cst, binding, NonTerminalKind::Keys) else {
            continue;
        };
        let keys = scan::parse_keys(input, cst, keys);
        let is_schema_key = matches!(
            keys.segments.as_slice(),
            [only] if matches!(&only.segment, Some(PathSegment::Extension(ext)) if ext.as_ref() == "schema")
        );
        if !is_schema_key {
            continue;
        }
        let Some(rhs) = scan::child_of_kind(cst, binding, NonTerminalKind::BindingRhs) else {
            continue;
        };
        let Some((node, kind)) = scan::child_of_kinds(
            cst,
            rhs,
            &[NonTerminalKind::ValueBinding, NonTerminalKind::TextBinding],
        ) else {
            continue;
        };
        let resolved = match kind {
            NonTerminalKind::ValueBinding => scan::child_of_kind(cst, node, NonTerminalKind::Value)
                .and_then(|value| Some((scan::value_string(input, cst, value)?, cst.span(value)?))),
            _ => scan::text_binding_content(input, cst, node)
                .and_then(|path| Some((path, cst.span(node)?))),
        };
        return Ok(resolved.map(|(path, span)| ResolvedSchemaExtension {
            path,
            origin: Origin::new(file, span),
        }));
    }
    Ok(None)
}

/// Check for schema extension errors (e.g., wrong type).
///
/// Returns diagnostics if `$schema` exists but is not a valid string.
#[query(debug = "{Self}({file})")]
pub fn get_schema_extension_diagnostics(
    db: &impl Db,
    file: TextFile,
) -> Result<ErrorReports, QueryError> {
    let result = db.query(ParseDocument::new(file.clone()))?;
    let parsed = result.as_ref().clone();

    let root_id = parsed.doc.get_root_id();
    let root_ctx = parsed.doc.parse_context(root_id);

    // Check if $schema extension exists
    let Some(schema_ctx) = root_ctx.ext_optional("schema") else {
        return Ok(ErrorReports::new());
    };

    // Try to parse as string
    if root_ctx.parse_ext_optional::<String>("schema").is_ok() {
        return Ok(ErrorReports::new());
    }

    // $schema exists but has wrong type - generate diagnostic
    let node_id = schema_ctx.node_id();
    let cst = db.query(ParseCst::new(file.clone()))?;
    let span = parsed.origins.get_value_span(node_id, &cst.cst);

    // FIXME: Fallback span (0, 1) points to file start instead of the actual $schema value.
    // The is_fallback flag is set, but the span itself is misleading.
    // Should find the actual span of the $schema extension key or value.
    let origin = crate::report::Origin {
        file,
        span: span.unwrap_or(eure_tree::tree::InputSpan { start: 0, end: 1 }),
        hints: Default::default(),
        is_fallback: span.is_none(),
    };

    Ok(ErrorReports::from(vec![ErrorReport::error(
        "$schema must be a string path to a schema file",
        origin,
    )]))
}

/// Resolve the schema file for a document.
///
/// Priority order:
/// 1. `$schema` extension in the document itself
/// 2. Workspace config (`Eure.eure`) schema mappings
/// 3. File name heuristics (e.g., `*.schema.eure` uses meta-schema)
///
/// Returns `None` if no schema can be determined.
/// Returns `ResolvedSchema` with the file and origin for error reporting.
#[query(debug = "{Self}({file})")]
pub fn resolve_schema(db: &impl Db, file: TextFile) -> Result<Option<ResolvedSchema>, QueryError> {
    // 1. Check $schema extension in the document
    if let Some(ext) = db.query(GetSchemaExtension::new(file.clone()))?.as_ref() {
        // Resolve relative to the document's directory (only for local files)
        if let Some(base_path) = file.as_local_path() {
            let base_dir = base_path.parent().unwrap_or(Path::new("."));
            return Ok(Some(ResolvedSchema {
                file: TextFile::resolve(&ext.path, base_dir)?,
                origin: Some(ext.origin.clone()),
            }));
        }
        // For remote files, only absolute URLs are supported
        if ext.path.starts_with("https://") {
            return Ok(Some(ResolvedSchema {
                file: TextFile::parse(&ext.path)?,
                origin: Some(ext.origin.clone()),
            }));
        }
    }

    // 2. Check workspace config (only for local files)
    if let Some(file_path) = file.as_local_path()
        && let Some(resolved) = db.query(ResolveConfig::new(file.clone()))?.as_ref()
        && let Some(schema_path) = resolved
            .config
            .schema_for_path(file_path, &resolved.config_dir)
    {
        return Ok(Some(ResolvedSchema {
            file: TextFile::resolve(&schema_path, &resolved.config_dir)?,
            origin: None, // Config-based resolution has no specific origin
        }));
    }

    // 3. File name heuristics (works for both local and remote)
    if file.ends_with(".schema.eure") {
        // Schema files are validated against the meta-schema
        return Ok(Some(ResolvedSchema {
            file: meta_schema_file(),
            origin: None, // Heuristic-based resolution has no specific origin
        }));
    }

    Ok(None)
}

/// Get the built-in meta-schema file.
fn meta_schema_file() -> TextFile {
    const LOCAL_META_SCHEMA: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/schemas/eure-schema.schema.eure"
    );

    if Path::new(LOCAL_META_SCHEMA).exists() {
        return TextFile::from_path(PathBuf::from(LOCAL_META_SCHEMA));
    }

    // The meta-schema is bundled with the application
    TextFile::parse(concat!(
        "https://eure.dev/v",
        env!("CARGO_PKG_VERSION"),
        "/schemas/eure-schema.schema.eure"
    ))
    .expect("hardcoded meta-schema URL is valid")
}

// =============================================================================
// Validation Error Span Resolution
// =============================================================================

/// Resolve the document span for a validation error.
///
/// Handles error-specific span resolution:
/// - `UnknownField`: Use key span for the unknown field name
/// - `MissingRequiredField`: Use key span if the field exists elsewhere, otherwise node span
/// - `InvalidKeyType`: Use key span for the invalid key
/// - Others: Use node span
pub fn resolve_validation_error_span(
    error: &ValidationError,
    origins: &OriginMap,
    cst: &Cst,
) -> Option<InputSpan> {
    let (node_id, _schema_node_id) = error.node_ids();

    match error {
        // For UnknownField, try to get the precise key span
        ValidationError::UnknownField { field, node_id, .. } => {
            let key = ObjectKey::String(field.clone());
            origins
                .get_key_span(*node_id, &key, cst)
                .or_else(|| origins.get_value_span(*node_id, cst))
        }

        // For InvalidKeyType, use the key span
        ValidationError::InvalidKeyType { key, node_id, .. } => origins
            .get_key_span(*node_id, key, cst)
            .or_else(|| origins.get_value_span(*node_id, cst)),

        // For MissingRequiredField, the node_id is the parent map
        // We can't point to the missing field, so use the parent span
        ValidationError::MissingRequiredField { .. } => origins.get_value_span(node_id, cst),

        // For all other errors, use the standard node span
        _ => origins.get_value_span(node_id, cst),
    }
}
