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

/// Home page with the Eure editor
#[component]
pub fn Home() -> Element {
    let mut theme = use_signal(|| Theme::Dark);
    let mut content = use_signal(String::new);
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

			// Editor container
			div {
				class: "w-full max-w-4xl h-96 rounded border",
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
