use clap::{Args, Parser, Subcommand};
use eure_fmt::unformat::{unformat, unformat_with_seed};
use std::fs;

#[derive(Parser)]
#[command(name = "eure", about = "EURE file utilities")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse and display EURE file syntax tree
    Inspect(Inspect),
    /// Unformat EURE file
    Unformat(Unformat),
}

#[derive(Args)]
struct Inspect {
    /// Path to EURE file to inspect
    file: String,
}

#[derive(Args)]
struct Unformat {
    /// Path to EURE file to unformat
    file: String,
    /// Seed for unformatting
    #[arg(short, long)]
    seed: Option<u64>,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Inspect(Inspect { file }) => {
            let contents = match fs::read_to_string(&file) {
                Ok(contents) => contents,
                Err(e) => {
                    eprintln!("Error reading file: {}", e);
                    return;
                }
            };

            let tree = eure_parol::parse(&contents).unwrap();
            let mut out = String::new();
            tree.inspect(&contents, &mut out).unwrap();
            println!("{}", out);
        }
        Commands::Unformat(Unformat { file, seed }) => {
            let contents = match fs::read_to_string(&file) {
                Ok(contents) => contents,
                Err(e) => {
                    eprintln!("Error reading file: {}", e);
                    return;
                }
            };
            let mut tree = eure_parol::parse(&contents).unwrap();
            if let Some(seed) = seed {
                unformat_with_seed(&mut tree, seed);
            } else {
                unformat(&mut tree);
            }
            let mut out = String::new();
            tree.write(&contents, &mut out).unwrap();
            println!("{}", out);
        }
    }
}
