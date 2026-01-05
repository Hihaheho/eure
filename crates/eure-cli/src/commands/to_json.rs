use eure::data_model::VariantRepr;
use eure::query::{TextFile, TextFileContent, build_runtime};
use eure::query_flow::DurabilityLevel;
use eure_json::{Config as JsonConfig, EureToJson};

use crate::util::{VariantFormat, display_path, handle_query_error, read_input};

#[derive(clap::Args)]
pub struct Args {
    /// Path to Eure file to convert (use - for stdin)
    pub file: String,
    /// Variant representation format
    #[arg(short = 'v', long, value_enum, default_value = "external")]
    pub variant: VariantFormat,
    /// Tag field name for internal/adjacent representations
    #[arg(short = 't', long, default_value = "type")]
    pub tag: String,
    /// Content field name for adjacent representation
    #[arg(short = 'c', long, default_value = "content")]
    pub content: String,
    /// Pretty print JSON output
    #[arg(short, long)]
    pub pretty: bool,
}

pub fn run(args: Args) {
    // Read input
    let file_opt = if args.file == "-" {
        None
    } else {
        Some(args.file.as_str())
    };
    let contents = match read_input(file_opt) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    // Create query runtime
    let runtime = build_runtime();

    let file = TextFile::from_path(display_path(file_opt).into());
    runtime.resolve_asset(
        file.clone(),
        TextFileContent(contents),
        DurabilityLevel::Static,
    );

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
    let json_value = match runtime.query(EureToJson::new(file.clone(), config)) {
        Ok(json) => json,
        Err(e) => handle_query_error(&runtime, e),
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
