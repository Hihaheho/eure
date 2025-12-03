use eure::error::format_parse_error_color;
use eure::tree::write_cst;
use eure_fmt::unformat::{unformat, unformat_with_seed};

use crate::util::{display_path, read_input};

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

    let mut tree = match eure_parol::parse(&contents) {
        Ok(tree) => tree,
        Err(e) => {
            eprintln!(
                "{}",
                format_parse_error_color(&e, &contents, display_path(args.file.as_deref()))
            );
            std::process::exit(1);
        }
    };

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
