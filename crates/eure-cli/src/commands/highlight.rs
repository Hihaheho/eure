use eure::query::{GetSemanticTokens, TextFile, TextFileContent, build_runtime};
use eure::query_flow::DurabilityLevel;
use nu_ansi_term::Color;
use std::io::{self, Write};

use crate::util::{display_path, handle_query_error, read_input};

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

    // Create query runtime
    let runtime = build_runtime();

    let file = TextFile::from_path(display_path(args.file.as_deref()).into());
    runtime.resolve_asset(
        file.clone(),
        TextFileContent(contents.clone()),
        DurabilityLevel::Static,
    );

    // Get semantic tokens
    let tokens = match runtime.query(GetSemanticTokens::new(file)) {
        Ok(result) => result,
        Err(e) => handle_query_error(&runtime, e),
    };

    // Build colored output
    let mut stdout = io::stdout().lock();
    let mut pos = 0usize;

    for token in tokens.iter() {
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

fn token_type_to_color(token_type: eure::query::SemanticTokenType) -> Color {
    use eure::query::SemanticTokenType;
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
