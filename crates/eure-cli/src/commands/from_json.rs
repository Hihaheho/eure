use eure::query::{DocumentToSchemaQuery, TextFile, TextFileContent, build_runtime};
use eure::query_flow::DurabilityLevel;
use eure_json::value_to_document_with_variant_repr;
use eure_document::document::EureDocument;
use eure_document::source::{EureSource, SourceDocument};
use eure_schema::interop::VariantRepr;
use eure_schema::{SchemaDocument, SchemaNodeContent};

use crate::util::{VariantFormat, read_input};

#[derive(clap::Args)]
pub struct Args {
    /// Path to JSON file to convert (use - for stdin)
    pub file: String,
    /// Optional schema file used to infer union variant representation (`$interop.variant-repr`)
    #[arg(short, long)]
    pub schema: Option<String>,
    /// Variant representation format
    ///
    /// If omitted:
    /// - with `--schema`: inferred from schema root union interop (falls back to untagged)
    /// - without `--schema`: defaults to untagged
    #[arg(short = 'v', long, value_enum)]
    pub variant: Option<VariantFormat>,
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

    // 3. Determine variant representation
    let variant_repr = if let Some(explicit) = args.variant.clone() {
        variant_format_to_repr(explicit, args.tag.clone(), args.content.clone())
    } else if let Some(schema_path) = args.schema.as_deref() {
        let schema_repr = load_schema_variant_repr(&runtime, schema_path);
        schema_repr.unwrap_or(VariantRepr::Untagged)
    } else {
        VariantRepr::Untagged
    };

    // 4. Convert JSON to EureDocument (schema/CLI-guided variant representation)
    let json_value: serde_json::Value = match serde_json::from_str(&contents) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error parsing JSON: {e}");
            std::process::exit(1);
        }
    };
    let document = match value_to_document_with_variant_repr(&json_value, &variant_repr) {
        Ok(doc) => doc,
        Err(e) => {
            eprintln!("Error converting JSON to Eure: {e}");
            std::process::exit(1);
        }
    };

    // 5. Build minimal SourceDocument for formatting
    let source_doc = build_minimal_source_document(std::sync::Arc::new(document));

    // 6. Format and output
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

fn variant_format_to_repr(format: VariantFormat, tag: String, content: String) -> VariantRepr {
    match format {
        VariantFormat::External => VariantRepr::External,
        VariantFormat::Internal => VariantRepr::Internal { tag },
        VariantFormat::Adjacent => VariantRepr::Adjacent { tag, content },
        VariantFormat::Untagged => VariantRepr::Untagged,
    }
}

fn load_schema_variant_repr(
    runtime: &eure::query_flow::QueryRuntime,
    schema_path: &str,
) -> Option<VariantRepr> {
    let schema_file = TextFile::from_path(schema_path.into());
    let schema_contents = match std::fs::read_to_string(schema_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading schema file: {e}");
            std::process::exit(1);
        }
    };
    runtime.resolve_asset(
        schema_file.clone(),
        TextFileContent(schema_contents),
        DurabilityLevel::Static,
    );

    let validated = match runtime.query(DocumentToSchemaQuery::new(schema_file)) {
        Ok(schema) => schema,
        Err(e) => {
            eprintln!("Error loading schema: {e}");
            std::process::exit(1);
        }
    };

    root_union_variant_repr(&validated.schema)
}

fn root_union_variant_repr(schema: &SchemaDocument) -> Option<VariantRepr> {
    match &schema.node(schema.root).content {
        SchemaNodeContent::Union(union) => union.interop.variant_repr.clone(),
        _ => None,
    }
}
