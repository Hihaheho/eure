use eure::query::{TextFile, TextFileContent, ValidCst};
use eure::query_flow::{DurabilityLevel, QueryRuntimeBuilder};
use eure::tree::write_cst;
use eure_fmt::unformat::{unformat, unformat_with_seed};

use crate::util::{display_path, handle_query_error, read_input};

#[derive(clap::Args)]
pub struct Args {
    /// Path to Eure file to unformat (use '-' for stdin)
    pub file: Option<String>,
    /// Seed for unformatting
    #[arg(short, long)]
    pub seed: Option<u64>,
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
    let runtime = QueryRuntimeBuilder::new().build();

    let file = TextFile::from_path(display_path(args.file.as_deref()).into());
    runtime.resolve_asset(
        file.clone(),
        TextFileContent::Content(contents.clone()),
        DurabilityLevel::Static,
    );

    // Parse CST (fails on parse errors)
    let cst = match runtime.query(ValidCst::new(file)) {
        Ok(result) => result,
        Err(e) => handle_query_error(&runtime, e),
    };

    // Clone CST for mutation
    let mut tree = (*cst).clone();

    if let Some(seed) = args.seed {
        unformat_with_seed(&mut tree, seed);
    } else {
        unformat(&mut tree);
    }

    let mut out = String::new();
    if let Err(e) = write_cst(&contents, &tree, &mut out) {
        eprintln!("Error writing output: {e}");
        std::process::exit(1);
    }
    print!("{out}");
}
