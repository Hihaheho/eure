use crate::{
    components::editor::{Editor, ErrorSpan},
    theme::Theme,
};
use dioxus::prelude::*;
use eure::error::format_parse_error_plain;
use eure_editor_support::semantic_token::{SemanticToken, semantic_tokens};
use eure_parol::{ParseResult, parse_tolerant};

/// Parsed result containing tokens and errors
#[derive(Debug, Clone, Default, PartialEq)]
struct ParsedData {
    tokens: Vec<SemanticToken>,
    errors: Vec<ErrorSpan>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
enum EureExample {
    #[default]
    Readme,
    HelloWorld,
}

impl EureExample {
    const ALL: &'static [EureExample] = &[EureExample::Readme, EureExample::HelloWorld];

    fn name(&self) -> &'static str {
        match self {
            EureExample::Readme => "Readme",
            EureExample::HelloWorld => "Hello World",
        }
    }

    fn value(&self) -> &'static str {
        match self {
            EureExample::Readme => "readme",
            EureExample::HelloWorld => "hello-world",
        }
    }

    fn from_value(value: &str) -> Option<Self> {
        match value {
            "readme" => Some(EureExample::Readme),
            "hello-world" => Some(EureExample::HelloWorld),
            _ => None,
        }
    }

    fn content(&self) -> &'static str {
        match self {
            EureExample::Readme => include_str!("../../assets/readme.eure"),
            EureExample::HelloWorld => include_str!("../../assets/examples/hello-world.eure"),
        }
    }
}

/// Home page with the Eure editor
#[component]
pub fn Home() -> Element {
    let mut theme = use_signal(|| Theme::Dark);
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

        ParsedData { tokens, errors }
    });
    // Create read signals for the editor
    let tokens = use_memo(move || parsed().tokens);
    let errors = use_memo(move || parsed().errors);

    let theme_val = theme();
    let bg_color = theme_val.bg_color();
    let text_color = theme_val.text_color();
    let border_color = theme_val.border_color();

    rsx! {
        div {
            class: "min-h-screen p-4 flex flex-col items-center",
            style: "background-color: {bg_color}; color: {text_color}",

            // Header
            div { class: "w-full max-w-4xl mb-4 flex justify-between items-center",

                h1 { class: "text-2xl font-bold", "Eure Editor" }

                div { class: "flex gap-2 items-center",
                    // Example selector
                    select {
                        class: "px-4 py-2 rounded border",
                        style: "border-color: {border_color}; background-color: {bg_color}; color: {text_color}",
                        value: "{example().value()}",
                        onchange: move |evt| {
                            if let Some(ex) = EureExample::from_value(&evt.value()) {
                                example.set(ex);
                                content.set(ex.content().to_string());
                            }
                        },
                        for ex in EureExample::ALL {
                            option {
                                value: "{ex.value()}",
                                "{ex.name()}"
                            }
                        }
                    }

                    // Theme toggle button
                    button {
                        class: "px-4 py-2 rounded border",
                        style: "border-color: {border_color}",
                        onclick: move |_| theme.set(theme().toggle()),
                        if theme() == Theme::Dark {
                            "Light Mode"
                        } else {
                            "Dark Mode"
                        }
                    }
                }
            }

            // Editor container
            div {
                class: "w-full max-w-4xl h-full rounded border text-xl",
                style: "border-color: {border_color}",
                Editor {
                    content,
                    tokens,
                    errors,
                    theme,
                    on_change: move |s| content.set(s),
                }
            }

            // Status bar
            div { class: "w-full max-w-4xl mt-2 text-sm opacity-70 max-h-64 overflow-y-auto",
                if !parsed().errors.is_empty() {
                    for error in parsed().errors {
                        pre { class: "text-red-500", "{error.message}" }
                    }
                } else {
                    "No errors"
                }
            }
        }
    }
}
