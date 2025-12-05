use std::path::Path;

use eure::document::cst_to_document_and_origins;
use eure::error::{format_document_error, format_parse_error_color, format_schema_error, SchemaErrorContext};
use eure_schema::SchemaRef;
use eure_schema::convert::document_to_schema;
use eure_schema::validate::{validate, ValidationResult, ValidationWarning};
use nu_ansi_term::Color;

use crate::util::{display_path, read_input};

#[derive(clap::Args)]
pub struct Args {
    /// Path to Eure file to validate
    pub file: String,
    /// Path to schema file (overrides $schema in document)
    #[arg(short, long)]
    pub schema: Option<String>,
}

pub fn run(args: Args) {
    // Read document file
    let file_opt = if args.file == "-" {
        None
    } else {
        Some(args.file.as_str())
    };

    let doc_contents = match read_input(file_opt) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", Color::Red.paint(format!("Error: {e}")));
            std::process::exit(1);
        }
    };

    // Parse document CST
    let doc_cst = match eure_parol::parse(&doc_contents) {
        Ok(tree) => tree,
        Err(e) => {
            eprintln!(
                "{}",
                format_parse_error_color(&e, &doc_contents, display_path(file_opt))
            );
            std::process::exit(1);
        }
    };

    // Convert to document with origins (keep CST for span resolution)
    let (document, doc_origins) = match cst_to_document_and_origins(&doc_contents, &doc_cst) {
        Ok(result) => result,
        Err(e) => {
            eprintln!(
                "{}",
                format_document_error(&e, &doc_contents, display_path(file_opt), &doc_cst)
            );
            std::process::exit(1);
        }
    };

    // Determine schema path
    let schema_path = match determine_schema_path(&args, &document, &args.file) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("{}", Color::Red.paint(format!("Error: {e}")));
            std::process::exit(1);
        }
    };

    // Read schema file
    let schema_contents = match std::fs::read_to_string(&schema_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "{}",
                Color::Red.paint(format!("Error reading schema file '{}': {e}", schema_path))
            );
            std::process::exit(1);
        }
    };

    // Parse schema CST
    let schema_cst = match eure_parol::parse(&schema_contents) {
        Ok(tree) => tree,
        Err(e) => {
            eprintln!(
                "{}",
                format_parse_error_color(&e, &schema_contents, &schema_path)
            );
            std::process::exit(1);
        }
    };

    // Convert schema to document with origins
    let (schema_doc, schema_origins) = match cst_to_document_and_origins(&schema_contents, &schema_cst) {
        Ok(result) => result,
        Err(e) => {
            eprintln!(
                "{}",
                format_document_error(&e, &schema_contents, &schema_path, &schema_cst)
            );
            std::process::exit(1);
        }
    };

    // Convert to SchemaDocument (keep source_map for error formatting)
    let (schema, schema_source_map) = match document_to_schema(&schema_doc) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}", Color::Red.paint(format!("Error parsing schema: {e}")));
            std::process::exit(1);
        }
    };

    // Run validation
    let result = validate(&document, &schema);

    // Create error context for span-annotated formatting
    let context = SchemaErrorContext {
        doc_source: &doc_contents,
        doc_path: display_path(file_opt),
        doc_cst: &doc_cst,
        doc_origins: &doc_origins,
        schema_source: &schema_contents,
        schema_path: &schema_path,
        schema_cst: &schema_cst,
        schema_origins: &schema_origins,
        schema_source_map: &schema_source_map,
    };

    // Output results
    print_validation_result(&result, &context, display_path(file_opt));

    // Exit with appropriate code
    if result.is_valid {
        std::process::exit(0);
    } else {
        std::process::exit(1);
    }
}

/// Determine the schema path from --schema flag or $schema extension
fn determine_schema_path(
    args: &Args,
    document: &eure_value::document::EureDocument,
    doc_file: &str,
) -> Result<String, String> {
    // Use --schema flag if provided
    if let Some(path) = &args.schema {
        return Ok(path.clone());
    }

    // Parse SchemaRef from document root (extracts $schema extension)
    let schema_ref: SchemaRef = document
        .parse(document.get_root_id())
        .map_err(|e| format!("No schema specified. Use --schema flag or add $schema extension to document.\nParse error: {:?}", e))?;

    // Resolve relative to document directory
    if doc_file == "-" {
        // stdin - use current working directory
        return Ok(schema_ref.path);
    }

    let doc_path = Path::new(doc_file);
    let doc_dir = doc_path.parent().unwrap_or(Path::new("."));
    let resolved_path = doc_dir.join(&schema_ref.path);

    Ok(resolved_path.to_string_lossy().to_string())
}

/// Print validation results with span-annotated errors
fn print_validation_result(result: &ValidationResult, context: &SchemaErrorContext<'_>, file_path: &str) {
    // Print errors with span annotations
    for error in &result.errors {
        eprintln!("{}", format_schema_error(error, context));
    }

    // Print warnings
    for warning in &result.warnings {
        print_warning(warning);
    }

    // Print summary
    println!();
    if result.is_valid {
        if result.is_complete {
            println!(
                "{} {} is valid and complete",
                Color::Green.bold().paint("✓"),
                file_path
            );
        } else {
            println!(
                "{} {} is valid but contains holes",
                Color::Yellow.bold().paint("!"),
                file_path
            );
        }
    } else {
        println!(
            "{} {} has {} error(s)",
            Color::Red.bold().paint("✗"),
            file_path,
            result.errors.len()
        );
    }

    if !result.warnings.is_empty() {
        println!(
            "  {} warning(s)",
            Color::Yellow.paint(result.warnings.len().to_string())
        );
    }
}

/// Print a single validation warning
fn print_warning(warning: &ValidationWarning) {
    let warning_prefix = Color::Yellow.bold().paint("warning");
    match warning {
        ValidationWarning::UnknownExtension { name, path } => {
            eprintln!(
                "{}: Unknown extension '{}' at {}",
                warning_prefix, name, path
            );
        }
        ValidationWarning::DeprecatedField { field, path } => {
            eprintln!(
                "{}: Deprecated field '{}' at {}",
                warning_prefix, field, path
            );
        }
    }
}
