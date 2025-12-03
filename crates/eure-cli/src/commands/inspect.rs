use eure::error::format_parse_error_color;
use eure::tree::inspect_cst;

use crate::util::{display_path, read_input};

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

    let parse_result = eure_parol::parse_tolerant(&contents);

    // Print any parse errors
    if let Some(error) = parse_result.error() {
        eprintln!(
            "{}",
            format_parse_error_color(error, &contents, display_path(args.file.as_deref()))
        );
        eprintln!("Note: Showing partial syntax tree below");
        eprintln!();
    }

    let tree = parse_result.cst();
    let mut out = String::new();
    if let Err(e) = inspect_cst(&contents, &tree, &mut out) {
        eprintln!("Error inspecting tree: {e}");
        std::process::exit(1);
    }
    println!("{out}");
}
