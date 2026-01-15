use std::path::PathBuf;

use crate::{
    Route,
    components::editor::{Editor, ErrorSpan},
    theme::Theme,
};
use dioxus::prelude::*;
use eure::query::{
    DiagnosticMessage, GetFileDiagnostics, GetSemanticTokens, OpenDocuments, OpenDocumentsList,
    ParseDocument, SemanticToken, TextFile, TextFileContent, Workspace, WorkspaceId, build_runtime,
};
use query_flow::{Db, DurabilityLevel, QueryError, QueryRuntime, query};
use url::Url;

/// Convert document to pretty-printed JSON.
#[query]
fn document_to_json(db: &impl Db, file: TextFile) -> Result<String, QueryError> {
    let parsed = db.query(ParseDocument::new(file))?;

    let value = eure_json::document_to_value(&parsed.doc, &eure_json::Config::default())
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let json = serde_json::to_string_pretty(&value).map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(json)
}

/// Convert DiagnosticMessage to ErrorSpan for the editor UI.
fn diagnostic_to_error_span(diag: &DiagnosticMessage) -> ErrorSpan {
    ErrorSpan {
        start: diag.start as u32,
        end: diag.end as u32,
        message: diag.message.clone(),
    }
}

/// Filter diagnostics by file and convert to ErrorSpans.
fn diagnostics_to_spans(
    diagnostics: &[DiagnosticMessage],
    target_file: &TextFile,
) -> Vec<ErrorSpan> {
    diagnostics
        .iter()
        .filter(|d| &d.file == target_file)
        .map(diagnostic_to_error_span)
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
    SyntaxReference,
    EureSchema,
    Cargo,
    GitHubAction,
    GameScript,
    TextMateGrammar,
    Minimal,
}

impl EureExample {
    const ALL: &'static [EureExample] = &[
        EureExample::Readme,
        EureExample::HelloWorld,
        EureExample::SyntaxReference,
        EureExample::EureSchema,
        EureExample::Cargo,
        EureExample::GitHubAction,
        EureExample::GameScript,
        EureExample::TextMateGrammar,
        EureExample::Minimal,
    ];

    fn name(&self) -> &'static str {
        match self {
            EureExample::Readme => "Readme",
            EureExample::HelloWorld => "Hello World",
            EureExample::SyntaxReference => "Syntax Reference",
            EureExample::EureSchema => "Eure Schema",
            EureExample::Cargo => "Cargo",
            EureExample::GitHubAction => "GitHub Action",
            EureExample::GameScript => "Game Script",
            EureExample::TextMateGrammar => "TextMate Grammar",
            EureExample::Minimal => "Minimal",
        }
    }

    fn value(&self) -> &'static str {
        match self {
            EureExample::Readme => "readme",
            EureExample::HelloWorld => "hello-world",
            EureExample::SyntaxReference => "syntax-reference",
            EureExample::EureSchema => "eure-schema",
            EureExample::Cargo => "cargo",
            EureExample::GitHubAction => "github-action",
            EureExample::GameScript => "game-script",
            EureExample::TextMateGrammar => "textmate-grammar",
            EureExample::Minimal => "minimal",
        }
    }

    fn from_value(value: &str) -> Option<Self> {
        match value {
            "readme" => Some(EureExample::Readme),
            "hello-world" => Some(EureExample::HelloWorld),
            "syntax-reference" => Some(EureExample::SyntaxReference),
            "eure-schema" => Some(EureExample::EureSchema),
            "cargo" => Some(EureExample::Cargo),
            "github-action" => Some(EureExample::GitHubAction),
            "game-script" => Some(EureExample::GameScript),
            "textmate-grammar" => Some(EureExample::TextMateGrammar),
            "minimal" => Some(EureExample::Minimal),
            _ => None,
        }
    }

    fn content(&self) -> &'static str {
        match self {
            EureExample::Readme => include_str!("../../assets/readme.eure"),
            EureExample::HelloWorld => include_str!("../../assets/examples/hello-world.eure"),
            EureExample::SyntaxReference => {
                include_str!("../../assets/examples/syntax-reference.eure")
            }
            EureExample::EureSchema => {
                include_str!("../../../../assets/schemas/eure-schema.schema.eure")
            }
            EureExample::Cargo => include_str!("../../assets/examples/cargo.eure"),
            EureExample::GitHubAction => include_str!("../../assets/examples/github-action.eure"),
            EureExample::GameScript => {
                include_str!("../../../../assets/examples/game-script.eure")
            }
            EureExample::TextMateGrammar => {
                include_str!("../../../../editors/vscode/syntaxes/eure.tmLanguage.eure")
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
            EureExample::SyntaxReference => {
                include_str!("../../assets/examples/syntax-reference.schema.eure")
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
            EureExample::TextMateGrammar => {
                include_str!("../../../../assets/schemas/textmate-grammar.schema.eure")
            }
            EureExample::Minimal => "= `any`\n",
        }
    }

    fn file_name(&self) -> &'static str {
        match self {
            EureExample::Readme => "/readme.eure",
            EureExample::HelloWorld => "/hello-world.eure",
            EureExample::SyntaxReference => "/syntax-reference.eure",
            EureExample::EureSchema => "/eure-schema.schema.eure",
            EureExample::Cargo => "/cargo.eure",
            EureExample::GitHubAction => "/github-action.eure",
            EureExample::GameScript => "/game-script.eure",
            EureExample::TextMateGrammar => "/textmate-grammar.eure",
            EureExample::Minimal => "/minimal.eure",
        }
    }

    fn schema_file_name(&self) -> &'static str {
        match self {
            EureExample::Readme => "/readme.schema.eure",
            EureExample::HelloWorld => "/hello-world.schema.eure",
            EureExample::SyntaxReference => "/syntax-reference.schema.eure",
            EureExample::EureSchema => "/eure-schema.schema.eure",
            EureExample::Cargo => "/cargo.schema.eure",
            EureExample::GitHubAction => "/github-action.schema.eure",
            EureExample::GameScript => "/game-script.schema.eure",
            EureExample::TextMateGrammar => "/textmate-grammar.schema.eure",
            EureExample::Minimal => "/minimal.schema.eure",
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
            TextFile::from_path("/meta-schema.eure".into()),
            TextFileContent(
                include_str!("../../../../assets/schemas/eure-schema.schema.eure").to_string(),
            ),
            DurabilityLevel::Static,
        );

        // Register workspace config for schema resolution
        // Build config content that maps each example file to its schema
        let config_content = Self::build_config_content();
        runtime.resolve_asset(
            TextFile::from_path("/eure.config.eure".into()),
            TextFileContent(config_content),
            DurabilityLevel::Static,
        );
        runtime.resolve_asset(
            WorkspaceId("eure-dev".to_string()),
            Workspace {
                path: PathBuf::from("/"),
                config_path: PathBuf::from("/eure.config.eure"),
            },
            DurabilityLevel::Static,
        );

        // Register initial open documents (default example)
        let default_example = EureExample::default();
        runtime.resolve_asset(
            OpenDocuments,
            OpenDocumentsList(vec![
                TextFile::from_path(default_example.file_name().into()),
                TextFile::from_path(default_example.schema_file_name().into()),
            ]),
            DurabilityLevel::Volatile,
        );
    }

    /// Build eure.config.eure content for all examples.
    fn build_config_content() -> String {
        let mut config = String::from("// Auto-generated config for eure-dev examples\n");
        for example in EureExample::ALL {
            config.push_str(&format!(
                "\n@ targets.{}\nglobs[] = \"{}\"\nschema = \"{}\"\n",
                example.value().replace('-', "_"),
                example.file_name(),
                example.schema_file_name()
            ));
        }
        config
    }

    fn on_change_tab(&self, runtime: &QueryRuntime) {
        let doc_file = TextFile::from_path(self.file_name().into());
        let schema_file = TextFile::from_path(self.schema_file_name().into());

        runtime.resolve_asset(
            doc_file.clone(),
            TextFileContent(self.content().to_string()),
            DurabilityLevel::Volatile,
        );
        runtime.resolve_asset(
            schema_file.clone(),
            TextFileContent(self.schema().to_string()),
            DurabilityLevel::Volatile,
        );

        // Register open documents for diagnostic collection
        runtime.resolve_asset(
            OpenDocuments,
            OpenDocumentsList(vec![doc_file, schema_file]),
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

/// Fetch a remote URL using browser fetch API
async fn fetch_remote_url(url: &str) -> Result<String, String> {
    use gloo_net::http::Request;

    Request::get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch {}: {}", url, e))?
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))
}

/// Fetch pending remote assets and resolve them in the runtime
async fn fetch_and_resolve_assets(runtime: QueryRuntime, pending_urls: Vec<Url>) {
    for url in pending_urls {
        let url_str = url.to_string();
        tracing::info!("Fetching remote schema: {}", url_str);

        match fetch_remote_url(&url_str).await {
            Ok(content) => {
                tracing::info!("Successfully fetched {}", url_str);
                runtime.resolve_asset(
                    TextFile::Remote(url.clone()),
                    TextFileContent(content),
                    DurabilityLevel::Static,
                );
            }
            Err(e) => {
                tracing::error!("Failed to fetch {}: {}", url_str, e);
                // Resolve with error comment so queries can complete
                // This allows diagnostics to show a fetch error instead of staying suspended
                let error_content =
                    format!("// Failed to fetch schema: {}\n// URL: {}", e, url_str);
                runtime.resolve_asset(
                    TextFile::Remote(url.clone()),
                    TextFileContent(error_content),
                    DurabilityLevel::Volatile, // Volatile so retry is possible
                );
            }
        }
    }
}

/// Run all queries and update signals
/// Returns Vec of pending remote URLs that need to be fetched
fn run_queries(
    runtime: &QueryRuntime,
    doc_file: &TextFile,
    schema_file: &TextFile,
    mut doc_tokens: Signal<Vec<SemanticToken>>,
    mut schema_tokens: Signal<Vec<SemanticToken>>,
    mut json_output: Signal<String>,
    mut all_errors: Signal<AllErrors>,
) -> Vec<Url> {
    let mut pending_urls = Vec::new();

    // Helper to collect pending remote URLs from suspended queries
    let mut collect_pending_urls = || {
        for asset in runtime.pending_assets() {
            if let Some(TextFile::Remote(url)) = asset.key::<TextFile>()
                && !pending_urls.contains(url)
            {
                pending_urls.push(url.clone());
            }
        }
    };

    // Get semantic tokens for document
    match runtime.query(GetSemanticTokens::new(doc_file.clone())) {
        Ok(result) => doc_tokens.set((*result).clone()),
        Err(QueryError::Suspend { .. }) => collect_pending_urls(),
        Err(e) => tracing::error!("Semantic tokens query failed: {}", e),
    }

    // Get semantic tokens for schema
    match runtime.query(GetSemanticTokens::new(schema_file.clone())) {
        Ok(result) => schema_tokens.set((*result).clone()),
        Err(QueryError::Suspend { .. }) => collect_pending_urls(),
        Err(e) => tracing::error!("Semantic tokens query failed: {}", e),
    }

    // Get JSON output
    match runtime.query(DocumentToJson::new(doc_file.clone())) {
        Ok(json) => json_output.set(json.as_ref().clone()),
        Err(QueryError::Suspend { .. }) => collect_pending_urls(),
        Err(_) => json_output.set(String::new()),
    }

    // Get diagnostics for the document
    let doc_errors = match runtime.query(GetFileDiagnostics::new(doc_file.clone())) {
        Ok(diagnostics) => diagnostics_to_spans(&diagnostics, doc_file),
        Err(QueryError::Suspend { .. }) => {
            collect_pending_urls();
            vec![]
        }
        Err(e) => {
            tracing::error!("Diagnostics query failed: {}", e);
            vec![]
        }
    };

    let schema_errors = match runtime.query(GetFileDiagnostics::new(schema_file.clone())) {
        Ok(diagnostics) => diagnostics_to_spans(&diagnostics, schema_file),
        Err(QueryError::Suspend { .. }) => {
            collect_pending_urls();
            vec![]
        }
        Err(e) => {
            tracing::error!("Diagnostics query failed: {}", e);
            vec![]
        }
    };

    all_errors.set(AllErrors {
        doc_errors,
        schema_errors,
    });

    pending_urls
}

/// Home page with the Eure editor
#[component]
pub fn Home(example: ReadSignal<Option<String>>, tab: ReadSignal<Option<String>>) -> Element {
    let theme: Signal<Theme> = use_context();
    let navigator = use_navigator();
    let runtime = use_signal(|| {
        let runtime = build_runtime();
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

    // Loading state signal for remote schema fetching
    let mut pending_remote_urls = use_signal(Vec::<Url>::new);

    // Helper to run queries and fetch pending remote assets
    let mut run_queries_with_fetch = move |doc_file: TextFile, schema_file: TextFile| {
        let pending_urls = run_queries(
            &runtime(),
            &doc_file,
            &schema_file,
            doc_tokens,
            schema_tokens,
            json_output,
            all_errors,
        );

        // Update pending URLs state (used for loading indicator)
        pending_remote_urls.set(pending_urls.clone());

        if !pending_urls.is_empty() {
            // Spawn async task to fetch remote assets
            let runtime_clone = runtime();
            let doc_file_clone = doc_file.clone();
            let schema_file_clone = schema_file.clone();
            spawn(async move {
                fetch_and_resolve_assets(runtime_clone.clone(), pending_urls).await;

                // Re-run queries now that assets are resolved
                let new_pending_urls = run_queries(
                    &runtime_clone,
                    &doc_file_clone,
                    &schema_file_clone,
                    doc_tokens,
                    schema_tokens,
                    json_output,
                    all_errors,
                );

                // Update pending URLs (should be empty now)
                pending_remote_urls.set(new_pending_urls);
            });
        }
    };

    // Update content and run queries when example changes
    use_effect(move || {
        let ex = current_example();
        content.set(ex.content().to_string());
        schema_content.set(ex.schema().to_string());
        ex.on_change_tab(&runtime());

        let doc_file = TextFile::from_path(ex.file_name().into());
        let schema_file = TextFile::from_path(ex.schema_file_name().into());
        run_queries_with_fetch(doc_file, schema_file);
    });

    // Handler for document content changes
    let update_content = move |value: String| {
        content.set(value.clone());
        let ex = current_example();
        ex.on_input(&runtime(), value);

        let doc_file = TextFile::from_path(ex.file_name().into());
        let schema_file = TextFile::from_path(ex.schema_file_name().into());
        run_queries_with_fetch(doc_file, schema_file);
    };

    // Handler for schema content changes
    let update_schema = move |value: String| {
        schema_content.set(value.clone());
        let ex = current_example();
        ex.on_schema_input(&runtime(), value);

        let doc_file = TextFile::from_path(ex.file_name().into());
        let schema_file = TextFile::from_path(ex.schema_file_name().into());
        run_queries_with_fetch(doc_file, schema_file);
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
                                // Show loading indicator when fetching remote schemas
                                if !pending_remote_urls().is_empty() {
                                    div {
                                        class: "mb-3 p-2 rounded",
                                        style: "background: rgba(100, 150, 255, 0.1); border-left: 3px solid #6495ED;",
                                        "📡 リモートスキーマを取得中..."
                                    }
                                }

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
