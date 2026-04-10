use eure::query::{
    DocumentToSchemaQuery, ParseDocument, TextFile, TextFileContent, WithFormattedError,
    build_runtime,
};
use eure::query_flow::DurabilityLevel;
use eure_document::document::EureDocument;
use eure_json::{Config as JsonConfig, EureToJsonFormatted};
use eure_schema::interop::VariantRepr;
use eure_schema::SchemaDocument;
use serde::Serialize;
use serde_json::Serializer as JsonSerializer;

use crate::args::CacheArgs;
use crate::util::{
    VariantFormat, display_path, handle_formatted_error, read_input,
    run_query_with_file_loading_cached,
};

#[derive(clap::Args)]
pub struct Args {
    /// Path to Eure file to convert (use - for stdin)
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
    /// Pretty print JSON output
    #[arg(short, long)]
    pub pretty: bool,
    /// Cache options when the schema is loaded from a remote URL
    #[command(flatten)]
    pub cache: CacheArgs,
}

/// Bridges `serde_json::Serializer` with serde-eure's schema-aware `Serialize` implementation.
struct SchemaAwareEureDocument<'a> {
    doc: &'a EureDocument,
    schema: &'a SchemaDocument,
}

impl Serialize for SchemaAwareEureDocument<'_> {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        serde_eure::to_serializer(ser, self.doc, self.schema)
    }
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

    let cache_opts = args.cache.to_cache_options();

    let json_value = if let Some(schema_path) = args.schema.as_ref() {
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

        let parsed = handle_formatted_error(run_query_with_file_loading_cached(
            &runtime,
            WithFormattedError::new(ParseDocument::new(file.clone()), true),
            Some(&cache_opts),
        ));

        let wrapped = SchemaAwareEureDocument {
            doc: &parsed.doc,
            schema: &validated.schema,
        };
        let mut buf = Vec::new();
        let mut ser = JsonSerializer::new(&mut buf);
        if let Err(e) = wrapped.serialize(&mut ser) {
            eprintln!("Schema-aware JSON serialization failed: {e}");
            std::process::exit(1);
        }
        match serde_json::from_slice(&buf) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Internal error building JSON value: {e}");
                std::process::exit(1);
            }
        }
    } else {
        // Configure variant representation (schema-free path)
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

        handle_formatted_error(runtime.query(WithFormattedError::new(
            EureToJsonFormatted::new(file.clone(), config),
            true,
        )))
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
