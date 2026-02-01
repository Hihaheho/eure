//! Eure Markdown commands

use clap::{Parser, Subcommand};
use eure::query::{
    DecorStyle, DecorStyleKey, TextFile, TextFileContent, WithFormattedError, build_runtime,
};
use eure::query_flow::DurabilityLevel;
use eure::report::format_error_reports;
use eure_mark::CheckEumdReferences;

use crate::util::{display_path, handle_formatted_error, read_input};

#[derive(Parser)]
pub struct Args {
    #[command(subcommand)]
    command: MarkCommands,
}

#[derive(Subcommand)]
enum MarkCommands {
    /// Check references in eumd file (!cite, !footnote, !ref)
    Check(CheckArgs),
}

#[derive(Parser)]
struct CheckArgs {
    /// Path to eumd file to check (use '-' or omit for stdin)
    file: Option<String>,
}

pub fn run(args: Args) {
    match args.command {
        MarkCommands::Check(check_args) => run_check(check_args),
    }
}

fn run_check(args: CheckArgs) {
    let contents = match read_input(args.file.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    // Create query runtime
    let runtime = build_runtime();

    // Register DecorStyle preference
    runtime.resolve_asset(
        DecorStyleKey,
        DecorStyle::Unicode, // CLI uses Unicode by default
        DurabilityLevel::Static,
    );

    let path = display_path(args.file.as_deref());
    let file: TextFile = TextFile::from_path(path.into());
    runtime.resolve_asset(
        file.clone(),
        TextFileContent(contents),
        DurabilityLevel::Static,
    );

    // Check eumd references (single query handles parsing + validation)
    let reports = handle_formatted_error(runtime.query(WithFormattedError::new(
        CheckEumdReferences::new(file.clone()),
        true,
    )));

    if reports.is_empty() {
        println!("\x1b[1;32m✓\x1b[0m {} references OK", file);
    } else {
        eprintln!(
            "{}",
            format_error_reports(&runtime, &reports, true).expect("file content should be loaded")
        );
        std::process::exit(1);
    }
}
