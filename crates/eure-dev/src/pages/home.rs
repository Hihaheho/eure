use crate::{
    components::editor::{Editor, ErrorSpan},
    theme::Theme,
};
use dioxus::prelude::*;
use eure::document::cst_to_document;
use eure::error::format_parse_error_plain;
use eure_editor_support::semantic_token::{SemanticToken, semantic_tokens};
use eure_json::{Config as JsonConfig, document_to_value};
use eure_parol::{ParseResult, parse_tolerant};

/// Parsed result containing tokens, errors, and JSON output
#[derive(Debug, Clone, Default, PartialEq)]
struct ParsedData {
    tokens: Vec<SemanticToken>,
    errors: Vec<ErrorSpan>,
    json_output: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
enum RightTab {
    #[default]
    JsonOutput,
    Schema,
    Errors,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
enum EureExample {
    #[default]
    Readme,
    HelloWorld,
    EureSchema,
}

impl EureExample {
    const ALL: &'static [EureExample] = &[
        EureExample::Readme,
        EureExample::HelloWorld,
        EureExample::EureSchema,
    ];

    fn name(&self) -> &'static str {
        match self {
            EureExample::Readme => "Readme",
            EureExample::HelloWorld => "Hello World",
            EureExample::EureSchema => "Eure Schema",
        }
    }

    fn value(&self) -> &'static str {
        match self {
            EureExample::Readme => "readme",
            EureExample::HelloWorld => "hello-world",
            EureExample::EureSchema => "eure-schema",
        }
    }

    fn from_value(value: &str) -> Option<Self> {
        match value {
            "readme" => Some(EureExample::Readme),
            "hello-world" => Some(EureExample::HelloWorld),
            "eure-schema" => Some(EureExample::EureSchema),
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
        }
    }
}

/// Home page with the Eure editor
#[component]
pub fn Home() -> Element {
    let theme: Signal<Theme> = use_context();
    let mut example = use_signal(EureExample::default);
    let mut content = use_signal(|| EureExample::default().content().to_string());
    let parsed = use_memo(move || {
        let input = content();
        let result = parse_tolerant(&input);

        let (cst, error) = match result {
            ParseResult::Ok(cst) => (cst, None),
            ParseResult::ErrWithCst { cst, error } => (cst, Some(error)),
        };

        let tokens = semantic_tokens(&input, &cst);
        let errors = error
            .map(|e| {
                let message = format_parse_error_plain(&e, &input, "test.eure");
                e.entries
                    .into_iter()
                    .filter_map(|entry| {
                        entry.span.map(|s| ErrorSpan {
                            start: s.start,
                            end: s.end,
                            message: message.clone(),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // Convert to JSON if no errors
        let json_output = if errors.is_empty() {
            cst_to_document(&input, &cst)
                .ok()
                .and_then(|doc| document_to_value(&doc, &JsonConfig::default()).ok())
                .and_then(|v| serde_json::to_string_pretty(&v).ok())
                .unwrap_or_default()
        } else {
            String::new()
        };

        ParsedData {
            tokens,
            errors,
            json_output,
        }
    });
    // Create read signals for the editor
    let tokens = use_memo(move || parsed().tokens);
    let errors = use_memo(move || parsed().errors);
    let json_output = use_memo(move || parsed().json_output);

    // Schema editor state
    let mut schema_content = use_signal(|| EureExample::default().schema().to_string());
    let schema_parsed = use_memo(move || {
        let input = schema_content();
        let result = parse_tolerant(&input);

        let (cst, error) = match result {
            ParseResult::Ok(cst) => (cst, None),
            ParseResult::ErrWithCst { cst, error } => (cst, Some(error)),
        };

        let tokens = semantic_tokens(&input, &cst);
        let errors = error
            .map(|e| {
                let message = format_parse_error_plain(&e, &input, "schema.eure");
                e.entries
                    .into_iter()
                    .filter_map(|entry| {
                        entry.span.map(|s| ErrorSpan {
                            start: s.start,
                            end: s.end,
                            message: message.clone(),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        ParsedData {
            tokens,
            errors,
            json_output: String::new(),
        }
    });
    let schema_tokens = use_memo(move || schema_parsed().tokens);
    let schema_errors = use_memo(move || schema_parsed().errors);

    // Tab state for right column
    let mut active_tab = use_signal(RightTab::default);
    let error_count = use_memo(move || errors().len());

    let theme_val = theme();
    let bg_color = theme_val.bg_color();
    let border_color = theme_val.border_color();
    let surface1_color = theme_val.surface1_color();

    rsx! {
		div { class: "h-full px-4 pb-4 flex gap-4",

			// Left column: Eure Editor
			div {
				class: "w-1/2 flex flex-col rounded border min-h-0",
				style: "border-color: {border_color}; background-color: {bg_color}",

				// Section header
				div {
					class: "px-3 py-2 border-b text-sm font-semibold shrink-0 flex justify-between items-center",
					style: "border-color: {border_color}; background-color: {surface1_color}",
					span { "Eure" }
					select {
						class: "px-3 py-1 rounded border text-sm font-normal",
						style: "border-color: {border_color}; background-color: {bg_color}",
						value: "{example().value()}",
						onchange: move |evt| {
						    if let Some(ex) = EureExample::from_value(&evt.value()) {
						        example.set(ex);
						        content.set(ex.content().to_string());
						        schema_content.set(ex.schema().to_string());
						    }
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
						errors,
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
					class: "flex border-b shrink-0",
					style: "border-color: {border_color}; background-color: {surface1_color}",

					button {
						class: "px-4 py-2 text-sm font-semibold border-b-2 transition-colors",
						style: if active_tab() == RightTab::JsonOutput {
							"border-color: currentColor"
						} else {
							"border-color: transparent; opacity: 0.6"
						},
						onclick: move |_| active_tab.set(RightTab::JsonOutput),
						"JSON Output"
					}
					button {
						class: "px-4 py-2 text-sm font-semibold border-b-2 transition-colors",
						style: if active_tab() == RightTab::Schema {
							"border-color: currentColor"
						} else {
							"border-color: transparent; opacity: 0.6"
						},
						onclick: move |_| active_tab.set(RightTab::Schema),
						"Schema"
					}
					button {
						class: "px-4 py-2 text-sm font-semibold border-b-2 transition-colors",
						style: if active_tab() == RightTab::Errors {
							"border-color: currentColor"
						} else {
							"border-color: transparent; opacity: 0.6"
						},
						onclick: move |_| active_tab.set(RightTab::Errors),
						"Errors ({error_count()})"
					}
				}

				// Tab content
				div { class: "flex-1 overflow-hidden min-h-0",
					match active_tab() {
						RightTab::JsonOutput => rsx! {
							div { class: "h-full overflow-auto p-3 font-mono text-sm",
								pre {
									if json_output().is_empty() {
										span { class: "opacity-50",
											"// Parse the Eure document to see JSON output"
										}
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
									errors: schema_errors,
									theme,
									on_change: move |s| schema_content.set(s),
								}
							}
						},
						RightTab::Errors => rsx! {
							div { class: "h-full overflow-auto p-3 font-mono text-sm",
								if errors().is_empty() {
									span { class: "opacity-50", "No errors" }
								} else {
									for error in errors().iter() {
										div { class: "mb-2 p-2 rounded border",
											style: "border-color: {border_color}",
											pre { class: "whitespace-pre-wrap", "{error.message}" }
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
