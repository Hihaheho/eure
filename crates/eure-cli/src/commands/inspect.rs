use eure::query::{DecorStyle, DecorStyleKey, ParseCst, TextFile, TextFileContent, build_runtime};
use eure::query_flow::DurabilityLevel;
use eure::report::{format_error_reports, report_parse_error};
use eure::tree::inspect_cst;

use crate::util::{display_path, handle_query_error, read_input};

#[derive(clap::Args)]
pub struct Args {
    /// Path to Eure file to inspect (use '-' or omit for stdin)
    pub file: Option<String>,
}

pub fn run(args: Args) {
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

    let file = TextFile::from_path(display_path(args.file.as_deref()).into());
    runtime.resolve_asset(
        file.clone(),
        TextFileContent(contents.clone()),
        DurabilityLevel::Static,
    );

    // Parse with tolerant mode
    let parsed = match runtime.query(ParseCst::new(file.clone())) {
        Ok(result) => result,
        Err(e) => handle_query_error(&runtime, e),
    };

    // Print any parse errors
    if let Some(error) = &parsed.error {
        let reports = report_parse_error(error, file.clone());
        eprintln!(
            "{}",
            format_error_reports(&runtime, &reports, true).expect("file content should be loaded")
        );
        eprintln!("Note: Showing partial syntax tree below");
        eprintln!();
    }

    let mut out = String::new();
    if let Err(e) = inspect_cst(&contents, &parsed.cst, &mut out) {
        eprintln!("Error inspecting tree: {e}");
        std::process::exit(1);
    }
    println!("{out}");
}
