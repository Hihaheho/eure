use clap::{Args, Parser, Subcommand, ValueEnum};
use eure::data_model::VariantRepr;
use eure::document::cst_to_document;
use eure::error::format_parse_error;
use eure::tree::{inspect_cst, write_cst};
use eure_editor_support::semantic_token::{SemanticTokenType, semantic_tokens};
use eure_fmt::unformat::{unformat, unformat_with_seed};
use eure_json::{Config as JsonConfig, document_to_value};
use nu_ansi_term::Color;
use std::fs;
use std::io::{self, Read, Write};

#[derive(Parser)]
#[command(name = "eure", about = "Eure file utilities")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse and display Eure file syntax tree
    Inspect(Inspect),
    /// Unformat Eure file
    Unformat(Unformat),
    /// Format Eure file
    Fmt(Fmt),
    /// Convert Eure to JSON
    ToJson(ToJson),
    /// Convert JSON to Eure
    FromJson(FromJson),
    /// Syntax highlight Eure file with colors
    Highlight(Highlight),
}

#[derive(Args)]
struct Inspect {
    /// Path to Eure file to inspect (use '-' or omit for stdin)
    file: Option<String>,
}

#[derive(Args)]
struct Unformat {
    /// Path to Eure file to unformat (use '-' for stdin)
    file: Option<String>,
    /// Seed for unformatting
    #[arg(short, long)]
    seed: Option<u64>,
}

#[derive(Args)]
struct Fmt {
    /// Path to Eure file to format (use '-' for stdin)
    file: Option<String>,
    /// Check mode - exit with non-zero status if formatting is needed
    #[arg(short, long)]
    check: bool,
    /// Indent width (default: 2)
    #[arg(short, long, default_value = "2")]
    indent_width: usize,
}

#[derive(ValueEnum, Clone, Debug)]
enum VariantFormat {
    /// Default: {"variant-name": {...}}
    External,
    /// {"type": "variant-name", ...fields...}
    Internal,
    /// {"type": "variant-name", "content": {...}}
    Adjacent,
    /// Just the content without variant information
    Untagged,
}

#[derive(Args)]
struct ToJson {
    /// Path to Eure file to convert (use - for stdin)
    file: String,
    /// Variant representation format
    #[arg(short = 'v', long, value_enum, default_value = "external")]
    variant: VariantFormat,
    /// Tag field name for internal/adjacent representations
    #[arg(short = 't', long, default_value = "type")]
    tag: String,
    /// Content field name for adjacent representation
    #[arg(short = 'c', long, default_value = "content")]
    content: String,
    /// Pretty print JSON output
    #[arg(short, long)]
    pretty: bool,
}

#[derive(Args)]
struct FromJson {
    /// Path to JSON file to convert (use - for stdin)
    file: String,
    /// Variant representation format
    #[arg(short = 'v', long, value_enum, default_value = "external")]
    variant: VariantFormat,
    /// Tag field name for internal/adjacent representations
    #[arg(short = 't', long, default_value = "type")]
    tag: String,
    /// Content field name for adjacent representation
    #[arg(short = 'c', long, default_value = "content")]
    content: String,
}

#[derive(Args)]
struct Highlight {
    /// Path to Eure file to highlight (use '-' or omit for stdin)
    file: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Inspect(Inspect { file }) => {
            let contents = match file.as_deref() {
                None | Some("-") => {
                    // Read from stdin
                    let mut buffer = String::new();
                    if let Err(e) = io::stdin().read_to_string(&mut buffer) {
                        eprintln!("Error reading from stdin: {e}");
                        std::process::exit(1);
                    }
                    buffer
                }
                Some(path) => match fs::read_to_string(path) {
                    Ok(contents) => contents,
                    Err(e) => {
                        eprintln!("Error reading file: {e}");
                        return;
                    }
                },
            };

            let parse_result = eure_parol::parse_tolerant(&contents);

            // Print any parse errors
            if let Some(error) = parse_result.error() {
                let path = file.as_deref().unwrap_or("<stdin>");
                eprintln!("{}", format_parse_error(error, &contents, path));
                eprintln!("Note: Showing partial syntax tree below");
                eprintln!();
            }

            let tree = parse_result.cst();
            let mut out = String::new();
            if let Err(e) = inspect_cst(&contents, &tree, &mut out) {
                eprintln!("Error inspecting tree: {e}");
                std::process::exit(1);
            }
            println!("{out}");
        }
        Commands::Unformat(Unformat { file, seed }) => {
            // Read input from file or stdin
            let contents = match file.as_deref() {
                None | Some("-") => {
                    // Read from stdin
                    let mut buffer = String::new();
                    if let Err(e) = io::stdin().read_to_string(&mut buffer) {
                        eprintln!("Error reading from stdin: {e}");
                        std::process::exit(1);
                    }
                    buffer
                }
                Some(path) => match fs::read_to_string(path) {
                    Ok(contents) => contents,
                    Err(e) => {
                        eprintln!("Error reading file: {e}");
                        std::process::exit(1);
                    }
                },
            };

            let mut tree = match eure_parol::parse(&contents) {
                Ok(tree) => tree,
                Err(e) => {
                    let path = file.as_deref().unwrap_or("<stdin>");
                    eprintln!("{}", format_parse_error(&e, &contents, path));
                    std::process::exit(1);
                }
            };

            if let Some(seed) = seed {
                unformat_with_seed(&mut tree, seed);
            } else {
                unformat(&mut tree);
            }

            let mut out = String::new();
            if let Err(e) = write_cst(&contents, &tree, &mut out) {
                eprintln!("Error writing output: {e}");
                std::process::exit(1);
            }
            print!("{out}");
        }
        Commands::Fmt(_) => {
            eprintln!("Error: Formatting is not yet implemented.");
            eprintln!("The formatter API is currently under development.");
            eprintln!("Use `eure unformat` to remove all formatting instead.");
            std::process::exit(1);
        }
        Commands::ToJson(args) => handle_to_json(args),
        Commands::FromJson(args) => handle_from_json(args),
        Commands::Highlight(args) => handle_highlight(args),
    }
}

fn handle_to_json(args: ToJson) {
    // Read input
    let contents = if args.file == "-" {
        let mut buffer = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut buffer) {
            eprintln!("Error reading from stdin: {e}");
            std::process::exit(1);
        }
        buffer
    } else {
        match fs::read_to_string(&args.file) {
            Ok(contents) => contents,
            Err(e) => {
                eprintln!("Error reading file: {e}");
                std::process::exit(1);
            }
        }
    };

    // Parse Eure
    let tree = match eure_parol::parse(&contents) {
        Ok(tree) => tree,
        Err(e) => {
            let path = if args.file == "-" {
                "<stdin>"
            } else {
                &args.file
            };
            eprintln!("{}", format_parse_error(&e, &contents, path));
            std::process::exit(1);
        }
    };

    // Extract document from CST
    let document = match cst_to_document(&contents, &tree) {
        Ok(doc) => doc,
        Err(e) => {
            eprintln!("Error converting CST to document: {e:?}");
            std::process::exit(1);
        }
    };

    // Configure variant representation
    let variant_repr = match args.variant {
        VariantFormat::External => VariantRepr::External,
        VariantFormat::Internal => VariantRepr::Internal { tag: args.tag },
        VariantFormat::Adjacent => VariantRepr::Adjacent {
            tag: args.tag,
            content: args.content,
        },
        VariantFormat::Untagged => VariantRepr::Untagged,
    };

    let config = JsonConfig { variant_repr };

    // Convert document to JSON
    let json_value = match document_to_value(&document, &config) {
        Ok(json) => json,
        Err(e) => {
            eprintln!("Error converting to JSON: {e}");
            std::process::exit(1);
        }
    };

    // Output JSON
    let output = if args.pretty {
        match serde_json::to_string_pretty(&json_value) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error serializing JSON: {e}");
                std::process::exit(1);
            }
        }
    } else {
        match serde_json::to_string(&json_value) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error serializing JSON: {e}");
                std::process::exit(1);
            }
        }
    };

    println!("{output}");
}

fn handle_from_json(_args: FromJson) {
    eprintln!("Error: JSON to Eure conversion is not yet implemented.");
    eprintln!("The reverse conversion API is currently under development.");
    eprintln!("You can only convert Eure → JSON using `eure to-json`.");
    std::process::exit(1);
}

fn handle_highlight(args: Highlight) {
    // Read input from file or stdin
    let contents = match args.file.as_deref() {
        None | Some("-") => {
            let mut buffer = String::new();
            if let Err(e) = io::stdin().read_to_string(&mut buffer) {
                eprintln!("Error reading from stdin: {e}");
                std::process::exit(1);
            }
            buffer
        }
        Some(path) => match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(e) => {
                eprintln!("Error reading file: {e}");
                std::process::exit(1);
            }
        },
    };

    // Parse with tolerant mode to show partial highlighting even with errors
    let parse_result = eure_parol::parse_tolerant(&contents);

    // Print any parse errors to stderr
    if let Some(error) = parse_result.error() {
        let path = args.file.as_deref().unwrap_or("<stdin>");
        eprintln!("{}", format_parse_error(error, &contents, path));
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
