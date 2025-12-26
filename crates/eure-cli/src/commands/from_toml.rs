use eure::document::cst_to_document;

use crate::util::{display_path, read_input};

#[derive(clap::Args)]
pub struct Args {
    /// Path to TOML file to convert (use - for stdin)
    pub file: String,
    /// No validate the conversion
    #[arg(short, long)]
    pub no_validate: bool,
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

    // Convert TOML to Eure SourceDocument
    let source_doc = match eure_toml::to_source_document(&contents) {
        Ok(doc) => doc,
        Err(e) => {
            eprintln!("Error parsing TOML from {}: {e}", display_path(file_opt));
            std::process::exit(1);
        }
    };

    // Format and output Eure
    let output = eure_toml::format_source_document(&source_doc);
    if !args.no_validate {
        validate(&contents, &output);
    }
    print!("{output}");
}

pub fn validate(toml: &str, eure: &str) {
    let toml_json = toml::from_str::<serde_json::Value>(toml).unwrap();
    let cst = eure_parol::parse(eure).unwrap();
    let document = cst_to_document(eure, &cst).unwrap();
    let eure_json = eure_json::document_to_value(&document, &eure_json::Config::default()).unwrap();
    if toml_json != eure_json {
        eprintln!(
            "Converted eure does not match original TOML. This is likely due to a bug in the eure-toml crate, please report it to the github repository."
        );
        std::process::exit(1);
    }
}
