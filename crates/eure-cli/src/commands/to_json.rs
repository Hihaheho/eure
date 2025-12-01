use eure::data_model::VariantRepr;
use eure::document::cst_to_document;
use eure::error::format_parse_error;
use eure_json::{document_to_value, Config as JsonConfig};

use crate::util::{display_path, read_input, VariantFormat};

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

    // Parse Eure
    let tree = match eure_parol::parse(&contents) {
        Ok(tree) => tree,
        Err(e) => {
            eprintln!(
                "{}",
                format_parse_error(&e, &contents, display_path(file_opt))
            );
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
