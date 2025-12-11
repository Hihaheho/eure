use std::path::{Path, PathBuf};

use eure::document::cst_to_document_and_origin_map;
use eure::error::{
    format_document_error, format_parse_error_color, format_schema_error, SchemaErrorContext,
};
use eure_config::{EureConfig, Target, CONFIG_FILENAME};
use eure_schema::convert::document_to_schema;
use eure_schema::validate::{validate, ValidationOutput, ValidationWarning};
use eure_schema::SchemaRef;
use nu_ansi_term::Color;

use crate::util::{display_path, read_input};

#[derive(clap::Args)]
pub struct Args {
    /// Path to Eure file to validate, or target names when using project mode.
    /// If omitted and Eure.eure exists, runs project mode with default targets.
    #[arg(num_args = 0..)]
    pub files_or_targets: Vec<String>,

    /// Path to schema file (overrides $schema in document)
    #[arg(short, long)]
    pub schema: Option<String>,

    /// Run all targets defined in Eure.eure
    #[arg(long)]
    pub all: bool,
}

pub fn run(args: Args) {
    // Determine mode: project mode or file mode
    let current_dir = std::env::current_dir().expect("Failed to get current directory");

    // Check for Eure.eure in current or parent directories
    if let Some(config_path) = EureConfig::find_config_file(&current_dir) {
        // Project mode
        run_project_mode(args, &config_path);
    } else if args.files_or_targets.is_empty() {
        eprintln!(
            "{}",
            Color::Red.paint("Error: No file specified and no Eure.eure found")
        );
        eprintln!("Usage: eure check <file> [--schema <schema>]");
        eprintln!("       eure check [targets...] (with Eure.eure)");
        std::process::exit(1);
    } else {
        // File mode - treat first argument as file
        run_file_mode(args);
    }
}

fn run_project_mode(args: Args, config_path: &Path) {
    let config = match EureConfig::load(config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "{}",
                Color::Red.paint(format!("Error loading {}: {e}", CONFIG_FILENAME))
            );
            std::process::exit(1);
        }
    };

    let config_dir = config_path.parent().unwrap_or(Path::new("."));

    // Determine which targets to run
    let target_names: Vec<&str> = if args.all {
        config.target_names().collect()
    } else if args.files_or_targets.is_empty() {
        // Use default targets
        config.default_targets().iter().map(|s| s.as_str()).collect()
    } else {
        // Check if arguments are target names or files
        let first = &args.files_or_targets[0];
        if config.get_target(first).is_some() || !Path::new(first).exists() {
            // Treat as target names
            args.files_or_targets.iter().map(|s| s.as_str()).collect()
        } else {
            // Treat as files - fall back to file mode
            return run_file_mode(args);
        }
    };

    if target_names.is_empty() {
        eprintln!(
            "{}",
            Color::Yellow.paint("No targets specified. Add targets to Eure.eure or use --all")
        );
        println!("\nAvailable targets:");
        for name in config.target_names() {
            println!("  - {}", name);
        }
        std::process::exit(0);
    }

    let mut total_errors = 0;
    let mut total_files = 0;

    for target_name in &target_names {
        let target = match config.get_target(target_name) {
            Some(t) => t,
            None => {
                eprintln!(
                    "{}",
                    Color::Red.paint(format!("Error: Unknown target '{}'", target_name))
                );
                println!("\nAvailable targets:");
                for name in config.target_names() {
                    println!("  - {}", name);
                }
                std::process::exit(1);
            }
        };

        let (files_checked, errors) = run_target(target_name, target, config_dir, &args);
        total_files += files_checked;
        total_errors += errors;
    }

    // Print summary
    println!();
    if total_errors == 0 {
        println!(
            "{} Checked {} file(s) in {} target(s) - all valid",
            Color::Green.bold().paint("✓"),
            total_files,
            target_names.len()
        );
        std::process::exit(0);
    } else {
        println!(
            "{} Checked {} file(s) in {} target(s) - {} error(s)",
            Color::Red.bold().paint("✗"),
            total_files,
            target_names.len(),
            total_errors
        );
        std::process::exit(1);
    }
}

fn run_target(name: &str, target: &Target, config_dir: &Path, args: &Args) -> (usize, usize) {
    println!(
        "\n{} Checking target: {}",
        Color::Blue.bold().paint("→"),
        Color::Cyan.paint(name)
    );

    let mut files_checked = 0;
    let mut errors = 0;

    // Expand globs
    let mut files: Vec<PathBuf> = Vec::new();
    for glob_pattern in &target.globs {
        let pattern = config_dir.join(glob_pattern);
        let pattern_str = pattern.to_string_lossy();

        match glob::glob(&pattern_str) {
            Ok(paths) => {
                for entry in paths {
                    match entry {
                        Ok(path) => files.push(path),
                        Err(e) => {
                            eprintln!(
                                "{}",
                                Color::Yellow.paint(format!("Warning: glob error: {e}"))
                            );
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "{}",
                    Color::Red.paint(format!(
                        "Error: Invalid glob pattern '{}': {e}",
                        glob_pattern
                    ))
                );
            }
        }
    }

    if files.is_empty() {
        println!(
            "  {}",
            Color::Yellow.paint(format!("No files matched for target '{}'", name))
        );
        return (0, 0);
    }

    // Resolve schema path
    let schema_path = args.schema.clone().or_else(|| {
        target
            .schema
            .as_ref()
            .map(|s| config_dir.join(s).to_string_lossy().to_string())
    });

    for file in files {
        files_checked += 1;
        let file_str = file.to_string_lossy().to_string();

        let result = check_single_file(&file_str, schema_path.as_deref());
        if !result {
            errors += 1;
        }
    }

    (files_checked, errors)
}

fn run_file_mode(args: Args) {
    if args.files_or_targets.is_empty() {
        eprintln!("{}", Color::Red.paint("Error: No file specified"));
        std::process::exit(1);
    }

    // Check first file (original behavior)
    let file = &args.files_or_targets[0];
    let success = check_single_file_verbose(file, args.schema.as_deref());

    if success {
        std::process::exit(0);
    } else {
        std::process::exit(1);
    }
}

/// Check a single file, returning true if valid
fn check_single_file(file: &str, schema_override: Option<&str>) -> bool {
    let file_opt = if file == "-" { None } else { Some(file) };

    let doc_contents = match read_input(file_opt) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "  {} {}: {}",
                Color::Red.paint("✗"),
                file,
                Color::Red.paint(e.to_string())
            );
            return false;
        }
    };

    // Parse document CST
    let doc_cst = match eure_parol::parse(&doc_contents) {
        Ok(tree) => tree,
        Err(e) => {
            eprintln!("  {} {}: parse error", Color::Red.paint("✗"), file);
            eprintln!(
                "{}",
                format_parse_error_color(&e, &doc_contents, display_path(file_opt))
            );
            return false;
        }
    };

    // Convert to document with origins
    let (document, doc_origins) = match cst_to_document_and_origin_map(&doc_contents, &doc_cst) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("  {} {}: document error", Color::Red.paint("✗"), file);
            eprintln!(
                "{}",
                format_document_error(&e, &doc_contents, display_path(file_opt), &doc_cst)
            );
            return false;
        }
    };

    // Determine schema path
    let schema_path = match determine_schema_path(schema_override, &document, file) {
        Ok(path) => path,
        Err(_) => {
            // No schema - just syntax check passed
            println!("  {} {} (syntax only)", Color::Green.paint("✓"), file);
            return true;
        }
    };

    // Load and validate against schema
    let schema_contents = match std::fs::read_to_string(&schema_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "  {} {}: schema error: {}",
                Color::Red.paint("✗"),
                file,
                e
            );
            return false;
        }
    };

    let schema_cst = match eure_parol::parse(&schema_contents) {
        Ok(tree) => tree,
        Err(e) => {
            eprintln!("  {} {}: schema parse error", Color::Red.paint("✗"), file);
            eprintln!(
                "{}",
                format_parse_error_color(&e, &schema_contents, &schema_path)
            );
            return false;
        }
    };

    let (schema_doc, schema_origins) =
        match cst_to_document_and_origin_map(&schema_contents, &schema_cst) {
            Ok(result) => result,
            Err(e) => {
                eprintln!(
                    "  {} {}: schema document error",
                    Color::Red.paint("✗"),
                    file
                );
                eprintln!(
                    "{}",
                    format_document_error(&e, &schema_contents, &schema_path, &schema_cst)
                );
                return false;
            }
        };

    let (schema, schema_source_map) = match document_to_schema(&schema_doc) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "  {} {}: schema conversion error: {}",
                Color::Red.paint("✗"),
                file,
                e
            );
            return false;
        }
    };

    let result = validate(&document, &schema);

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

    if result.is_valid {
        let status = if result.is_complete {
            Color::Green.paint("✓")
        } else {
            Color::Yellow.paint("!")
        };
        println!("  {} {}", status, file);

        // Print warnings inline
        for warning in &result.warnings {
            print_warning_inline(warning);
        }
        true
    } else {
        eprintln!("  {} {}", Color::Red.paint("✗"), file);
        for error in &result.errors {
            eprintln!("{}", format_schema_error(error, &context));
        }
        false
    }
}

/// Check a single file with verbose output (original behavior)
fn check_single_file_verbose(file: &str, schema_override: Option<&str>) -> bool {
    let file_opt = if file == "-" { None } else { Some(file) };

    let doc_contents = match read_input(file_opt) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", Color::Red.paint(format!("Error: {e}")));
            return false;
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
            return false;
        }
    };

    // Convert to document with origins
    let (document, doc_origins) = match cst_to_document_and_origin_map(&doc_contents, &doc_cst) {
        Ok(result) => result,
        Err(e) => {
            eprintln!(
                "{}",
                format_document_error(&e, &doc_contents, display_path(file_opt), &doc_cst)
            );
            return false;
        }
    };

    // Determine schema path
    let schema_path = match determine_schema_path(schema_override, &document, file) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("{}", Color::Red.paint(format!("Error: {e}")));
            return false;
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
            return false;
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
            return false;
        }
    };

    // Convert schema to document with origins
    let (schema_doc, schema_origins) =
        match cst_to_document_and_origin_map(&schema_contents, &schema_cst) {
            Ok(result) => result,
            Err(e) => {
                eprintln!(
                    "{}",
                    format_document_error(&e, &schema_contents, &schema_path, &schema_cst)
                );
                return false;
            }
        };

    // Convert to SchemaDocument
    let (schema, schema_source_map) = match document_to_schema(&schema_doc) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}", Color::Red.paint(format!("Error parsing schema: {e}")));
            return false;
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

    result.is_valid
}

/// Determine the schema path from override, $schema extension, or return error
fn determine_schema_path(
    schema_override: Option<&str>,
    document: &eure_document::document::EureDocument,
    doc_file: &str,
) -> Result<String, String> {
    // Use override if provided
    if let Some(path) = schema_override {
        return Ok(path.to_string());
    }

    // Parse SchemaRef from document root (extracts $schema extension)
    let schema_ref: SchemaRef = document.parse(document.get_root_id()).map_err(|_| {
        "No schema specified. Use --schema flag or add $schema extension to document.".to_string()
    })?;

    // Resolve relative to document directory
    if doc_file == "-" {
        return Ok(schema_ref.path);
    }

    let doc_path = Path::new(doc_file);
    let doc_dir = doc_path.parent().unwrap_or(Path::new("."));
    let resolved_path = doc_dir.join(&schema_ref.path);

    Ok(resolved_path.to_string_lossy().to_string())
}

/// Print validation results with span-annotated errors
fn print_validation_result(
    result: &ValidationOutput,
    context: &SchemaErrorContext<'_>,
    file_path: &str,
) {
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

/// Print a warning inline (for project mode)
fn print_warning_inline(warning: &ValidationWarning) {
    match warning {
        ValidationWarning::UnknownExtension { name, path } => {
            eprintln!(
                "    {}: Unknown extension '{}' at {}",
                Color::Yellow.paint("warn"),
                name,
                path
            );
        }
        ValidationWarning::DeprecatedField { field, path } => {
            eprintln!(
                "    {}: Deprecated field '{}' at {}",
                Color::Yellow.paint("warn"),
                field,
                path
            );
        }
    }
}
