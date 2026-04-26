use eure::query::{
    DocumentToSchemaQuery, TextFile, TextFileContent, WithFormattedError, build_runtime,
};
use eure::query_flow::DurabilityLevel;
use eure_json::{Config as JsonConfig, JsonToEure};
use eure_document::plan::LayoutPlan;
use eure_schema::interop::VariantRepr;

use crate::args::CacheArgs;
use crate::util::{
    VariantFormat, display_path, handle_formatted_error, handle_query_error, read_input,
    run_query_with_file_loading_cached,
};

#[derive(clap::Args)]
pub struct Args {
    /// Path to JSON file to convert (use - for stdin)
    pub file: String,
    /// Schema file for schema-aware JSON via serde-eure (union wire shape follows schema $interop)
    #[arg(short, long)]
    pub schema: Option<String>,
    /// Variant representation format (ignored when --schema is set)
    #[arg(short = 'v', long, value_enum, default_value = "external")]
    pub variant: VariantFormat,
    /// Tag field name for internal/adjacent representations
    #[arg(short = 't', long, default_value = "type")]
    pub tag: String,
    /// Content field name for adjacent representation
    #[arg(short = 'c', long, default_value = "content")]
    pub content: String,
    /// Cache options when the schema is loaded from a remote URL
    #[command(flatten)]
    pub cache: CacheArgs,
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

    let file = TextFile::from_path(display_path(file_opt).into());
    let cache_opts = args.cache.to_cache_options();

    // 3–5. Convert JSON to EureDocument
    let document = if let Some(schema_path) = args.schema.as_ref() {
        let schema_file = match TextFile::parse(schema_path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Invalid schema path or URL: {e}");
                std::process::exit(1);
            }
        };

        let validated = handle_formatted_error(run_query_with_file_loading_cached(
            &runtime,
            WithFormattedError::new(DocumentToSchemaQuery::new(schema_file), false),
            Some(&cache_opts),
        ));

        let mut de = serde_json::Deserializer::from_str(&contents);
        match serde_eure::from_deserializer(&mut de, &validated.schema).and_then(|doc| {
            de.end()
                .map_err(|e| serde_eure::DeError::Custom(e.to_string()))?;
            Ok(doc)
        }) {
            Ok(doc) => std::sync::Arc::new(doc),
            Err(e) => {
                eprintln!("Schema-aware JSON deserialization failed: {e}");
                std::process::exit(1);
            }
        }
    } else {
        runtime.resolve_asset(
            file.clone(),
            TextFileContent(contents),
            DurabilityLevel::Static,
        );

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

        match runtime.query(JsonToEure::new(file.clone(), config)) {
            Ok(doc) => doc,
            Err(e) => handle_query_error(e),
        }
    };

    // 6. Plan layout and build SourceDocument for formatting
    let doc = std::sync::Arc::unwrap_or_clone(document);
    let plan = match LayoutPlan::auto(doc) {
        Ok(plan) => plan,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };
    let source_doc = plan.emit();

    // 7. Format and output
    let output = eure_fmt::format_source_document(&source_doc);
    println!("{output}");
}
