//! Eure Markdown commands

use clap::{Parser, Subcommand};
use eure::document::cst_to_document_and_origin_map;
use eure::error::format_parse_error_color;
use eure_mark::{check_references_with_spans, format_check_errors, EumdDocument};

use crate::util::{display_path, read_input};

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

    let path = display_path(args.file.as_deref());
    let parse_result = eure_parol::parse_tolerant(&contents);

    if let Some(error) = parse_result.error() {
        eprintln!("{}", format_parse_error_color(error, &contents, path));
        std::process::exit(1);
    }

    let cst = parse_result.cst();
    let (doc, origin_map) = match cst_to_document_and_origin_map(&contents, &cst) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Error converting to document: {e}");
            std::process::exit(1);
        }
    };

    let root_id = doc.get_root_id();
    let eumd_doc: EumdDocument = match doc.parse(root_id) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error parsing eumd document: {e}");
            std::process::exit(1);
        }
    };

    // Use the span-aware check function
    let result = check_references_with_spans(&eumd_doc, &doc);

    if result.is_ok() {
        println!("\x1b[1;32m✓\x1b[0m {} references OK", path);
    } else {
        // Format errors with source spans using styled output
        let formatted = format_check_errors(&result, &contents, path, &cst, &origin_map, true);
        eprintln!("{formatted}");
        std::process::exit(1);
    }
}
