use crate::{
    Route,
    components::editor::{Editor, ErrorSpan},
    theme::Theme,
};
use dioxus::prelude::*;
use eure::document::{OriginMap, cst_to_document, cst_to_document_and_origin_map};
use eure::error::format_parse_error_plain;
use eure::report::{DocumentReportContext, FileRegistry, format_error_report, report_conversion_error};
use eure::tree::Cst;
use eure_editor_support::semantic_token::{SemanticToken, semantic_tokens};
use eure_json::{Config as JsonConfig, document_to_value};
use eure_parol::{EureParseError, ParseResult, parse_tolerant};
use eure_schema::SchemaDocument;
use eure_schema::convert::document_to_schema;
use eure_schema::validate::{ValidationError, validate};

/// Parsed result containing tokens, errors, and JSON output
#[derive(Debug, Clone, Default, PartialEq)]
struct ParsedData {
    tokens: Vec<SemanticToken>,
    errors: Vec<ErrorSpan>,
    json_output: String,
}

/// Parsed schema result with validation against meta-schema
#[derive(Debug, Clone, Default, PartialEq)]
struct ParsedSchemaData {
    tokens: Vec<SemanticToken>,
    parser_errors: Vec<ErrorSpan>,
    schema_errors: Vec<ErrorSpan>,             // Schema conversion errors (with spans)
    schema_validation_errors: Vec<ErrorSpan>,  // Meta-schema validation errors (with spans)
    schema_valid: bool,                        // Whether schema is valid (for document validation)
}

/// All errors organized by category for display
#[derive(Debug, Clone, Default, PartialEq)]
struct AllErrors {
    doc_parser_errors: Vec<ErrorSpan>,
    schema_parser_errors: Vec<ErrorSpan>,
    schema_errors: Vec<ErrorSpan>,            // Schema conversion errors (with spans)
    schema_validation_errors: Vec<ErrorSpan>, // Meta-schema validation errors
    validation_errors: Vec<ErrorSpan>,
}

impl AllErrors {
    fn total_count(&self) -> usize {
        self.doc_parser_errors.len()
            + self.schema_parser_errors.len()
            + self.schema_errors.len()
            + self.schema_validation_errors.len()
            + self.validation_errors.len()
    }

    fn is_empty(&self) -> bool {
        self.total_count() == 0
    }
}

/// Convert a validation error to an ErrorSpan using the origin map
fn validation_error_to_span(
    error: &ValidationError,
    cst: &Cst,
    origins: &OriginMap,
) -> ErrorSpan {
    let message = error.to_string();
    let (node_id, _schema_node_id) = error.node_ids();

    // Try to get span from node_id via origin map
    let span = origins.get_node_span(node_id, cst);

    match span {
        Some(s) => ErrorSpan {
            start: s.start,
            end: s.end,
            message,
        },
        None => ErrorSpan {
            start: 0,
            end: 1,
            message,
        },
    }
}

/// Load and cache the meta-schema for validating schemas
fn load_meta_schema() -> Option<SchemaDocument> {
    static META_SCHEMA_TEXT: &str =
        include_str!("../../../../assets/schemas/eure-schema.schema.eure");

    parse_to_schema(META_SCHEMA_TEXT)
}

// ============================================================================
// Parsing Helper Functions
// ============================================================================

/// Parse Eure input and return CST with optional parse error
fn parse_eure(input: &str) -> (Cst, Option<EureParseError>) {
    match parse_tolerant(input) {
        ParseResult::Ok(cst) => (cst, None),
        ParseResult::ErrWithCst { cst, error } => (cst, Some(error)),
    }
}

/// Convert parse errors to ErrorSpan list
fn format_parser_errors(
    error: Option<EureParseError>,
    input: &str,
    filename: &str,
) -> Vec<ErrorSpan> {
    error
        .map(|e| {
            let message = format_parse_error_plain(&e, input, filename);
            e.entries
                .into_iter()
                .filter_map(|entry| {
                    entry.span.map(|s| ErrorSpan {
                        start: s.start,
                        end: s.end,
                        message: message.clone(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Parse input and convert to SchemaDocument
fn parse_to_schema(input: &str) -> Option<SchemaDocument> {
    let (cst, error) = parse_eure(input);
    if error.is_some() {
        return None;
    }
    let doc = cst_to_document(input, &cst).ok()?;
    let (schema, _) = document_to_schema(&doc).ok()?;
    Some(schema)
}

/// Parse document and return tokens, errors, and JSON output
fn parse_document(input: &str) -> ParsedData {
    let (cst, error) = parse_eure(input);
    let tokens = semantic_tokens(input, &cst);
    let parser_errors = format_parser_errors(error, input, "document.eure");

    let json_output = if parser_errors.is_empty() {
        cst_to_document(input, &cst)
            .ok()
            .and_then(|doc| document_to_value(&doc, &JsonConfig::default()).ok())
            .and_then(|v| serde_json::to_string_pretty(&v).ok())
            .unwrap_or_default()
    } else {
        String::new()
    };

    ParsedData {
        tokens,
        errors: parser_errors,
        json_output,
    }
}

/// Parse schema and validate against meta-schema
fn parse_schema(input: &str) -> ParsedSchemaData {
    let (cst, error) = parse_eure(input);
    let tokens = semantic_tokens(input, &cst);
    let parser_errors = format_parser_errors(error, input, "schema.eure");

    if !parser_errors.is_empty() {
        return ParsedSchemaData {
            tokens,
            parser_errors,
            schema_errors: Vec::new(),
            schema_validation_errors: Vec::new(),
            schema_valid: false,
        };
    }

    let mut schema_errors = Vec::new();
    let mut schema_validation_errors = Vec::new();

    let schema_valid = match cst_to_document_and_origin_map(input, &cst) {
        Ok((doc, origins)) => {
            // Always validate against meta-schema if parse succeeded
            if let Some(meta_schema) = load_meta_schema() {
                let validation_result = validate(&doc, &meta_schema);
                if !validation_result.is_valid {
                    for error in &validation_result.errors {
                        schema_validation_errors
                            .push(validation_error_to_span(error, &cst, &origins));
                    }
                }
            }

            // Check if schema can be used for document validation
            // (document_to_schema must succeed)
            if schema_validation_errors.is_empty() {
                match document_to_schema(&doc) {
                    Ok(_) => true,
                    Err(e) => {
                        // Format conversion error with source spans
                        let mut files = FileRegistry::new();
                        let file_id = files.register("schema.eure", input);
                        let ctx = DocumentReportContext {
                            file: file_id,
                            cst: &cst,
                            origins: &origins,
                        };
                        let report = report_conversion_error(&e, &ctx);
                        let span = report.primary_origin.span;
                        let message = format_error_report(&report, &files, false);
                        schema_errors.push(ErrorSpan {
                            start: span.start,
                            end: span.end,
                            message,
                        });
                        false
                    }
                }
            } else {
                false
            }
        }
        Err(e) => {
            // Document construction errors don't have spans
            schema_errors.push(ErrorSpan {
                start: 0,
                end: 1,
                message: format!("Document construction: {}", e),
            });
            false
        }
    };

    ParsedSchemaData {
        tokens,
        parser_errors,
        schema_errors,
        schema_validation_errors,
        schema_valid,
    }
}

/// Validate document against schema and return validation errors
fn compute_validation_errors(doc_input: &str, schema_input: &str) -> Vec<ErrorSpan> {
    let (doc_cst, doc_error) = parse_eure(doc_input);
    if doc_error.is_some() {
        return Vec::new();
    }

    let schema = match parse_to_schema(schema_input) {
        Some(s) => s,
        None => return Vec::new(),
    };

    match cst_to_document_and_origin_map(doc_input, &doc_cst) {
        Ok((doc, origins)) => {
            let validation_result = validate(&doc, &schema);
            validation_result
                .errors
                .iter()
                .map(|e| validation_error_to_span(e, &doc_cst, &origins))
                .collect()
        }
        Err(_) => Vec::new(),
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
enum RightTab {
    #[default]
    JsonOutput,
    Schema,
    Errors,
}

impl RightTab {
    fn value(&self) -> &'static str {
        match self {
            RightTab::JsonOutput => "json",
            RightTab::Schema => "schema",
            RightTab::Errors => "errors",
        }
    }

    fn from_value(value: &str) -> Option<Self> {
        match value {
            "json" => Some(RightTab::JsonOutput),
            "schema" => Some(RightTab::Schema),
            "errors" => Some(RightTab::Errors),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
enum EureExample {
    #[default]
    Readme,
    HelloWorld,
    EureSchema,
    Cargo,
    GitHubAction,
    GameScript,
    Minimal,
}

impl EureExample {
    const ALL: &'static [EureExample] = &[
        EureExample::Readme,
        EureExample::HelloWorld,
        EureExample::EureSchema,
        EureExample::Cargo,
        EureExample::GitHubAction,
        EureExample::GameScript,
        EureExample::Minimal,
    ];

    fn name(&self) -> &'static str {
        match self {
            EureExample::Readme => "Readme",
            EureExample::HelloWorld => "Hello World",
            EureExample::EureSchema => "Eure Schema",
            EureExample::Cargo => "Cargo",
            EureExample::GitHubAction => "GitHub Action",
            EureExample::GameScript => "Game Script",
            EureExample::Minimal => "Minimal",
        }
    }

    fn value(&self) -> &'static str {
        match self {
            EureExample::Readme => "readme",
            EureExample::HelloWorld => "hello-world",
            EureExample::EureSchema => "eure-schema",
            EureExample::Cargo => "cargo",
            EureExample::GitHubAction => "github-action",
            EureExample::GameScript => "game-script",
            EureExample::Minimal => "minimal",
        }
    }

    fn from_value(value: &str) -> Option<Self> {
        match value {
            "readme" => Some(EureExample::Readme),
            "hello-world" => Some(EureExample::HelloWorld),
            "eure-schema" => Some(EureExample::EureSchema),
            "cargo" => Some(EureExample::Cargo),
            "github-action" => Some(EureExample::GitHubAction),
            "game-script" => Some(EureExample::GameScript),
            "minimal" => Some(EureExample::Minimal),
            _ => None,
        }
    }

    fn content(&self) -> &'static str {
        match self {
            EureExample::Readme => include_str!("../../assets/readme.eure"),
            EureExample::HelloWorld => include_str!("../../assets/examples/hello-world.eure"),
            EureExample::EureSchema => {
                include_str!("../../../../assets/schemas/eure-schema.schema.eure")
            }
            EureExample::Cargo => include_str!("../../assets/examples/cargo.eure"),
            EureExample::GitHubAction => include_str!("../../assets/examples/github-action.eure"),
            EureExample::GameScript => {
                include_str!("../../../../assets/examples/game-script.eure")
            }
            EureExample::Minimal => "= 1\n",
        }
    }

    fn schema(&self) -> &'static str {
        match self {
            EureExample::Readme => include_str!("../../assets/readme.schema.eure"),
            EureExample::HelloWorld => {
                include_str!("../../assets/examples/hello-world.schema.eure")
            }
            EureExample::EureSchema => {
                include_str!("../../../../assets/schemas/eure-schema.schema.eure")
            }
            EureExample::Cargo => include_str!("../../assets/examples/cargo.schema.eure"),
            EureExample::GitHubAction => {
                include_str!("../../assets/examples/github-action.schema.eure")
            }
            EureExample::GameScript => {
                include_str!("../../../../assets/examples/game-script.schema.eure")
            }
            EureExample::Minimal => "= `any`\n",
        }
    }
}

/// Home page with the Eure editor
#[component]
pub fn Home(example: ReadSignal<Option<String>>, tab: ReadSignal<Option<String>>) -> Element {
    let theme: Signal<Theme> = use_context();
    let navigator = use_navigator();

    // Derive the current example from the route parameter
    let current_example = use_memo(move || {
        example()
            .as_deref()
            .and_then(EureExample::from_value)
            .unwrap_or_default()
    });

    // Derive the current tab from the route parameter
    let active_tab = use_memo(move || {
        tab()
            .as_deref()
            .and_then(RightTab::from_value)
            .unwrap_or_default()
    });

    let mut content = use_signal(|| EureExample::default().content().to_string());
    let mut schema_content = use_signal(|| EureExample::default().schema().to_string());

    // Update content when example changes via route
    use_effect(move || {
        let ex = current_example();
        content.set(ex.content().to_string());
        schema_content.set(ex.schema().to_string());
    });

    let parsed = use_memo(move || parse_document(&content()));
    // Create read signals for the editor
    let tokens = use_memo(move || parsed().tokens);
    let doc_parser_errors = use_memo(move || parsed().errors);
    let json_output = use_memo(move || parsed().json_output);
    let schema_parsed = use_memo(move || parse_schema(&schema_content()));
    let schema_tokens = use_memo(move || schema_parsed().tokens);

    // Combined schema errors for the schema editor (parser + meta-schema validation + conversion)
    let combined_schema_errors = use_memo(move || {
        let data = schema_parsed();
        let mut combined = data.parser_errors.clone();
        combined.extend(data.schema_validation_errors.clone());
        combined.extend(data.schema_errors.clone());
        combined
    });

    // Combined validation: validate document against schema
    let all_errors = use_memo(move || {
        let doc_errors = doc_parser_errors();
        let schema_data = schema_parsed();

        let validation_errors = if doc_errors.is_empty() && schema_data.schema_valid {
            compute_validation_errors(&content(), &schema_content())
        } else {
            Vec::new()
        };

        AllErrors {
            doc_parser_errors: doc_errors.clone(),
            schema_parser_errors: schema_data.parser_errors.clone(),
            schema_errors: schema_data.schema_errors.clone(),
            schema_validation_errors: schema_data.schema_validation_errors.clone(),
            validation_errors,
        }
    });

    // Combined errors for the document editor (parser + validation)
    let combined_doc_errors = use_memo(move || {
        let errors = all_errors();
        let mut combined = errors.doc_parser_errors.clone();
        combined.extend(errors.validation_errors.clone());
        combined
    });

    let error_count = use_memo(move || all_errors().total_count());

    let theme_val = theme();
    let bg_color = theme_val.bg_color();
    let border_color = theme_val.border_color();
    let surface1_color = theme_val.surface1_color();
    let accent_color = theme_val.accent_color();
    let error_color = theme_val.error_color();

    rsx! {
        div { class: "h-full px-4 pb-4 flex gap-4",

            // Left column: Eure Editor
            div {
                class: "w-1/2 flex flex-col rounded border min-h-0",
                style: "border-color: {border_color}; background-color: {bg_color}",

                // Section header
                div {
                    class: "h-14 px-3 border-b text-base font-semibold shrink-0 flex justify-between items-center",
                    style: "border-color: {border_color}; background-color: {surface1_color}",
                    span { "Eure" }
                    select {
                        class: "px-4 py-2 rounded-lg border-2 text-base font-semibold cursor-pointer shadow-sm",
                        style: "border-color: {accent_color}; background-color: {bg_color}; color: {accent_color}",
                        value: "{current_example().value()}",
                        onchange: move |evt| {
                            let value = evt.value();
                            navigator
                                .push(Route::Home {
                                    example: Some(value),
                                    tab: tab(),
                                });
                        },
                        for ex in EureExample::ALL {
                            option { value: "{ex.value()}", "{ex.name()}" }
                        }
                    }
                }

                // Editor
                div { class: "flex-1 text-xl overflow-hidden min-h-0",
                    Editor {
                        content,
                        tokens,
                        errors: combined_doc_errors,
                        theme,
                        on_change: move |s| content.set(s),
                    }
                }
            }

            // Right column: Tabbed view
            div {
                class: "w-1/2 flex flex-col rounded border min-h-0",
                style: "border-color: {border_color}; background-color: {bg_color}",

                // Tab header
                div {
                    class: "h-14 px-3 flex border-b shrink-0 items-center",
                    style: "border-color: {border_color}; background-color: {surface1_color}",

                    button {
                        class: "px-4 py-2 text-base font-semibold border-b-2 transition-colors",
                        style: if active_tab() == RightTab::JsonOutput { "border-color: currentColor" } else { "border-color: transparent" },
                        onclick: move |_| {
                            navigator.push(Route::Home {
                                example: example(),
                                tab: Some(RightTab::JsonOutput.value().to_string()),
                            });
                        },
                        "JSON Output"
                    }
                    button {
                        class: "px-4 py-2 text-base font-semibold border-b-2 transition-colors",
                        style: if active_tab() == RightTab::Schema { "border-color: currentColor" } else { "border-color: transparent" },
                        onclick: move |_| {
                            navigator.push(Route::Home {
                                example: example(),
                                tab: Some(RightTab::Schema.value().to_string()),
                            });
                        },
                        "Schema"
                    }
                    button {
                        class: "px-4 py-2 text-base font-semibold border-b-2 transition-colors flex items-center gap-2",
                        style: if active_tab() == RightTab::Errors { "border-color: currentColor" } else { "border-color: transparent" },
                        onclick: move |_| {
                            navigator.push(Route::Home {
                                example: example(),
                                tab: Some(RightTab::Errors.value().to_string()),
                            });
                        },
                        "Errors"
                        if error_count() > 0 {
                            span {
                                class: "inline-flex items-center justify-center min-w-5 h-5 px-1.5 text-xs font-bold rounded-full",
                                style: "background-color: {error_color}; color: white",
                                "{error_count()}"
                            }
                        }
                    }
                }

                // Tab content
                div { class: "flex-1 overflow-hidden min-h-0",
                    match active_tab() {
                        RightTab::JsonOutput => rsx! {
                            // Document Parser Errors

                            // Schema Parser Errors

                            // Schema Errors (conversion + meta-validation)

                            // Validation Errors (document vs schema)






                            div { class: "h-full overflow-auto p-3 font-mono text-sm",
                                pre {



                                    if json_output().is_empty() {
                                        span { class: "opacity-50", "// Parse the Eure document to see JSON output" }
                                    } else {
                                        "{json_output()}"
                                    }
                                }
                            }
                        },
                        RightTab::Schema => rsx! {
                            div { class: "h-full text-xl overflow-hidden",
                                Editor {
                                    content: schema_content,
                                    tokens: schema_tokens,
                                    errors: combined_schema_errors,
                                    theme,
                                    on_change: move |s| schema_content.set(s),
                                }
                            }
                        },
                        RightTab::Errors => rsx! {
                            div { class: "h-full overflow-auto p-3 font-mono text-sm",
                                if all_errors().is_empty() {
                                    span { class: "opacity-50", "No errors" }
                                } else {
                                    if !all_errors().doc_parser_errors.is_empty() {
                                        div { class: "mb-4",
                                            div { class: "text-xs font-bold uppercase opacity-60 mb-2",
                                                "Document Parser Errors ({all_errors().doc_parser_errors.len()})"
                                            }
                                            for error in all_errors().doc_parser_errors.iter() {
                                                div {
                                                    class: "mb-2 p-2 rounded border",
                                                    style: "border-color: {border_color}",
                                                    pre { class: "whitespace-pre-wrap", "{error.message}" }
                                                }
                                            }
                                        }
                                    }
                                    if !all_errors().schema_parser_errors.is_empty() {
                                        div { class: "mb-4",
                                            div { class: "text-xs font-bold uppercase opacity-60 mb-2",
                                                "Schema Parser Errors ({all_errors().schema_parser_errors.len()})"
                                            }
                                            for error in all_errors().schema_parser_errors.iter() {
                                                div {
                                                    class: "mb-2 p-2 rounded border",
                                                    style: "border-color: {border_color}",
                                                    pre { class: "whitespace-pre-wrap", "{error.message}" }
                                                }
                                            }
                                        }
                                    }
                                    if !all_errors().schema_errors.is_empty() {
                                        div { class: "mb-4",
                                            div { class: "text-xs font-bold uppercase opacity-60 mb-2",
                                                "Schema Conversion Errors ({all_errors().schema_errors.len()})"
                                            }
                                            for error in all_errors().schema_errors.iter() {
                                                div {
                                                    class: "mb-2 p-2 rounded border",
                                                    style: "border-color: {border_color}",
                                                    pre { class: "whitespace-pre-wrap", "{error.message}" }
                                                }
                                            }
                                        }
                                    }
                                    if !all_errors().schema_validation_errors.is_empty() {
                                        div { class: "mb-4",
                                            div { class: "text-xs font-bold uppercase opacity-60 mb-2",
                                                "Schema Validation Errors ({all_errors().schema_validation_errors.len()})"
                                            }
                                            for error in all_errors().schema_validation_errors.iter() {
                                                div {
                                                    class: "mb-2 p-2 rounded border",
                                                    style: "border-color: {border_color}",
                                                    pre { class: "whitespace-pre-wrap", "{error.message}" }
                                                }
                                            }
                                        }
                                    }
                                    if !all_errors().validation_errors.is_empty() {
                                        div { class: "mb-4",
                                            div { class: "text-xs font-bold uppercase opacity-60 mb-2",
                                                "Validation Errors ({all_errors().validation_errors.len()})"
                                            }
                                            for error in all_errors().validation_errors.iter() {
                                                div {
                                                    class: "mb-2 p-2 rounded border",
                                                    style: "border-color: {border_color}",
                                                    pre { class: "whitespace-pre-wrap", "{error.message}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                    }
                }
            }
        }
    }
}
