//! Syntax-highlighted editor component for Eure.

use std::rc::Rc;

use crate::theme::Theme;
use catppuccin::Hex;
use dioxus::events::MountedData;
use dioxus::prelude::*;
use eure::query::{SemanticToken, SemanticTokenType};

/// Error span information for displaying error underlines.
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorSpan {
    /// Start byte offset in the source.
    pub start: u32,
    /// End byte offset in the source.
    pub end: u32,
    /// Error message to display in tooltip.
    pub message: String,
}

/// A syntax-highlighted editor component.
#[component]
pub fn Editor(
    content: ReadSignal<String>,
    tokens: ReadSignal<Vec<SemanticToken>>,
    errors: ReadSignal<Vec<ErrorSpan>>,
    theme: ReadSignal<Theme>,
    on_change: EventHandler<String>,
) -> Element {
    let mut mouse_pos: Signal<Option<(f64, f64)>> = use_signal(|| None);
    let mut scroll_trigger = use_signal(|| ());

    let handle_input = move |e: Event<FormData>| {
        on_change.call(e.value());
    };

    let theme_val = theme();
    let bg_style = format!("background-color: {}", theme_val.bg_color());
    let caret_style = format!("caret-color: {}", theme_val.caret_color());

    rsx! {
        div {
            class: "w-full h-full font-mono text-sm",
            style: "{bg_style}",
            onmousemove: move |e: Event<MouseData>| {
                let coords = e.client_coordinates();
                mouse_pos.set(Some((coords.x, coords.y)));
            },
            onmouseleave: move |_| {
                mouse_pos.set(None);
            },
            onscroll: move |_| {
                scroll_trigger.set(());
            },

            div { class: "relative min-h-full",
                pre {
                    class: "m-0 p-2 border-0 pointer-events-none whitespace-pre-wrap break-words",
                    style: "font: inherit; line-height: 1.625",
                    Segments {
                        content,
                        tokens,
                        errors,
                        theme,
                        mouse_pos,
                        scroll_trigger,
                    }
                }

                textarea {
                    class: "absolute inset-0 w-full h-full m-0 p-2 bg-transparent text-transparent resize-none overflow-hidden whitespace-pre-wrap break-words outline-none border-0",
                    style: "{caret_style}; font: inherit; line-height: 1.625",
                    value: "{content}",
                    oninput: handle_input,
                    spellcheck: false,
                    autocomplete: "off",
                    autocorrect: "off",
                    autocapitalize: "off",
                }
            }
        }
    }
}

/// Renders all segments with syntax highlighting.
#[component]
fn Segments(
    content: ReadSignal<String>,
    tokens: ReadSignal<Vec<SemanticToken>>,
    errors: ReadSignal<Vec<ErrorSpan>>,
    theme: ReadSignal<Theme>,
    mouse_pos: Signal<Option<(f64, f64)>>,
    scroll_trigger: Signal<()>,
) -> Element {
    let input = content();
    let tokens = tokens();
    let errors = errors();
    let theme = theme();

    if input.is_empty() {
        return rsx! {
            span { "\u{200B}" }
        };
    }

    let input_len = input.len() as u32;
    let mut current_pos: u32 = 0;

    // Build render list: (start, end, token_type, has_error, error_message)
    let mut items: Vec<(&str, Option<SemanticTokenType>, Option<String>)> = Vec::new();

    for token in &tokens {
        let start = token.start.min(input_len);
        let end = (token.start + token.length).min(input_len);
        if start >= end {
            continue;
        }

        // Gap before this token
        if start > current_pos {
            items.push((&input[current_pos as usize..start as usize], None, None));
        }

        // Check error overlap
        let error_msg = errors
            .iter()
            .find(|e| start < e.end && end > e.start)
            .map(|e| e.message.clone());

        items.push((
            &input[start as usize..end as usize],
            Some(token.token_type),
            error_msg,
        ));
        current_pos = end;
    }

    // Trailing gap
    if current_pos < input_len {
        items.push((&input[current_pos as usize..input_len as usize], None, None));
    }

    rsx! {
        for (text , token_type , error_msg) in items {
            if let Some(msg) = error_msg {
                ErrorSegment {
                    text: text.to_string(),
                    color: token_type.map(|t| theme.token_color(t)).unwrap_or(theme.text_color()),
                    error_color: theme.error_color(),
                    message: msg,
                    mouse_pos,
                    scroll_trigger,
                    theme,
                }
            } else {
                Segment {
                    text: text.to_string(),
                    color: token_type.map(|t| theme.token_color(t)).unwrap_or(theme.text_color()),
                }
            }
        }
        if input.ends_with('\n') {
            span { "\u{200B}" }
        }
    }
}

/// A simple text segment.
#[component]
fn Segment(text: String, color: Hex) -> Element {
    rsx! {
        span { style: "color: {color}", "{text}" }
    }
}

/// Error segment with hover tooltip.
#[component]
fn ErrorSegment(
    text: String,
    color: Hex,
    error_color: Hex,
    message: String,
    mouse_pos: Signal<Option<(f64, f64)>>,
    scroll_trigger: Signal<()>,
    theme: Theme,
) -> Element {
    let mut rect: Signal<Option<(f64, f64, f64, f64)>> = use_signal(|| None);
    let mut mounted_el: Signal<Option<Rc<MountedData>>> = use_signal(|| None);

    use_effect(move || {
        scroll_trigger();
        if let Some(el) = mounted_el.read().clone() {
            spawn(async move {
                if let Ok(r) = el.get_client_rect().await {
                    rect.set(Some((
                        r.origin.x,
                        r.origin.y,
                        r.origin.x + r.size.width,
                        r.origin.y + r.size.height,
                    )));
                }
            });
        }
    });

    let is_hovered = use_memo(move || {
        let Some((mx, my)) = mouse_pos() else {
            return false;
        };
        let Some((left, top, right, bottom)) = rect() else {
            return false;
        };
        mx >= left && mx <= right && my >= top && my <= bottom
    });

    let style = if is_hovered() {
        format!(
            "anchor-name: --tooltip-anchor; color: {}; text-decoration-color: {}",
            color, error_color
        )
    } else {
        format!("color: {}; text-decoration-color: {}", color, error_color)
    };

    rsx! {
        span {
            class: "underline decoration-wavy pointer-events-none",
            style: "{style}",
            onmounted: move |e| {
                mounted_el.set(Some(e.data()));
            },
            "{text}"
            if is_hovered() {
                Tooltip { message: message.clone(), theme }
            }
        }
    }
}

/// Tooltip for error messages.
#[component]
fn Tooltip(message: String, theme: Theme) -> Element {
    let bg = theme.surface_color();
    let border = theme.error_color();
    let text = theme.text_color();

    rsx! {
        div {
            class: "z-50 px-2 py-1 rounded text-xs max-w-xs pointer-events-none",
            style: "position: fixed; position-anchor: --tooltip-anchor; bottom: anchor(top); left: anchor(left); margin-bottom: 4px; background-color: {bg}; border: 1px solid {border}; color: {text}",
            "{message}"
        }
    }
}
