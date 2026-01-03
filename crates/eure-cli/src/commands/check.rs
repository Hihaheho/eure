//! Check command - validates Eure files against schemas.
//!
//! Uses SSoT validation queries from eure crate.

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use eure::query::{
    TextFile, TextFileContent, ValidateDocument, ValidateTargetResult, ValidateTargets,
    ValidateTargetsResult, load_config,
};
use eure::query_flow::QueryRuntimeBuilder;
use eure::report::{ErrorReports, format_error_reports};
use eure_config::{CONFIG_FILENAME, EureConfig};
use nu_ansi_term::Color;

use crate::util::{display_path, handle_query_error, read_input, run_query_with_file_loading};

#[derive(clap::Args)]
pub struct Args {
    /// Path to Eure file to validate, or target names when using project mode.
    /// If omitted and Eure.eure exists, runs project mode with default targets.
    #[arg(num_args = 0..)]
    pub files_or_targets: Vec<String>,

    /// Path to schema file (overrides $schema in document)
    #[arg(short, long)]
    pub schema: Option<String>,

    /// Quiet mode: suppress per-file/per-target output on success (prints a single summary line).
    /// Warnings are treated as errors.
    #[arg(short, long)]
    pub quiet: bool,

    /// Run all targets defined in Eure.eure
    #[arg(long)]
    pub all: bool,
}

pub fn run(args: Args) {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");

    // Check for Eure.eure in current or parent directories
    if let Some(config_path) = EureConfig::find_config_file(&current_dir) {
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
        run_file_mode(args);
    }
}

fn run_project_mode(args: Args, config_path: &Path) {
    let start = Instant::now();

    let config = match load_config(config_path) {
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
        config
            .default_targets()
            .iter()
            .map(|s| s.as_str())
            .collect()
    } else {
        let first = &args.files_or_targets[0];
        if config.get_target(first).is_some() || !Path::new(first).exists() {
            args.files_or_targets.iter().map(|s| s.as_str()).collect()
        } else {
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

    // Collect targets
    let mut targets = Vec::new();
    for target_name in &target_names {
        let target = match config.get_target(target_name) {
            Some(t) => t.clone(),
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
        targets.push((target_name.to_string(), target));
    }

    // Create runtime and run validation
    let runtime = QueryRuntimeBuilder::new().build();

    let result = match run_query_with_file_loading(
        &runtime,
        ValidateTargets::new(Arc::new(targets), config_dir.to_path_buf()),
    ) {
        Ok(r) => r,
        Err(e) => handle_query_error(&runtime, e),
    };

    // Report results
    report_targets_result(&runtime, &result, &args, target_names.len(), start);
}

fn run_file_mode(args: Args) {
    if args.files_or_targets.is_empty() {
        eprintln!("{}", Color::Red.paint("Error: No file specified"));
        std::process::exit(1);
    }

    let start = Instant::now();
    let file = &args.files_or_targets[0];
    let file_opt = if file == "-" {
        None
    } else {
        Some(file.as_str())
    };

    // Create runtime
    let runtime = QueryRuntimeBuilder::new().build();

    // Read and register document content
    let doc_contents = match read_input(file_opt) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", Color::Red.paint(format!("Error: {e}")));
            std::process::exit(1);
        }
    };

    let doc_file = TextFile::from_path(display_path(file_opt).into());
    runtime.resolve_asset(doc_file.clone(), TextFileContent::Content(doc_contents));

    // Register schema override if provided
    let schema_file = args.schema.as_ref().map(|path| {
        let sf = TextFile::from_path(path.into());
        if let Ok(content) = std::fs::read_to_string(path) {
            runtime.resolve_asset(sf.clone(), TextFileContent::Content(content));
        }
        sf
    });

    // Single query for validation
    let result = match run_query_with_file_loading(
        &runtime,
        ValidateDocument::new(doc_file.clone(), schema_file),
    ) {
        Ok(r) => r,
        Err(e) => handle_query_error(&runtime, e),
    };

    // Report result
    report_document_result(&runtime, file, &result, args.quiet, start);
}

fn report_document_result(
    runtime: &eure::query_flow::QueryRuntime,
    file: &str,
    result: &Arc<ErrorReports>,
    quiet: bool,
    start: Instant,
) {
    let duration_s = start.elapsed().as_secs_f64();
    let errors = result.as_ref();

    if errors.is_empty() {
        if quiet {
            println!("eure check: 1 file, ok in {:.2}s", duration_s);
        } else {
            println!();
            println!("{} {} is valid", Color::Green.bold().paint("✓"), file);
        }
        std::process::exit(0);
    } else {
        if quiet {
            println!(
                "eure check: 1 file, {} error(s) in {:.2}s",
                errors.len(),
                duration_s
            );
        }
        eprintln!(
            "{}",
            format_error_reports(runtime, errors, true).expect("file content should be loaded")
        );
        if !quiet {
            println!(
                "{} {} has {} error(s)",
                Color::Red.bold().paint("✗"),
                file,
                errors.len()
            );
        }
        std::process::exit(1);
    }
}

fn report_targets_result(
    runtime: &eure::query_flow::QueryRuntime,
    result: &Arc<ValidateTargetsResult>,
    args: &Args,
    target_count: usize,
    start: Instant,
) {
    let duration_s = start.elapsed().as_secs_f64();
    let total_files: usize = result.iter().map(|(_, r)| r.len()).sum();
    let total_errors: usize = result
        .iter()
        .flat_map(|(_, r)| r.iter())
        .filter(|(_, e)| !e.is_empty())
        .count();

    // Print per-target results if not quiet
    if !args.quiet {
        for (name, target_result) in result.iter() {
            println!(
                "\n{} Checking target: {}",
                Color::Blue.bold().paint("→"),
                Color::Cyan.paint(name)
            );

            if target_result.is_empty() {
                println!(
                    "  {}",
                    Color::Yellow.paint(format!("No files matched for target '{}'", name))
                );
            } else {
                report_target_errors(runtime, target_result);
            }
        }
    } else {
        // In quiet mode, still print errors
        for (_, target_result) in result.iter() {
            report_target_errors(runtime, target_result);
        }
    }

    // Print summary
    if args.quiet {
        if total_errors == 0 {
            println!(
                "eure check: {} file(s), {} target(s), ok in {:.2}s",
                total_files, target_count, duration_s
            );
            std::process::exit(0);
        } else {
            println!(
                "eure check: {} file(s), {} target(s), {} error(s) in {:.2}s",
                total_files, target_count, total_errors, duration_s
            );
            std::process::exit(1);
        }
    } else {
        println!();
        if total_errors == 0 {
            println!(
                "{} Checked {} file(s) in {} target(s) - all valid",
                Color::Green.bold().paint("✓"),
                total_files,
                target_count
            );
            std::process::exit(0);
        } else {
            println!(
                "{} Checked {} file(s) in {} target(s) - {} error(s)",
                Color::Red.bold().paint("✗"),
                total_files,
                target_count,
                total_errors
            );
            std::process::exit(1);
        }
    }
}

fn report_target_errors(
    runtime: &eure::query_flow::QueryRuntime,
    target_result: &ValidateTargetResult,
) {
    for (file, errors) in target_result.iter() {
        if !errors.is_empty() {
            eprintln!(
                "{}",
                format_error_reports(runtime, errors, true).expect("file content should be loaded")
            );
            eprintln!("  {} {}", Color::Red.paint("✗"), file.path.display());
        }
    }
}
