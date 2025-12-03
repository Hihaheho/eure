mod editor;
mod theme;

use dioxus::prelude::*;
use editor::{Editor, ErrorSpan};
use eure_editor_support::semantic_token::{SemanticToken, semantic_tokens};
use eure_parol::{ParseResult, parse_tolerant};
use theme::Theme;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Navbar)]
    #[route("/")]
    Home {},
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        Router::<Route> {}
    }
}

/// Parsed result containing tokens and errors
#[derive(Debug, Clone, Default, PartialEq)]
struct ParsedData {
    tokens: Vec<SemanticToken>,
    errors: Vec<ErrorSpan>,
}

/// Home page with the Eure editor
#[component]
fn Home() -> Element {
    let mut theme = use_signal(|| Theme::Dark);
    let mut content = use_signal(String::new);
    let mut parsed = use_signal(ParsedData::default);

    // Update parsed data when content changes
    use_effect(move || {
        let input = content();
        let result = parse_tolerant(&input);

        let (cst, error) = match result {
            ParseResult::Ok(cst) => (cst, None),
            ParseResult::ErrWithCst { cst, error } => (cst, Some(error)),
        };

        let tokens = semantic_tokens(&input, &cst);
        let errors = error
            .and_then(|e| {
                e.span.map(|s| ErrorSpan {
                    start: s.start,
                    end: s.end,
                    message: e.message.clone(),
                })
            })
            .into_iter()
            .collect();

        parsed.set(ParsedData { tokens, errors });
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
            div {
                class: "w-full max-w-4xl mb-4 flex justify-between items-center",

                h1 {
                    class: "text-2xl font-bold",
                    "Eure Editor"
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
            div {
                class: "w-full max-w-4xl mt-2 text-sm opacity-70",
                {
                    let err_count = parsed().errors.len();
                    if err_count > 0 {
                        format!("{} error(s)", err_count)
                    } else {
                        "No errors".to_string()
                    }
                }
            }
        }
    }
}

/// Shared navbar component.
#[component]
fn Navbar() -> Element {
    rsx! {
        Outlet::<Route> {}
    }
}
