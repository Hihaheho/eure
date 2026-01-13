use eure::data_model::VariantRepr;
use eure::query::{TextFile, TextFileContent, build_runtime};
use eure::query_flow::DurabilityLevel;
use eure_json::{Config as JsonConfig, JsonToEure};
use eure_document::document::EureDocument;
use eure_document::source::{EureSource, SourceDocument};

use crate::util::{VariantFormat, display_path, handle_query_error, read_input};

#[derive(clap::Args)]
pub struct Args {
    /// Path to JSON file to convert (use - for stdin)
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
}

pub fn run(args: Args) {
    // 1. Read input from file or stdin
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

    // 2. Create query runtime
    let runtime = build_runtime();

    // 3. Register JSON file as asset
    let file = TextFile::from_path(display_path(file_opt).into());
    runtime.resolve_asset(
        file.clone(),
        TextFileContent(contents),
        DurabilityLevel::Static,
    );

    // 4. Configure variant representation
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

    // 5. Execute query to convert JSON to EureDocument
    let document = match runtime.query(JsonToEure::new(file.clone(), config)) {
        Ok(doc) => doc,
        Err(e) => handle_query_error(&runtime, e),
    };

    // 6. Build minimal SourceDocument for formatting
    let source_doc = build_minimal_source_document(document);

    // 7. Format and output
    let output = eure_fmt::format_source_document(&source_doc);
    println!("{output}");
}

fn build_minimal_source_document(document: std::sync::Arc<EureDocument>) -> SourceDocument {
    // Create a minimal SourceDocument with a single root source
    // that has the document's root as a value binding
    let doc = std::sync::Arc::unwrap_or_clone(document);
    let root_source = EureSource {
        value: Some(doc.get_root_id()),
        ..Default::default()
    };

    SourceDocument::new(doc, vec![root_source])
}
