use crate::{
    Route,
    components::editor::{Editor, ErrorSpan},
    theme::Theme,
};
use dioxus::prelude::*;
use eure::query::{
    GetSemanticTokens, ParseDocument, SemanticToken, TextFile, TextFileContent, ValidateDocument,
};
use eure::report::{ErrorReports, error_reports_comparator, format_error_report};
use query_flow::{Db, DurabilityLevel, QueryError, QueryRuntime, QueryRuntimeBuilder, query};

/// Convert document to pretty-printed JSON.
#[query]
fn document_to_json(db: &impl Db, file: TextFile) -> Result<String, QueryError> {
    let parsed = db.query(ParseDocument::new(file))?;

    let value = eure_json::document_to_value(&parsed.doc, &eure_json::Config::default())
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let json = serde_json::to_string_pretty(&value).map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(json)
}

/// Aggregated errors from document and schema validation.
#[derive(Clone, PartialEq, Default, Debug)]
struct EditorAllErrors {
    /// Errors from validating document against schema
    doc_errors: ErrorReports,
    /// Errors from validating schema against meta-schema
    schema_errors: ErrorReports,
}

/// Get all errors using SSoT ValidateDocument query.
///
/// Validates:
/// 1. Document against schema
/// 2. Schema against meta-schema
#[query]
fn get_all_errors(
    db: &impl Db,
    doc_file: TextFile,
    schema_file: TextFile,
    meta_schema_file: TextFile,
) -> Result<EditorAllErrors, QueryError> {
    // Validate schema against meta-schema first
    let schema_errors = db
        .query(ValidateDocument::new(
            schema_file.clone(),
            Some(meta_schema_file),
        ))?
        .as_ref()
        .clone();

    // Only validate document if schema is valid
    let doc_errors = if schema_errors.is_empty() {
        db.query(ValidateDocument::new(
            doc_file.clone(),
            Some(schema_file.clone()),
        ))?
        .as_ref()
        .clone()
    } else {
        ErrorReports::new()
    };

    Ok(EditorAllErrors {
        doc_errors,
        schema_errors,
    })
}

/// Convert ErrorReports to ErrorSpan list for UI.
fn error_reports_to_spans(db: &impl Db, reports: &ErrorReports) -> Vec<ErrorSpan> {
    reports
        .iter()
        .map(|report| {
            let formatted = format_error_report(db, report, false)
                .unwrap_or_else(|e| format!("Error formatting error report: {e}"));
            ErrorSpan {
                start: report.primary_origin.span.start,
                end: report.primary_origin.span.end,
                message: formatted,
            }
        })
        .collect()
}

/// All errors organized for display (simplified)
#[derive(Debug, Clone, Default, PartialEq)]
struct AllErrors {
    doc_errors: Vec<ErrorSpan>,
    schema_errors: Vec<ErrorSpan>,
}

impl AllErrors {
    fn total_count(&self) -> usize {
        self.doc_errors.len() + self.schema_errors.len()
    }

    fn is_empty(&self) -> bool {
        self.total_count() == 0
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

    fn file_name(&self) -> &'static str {
        match self {
            EureExample::Readme => "readme.eure",
            EureExample::HelloWorld => "hello-world.eure",
            EureExample::EureSchema => "eure-schema.schema.eure",
            EureExample::Cargo => "cargo.eure",
            EureExample::GitHubAction => "github-action.eure",
            EureExample::GameScript => "game-script.eure",
            EureExample::Minimal => "minimal.eure",
        }
    }

    fn schema_file_name(&self) -> &'static str {
        match self {
            EureExample::Readme => "readme.schema.eure",
            EureExample::HelloWorld => "hello-world.schema.eure",
            EureExample::EureSchema => "eure-schema.schema.eure",
            EureExample::Cargo => "cargo.schema.eure",
            EureExample::GitHubAction => "github-action.schema.eure",
            EureExample::GameScript => "game-script.schema.eure",
            EureExample::Minimal => "minimal.schema.eure",
        }
    }

    fn register_all(runtime: &QueryRuntime) {
        // Register all example files
        for example in EureExample::ALL {
            runtime.resolve_asset(
                TextFile::from_path(example.file_name().into()),
                TextFileContent(example.content().to_string()),
                DurabilityLevel::Volatile,
            );
            runtime.resolve_asset(
                TextFile::from_path(example.schema_file_name().into()),
                TextFileContent(example.schema().to_string()),
                DurabilityLevel::Volatile,
            );
        }
        // Register meta-schema for schema validation
        runtime.resolve_asset(
            TextFile::from_path("meta-schema.eure".into()),
            TextFileContent(
                include_str!("../../../../assets/schemas/eure-schema.schema.eure").to_string(),
            ),
            DurabilityLevel::Static,
        );
    }

    fn on_change_tab(&self, runtime: &QueryRuntime) {
        runtime.resolve_asset(
            TextFile::from_path(self.file_name().into()),
            TextFileContent(self.content().to_string()),
            DurabilityLevel::Volatile,
        );
        runtime.resolve_asset(
            TextFile::from_path(self.schema_file_name().into()),
            TextFileContent(self.schema().to_string()),
            DurabilityLevel::Volatile,
        );
    }

    fn on_input(&self, runtime: &QueryRuntime, value: String) {
        runtime.resolve_asset(
            TextFile::from_path(self.file_name().into()),
            TextFileContent(value),
            DurabilityLevel::Volatile,
        );
    }

    fn on_schema_input(&self, runtime: &QueryRuntime, value: String) {
        runtime.resolve_asset(
            TextFile::from_path(self.schema_file_name().into()),
            TextFileContent(value),
            DurabilityLevel::Volatile,
        );
    }
}

/// Run all queries and update signals
fn run_queries(
    runtime: &QueryRuntime,
    doc_file: &TextFile,
    schema_file: &TextFile,
    mut doc_tokens: Signal<Vec<SemanticToken>>,
    mut schema_tokens: Signal<Vec<SemanticToken>>,
    mut json_output: Signal<String>,
    mut all_errors: Signal<AllErrors>,
) {
    // Get semantic tokens for document
    if let Ok(result) = runtime.query(GetSemanticTokens::new(doc_file.clone())) {
        doc_tokens.set((*result).clone());
    }

    // Get semantic tokens for schema
    if let Ok(result) = runtime.query(GetSemanticTokens::new(schema_file.clone())) {
        schema_tokens.set((*result).clone());
    }

    // Get JSON output
    if let Ok(json) = runtime.query(DocumentToJson::new(doc_file.clone())) {
        json_output.set(json.as_ref().clone());
    }

    // Get all errors
    let meta_file = TextFile::from_path("meta-schema.eure".into());
    match runtime.query(GetAllErrors::new(
        doc_file.clone(),
        schema_file.clone(),
        meta_file,
    )) {
        Ok(errors) => {
            all_errors.set(AllErrors {
                doc_errors: error_reports_to_spans(runtime, &errors.doc_errors),
                schema_errors: error_reports_to_spans(runtime, &errors.schema_errors),
            });
        }
        Err(e) => {
            // Query failed unexpectedly - show the error
            all_errors.set(AllErrors {
                doc_errors: vec![ErrorSpan {
                    start: 0,
                    end: 1,
                    message: e.to_string(),
                }],
                ..Default::default()
            });
        }
    }
}

/// Home page with the Eure editor
#[component]
pub fn Home(example: ReadSignal<Option<String>>, tab: ReadSignal<Option<String>>) -> Element {
    let theme: Signal<Theme> = use_context();
    let navigator = use_navigator();
    let runtime = use_signal(|| {
        let runtime = QueryRuntimeBuilder::new()
            .error_comparator(error_reports_comparator)
            .build();
        EureExample::register_all(&runtime);
        runtime
    });

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

    // Content signals (updated from runtime assets)
    let mut content = use_signal(|| EureExample::default().content().to_string());
    let mut schema_content = use_signal(|| EureExample::default().schema().to_string());

    // Derived data signals (updated by queries)
    let doc_tokens: Signal<Vec<SemanticToken>> = use_signal(Vec::new);
    let schema_tokens: Signal<Vec<SemanticToken>> = use_signal(Vec::new);
    let json_output: Signal<String> = use_signal(String::new);
    let all_errors: Signal<AllErrors> = use_signal(AllErrors::default);

    // Update content and run queries when example changes
    use_effect(move || {
        let ex = current_example();
        content.set(ex.content().to_string());
        schema_content.set(ex.schema().to_string());
        ex.on_change_tab(&runtime());

        let doc_file = TextFile::from_path(ex.file_name().into());
        let schema_file = TextFile::from_path(ex.schema_file_name().into());
        run_queries(
            &runtime(),
            &doc_file,
            &schema_file,
            doc_tokens,
            schema_tokens,
            json_output,
            all_errors,
        );
    });

    // Handler for document content changes
    let update_content = move |value: String| {
        content.set(value.clone());
        let ex = current_example();
        ex.on_input(&runtime(), value);

        let doc_file = TextFile::from_path(ex.file_name().into());
        let schema_file = TextFile::from_path(ex.schema_file_name().into());
        run_queries(
            &runtime(),
            &doc_file,
            &schema_file,
            doc_tokens,
            schema_tokens,
            json_output,
            all_errors,
        );
    };

    // Handler for schema content changes
    let update_schema = move |value: String| {
        schema_content.set(value.clone());
        let ex = current_example();
        ex.on_schema_input(&runtime(), value);

        let doc_file = TextFile::from_path(ex.file_name().into());
        let schema_file = TextFile::from_path(ex.schema_file_name().into());
        run_queries(
            &runtime(),
            &doc_file,
            &schema_file,
            doc_tokens,
            schema_tokens,
            json_output,
            all_errors,
        );
    };

    // Schema errors for the schema editor
    let combined_schema_errors = use_memo(move || all_errors().schema_errors.clone());

    // Document errors for the document editor
    let combined_doc_errors = use_memo(move || all_errors().doc_errors.clone());

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
                        tokens: doc_tokens,
                        errors: combined_doc_errors,
                        theme,
                        on_change: update_content,
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
                                    on_change: update_schema,
                                }
                            }
                        },
                        RightTab::Errors => rsx! {
                            div { class: "h-full overflow-auto p-3 font-mono text-sm",
                                if all_errors().is_empty() {
                                    span { class: "opacity-50", "No errors" }
                                } else {
                                    if !all_errors().doc_errors.is_empty() {
                                        div { class: "mb-4",
                                            div { class: "text-xs font-bold uppercase opacity-60 mb-2",
                                                "Document Errors ({all_errors().doc_errors.len()})"
                                            }
                                            for error in all_errors().doc_errors.iter() {
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
                                                "Schema Errors ({all_errors().schema_errors.len()})"
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
                                }
                            }
                        },
                    }
                }
            }
        }
    }
}
