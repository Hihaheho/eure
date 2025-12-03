use eure::error::format_parse_error_color;
use eure_editor_support::semantic_token::{SemanticTokenType, semantic_tokens};
use nu_ansi_term::Color;
use std::io::{self, Write};

use crate::util::{display_path, read_input};

#[derive(clap::Args)]
pub struct Args {
    /// Path to Eure file to highlight (use '-' or omit for stdin)
    pub file: Option<String>,
}

pub fn run(args: Args) {
    let contents = match read_input(args.file.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    // Parse with tolerant mode to show partial highlighting even with errors
    let parse_result = eure_parol::parse_tolerant(&contents);

    // Print any parse errors to stderr
    if let Some(error) = parse_result.error() {
        eprintln!(
            "{}",
            format_parse_error_color(error, &contents, display_path(args.file.as_deref()))
        );
        eprintln!();
    }

    let cst = parse_result.cst();

    // Get semantic tokens
    let tokens = semantic_tokens(&contents, &cst);

    // Build colored output
    let mut stdout = io::stdout().lock();
    let mut pos = 0usize;

    for token in &tokens {
        let start = token.start as usize;
        let end = start + token.length as usize;

        // Print any content before this token (whitespace, etc.) without color
        if pos < start {
            let _ = write!(stdout, "{}", &contents[pos..start]);
        }

        // Print the token with its color
        let text = &contents[start..end];
        let color = token_type_to_color(token.token_type);
        let _ = write!(stdout, "{}", color.paint(text));

        pos = end;
    }

    // Print any remaining content after the last token
    if pos < contents.len() {
        let _ = write!(stdout, "{}", &contents[pos..]);
    }

    let _ = stdout.flush();
}

fn token_type_to_color(token_type: SemanticTokenType) -> Color {
    match token_type {
        SemanticTokenType::Keyword => Color::Purple,
        SemanticTokenType::Number => Color::Cyan,
        SemanticTokenType::String => Color::Green,
        SemanticTokenType::Comment => Color::DarkGray,
        SemanticTokenType::Operator => Color::White,
        SemanticTokenType::Property => Color::Yellow,
        SemanticTokenType::Punctuation => Color::LightGray,
        SemanticTokenType::Macro => Color::LightCyan,
        SemanticTokenType::Decorator => Color::Magenta,
        SemanticTokenType::SectionMarker => Color::LightRed,
        SemanticTokenType::ExtensionMarker => Color::LightMagenta,
        SemanticTokenType::ExtensionIdent => Color::LightPurple,
    }
}
