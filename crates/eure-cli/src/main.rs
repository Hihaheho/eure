use clap::{Args, Parser, Subcommand};
use eure_fmt::unformat::{unformat, unformat_with_seed};
use std::fs;
use std::io::{self, Read};

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
    /// Format EURE file
    Fmt(Fmt),
}

#[derive(Args)]
struct Inspect {
    /// Path to EURE file to inspect
    file: String,
}

#[derive(Args)]
struct Unformat {
    /// Path to EURE file to unformat (use '-' for stdin)
    file: Option<String>,
    /// Seed for unformatting
    #[arg(short, long)]
    seed: Option<u64>,
}

#[derive(Args)]
struct Fmt {
    /// Path to EURE file to format (use '-' for stdin)
    file: Option<String>,
    /// Check mode - exit with non-zero status if formatting is needed
    #[arg(short, long)]
    check: bool,
    /// Indent width (default: 2)
    #[arg(short, long, default_value = "2")]
    indent_width: usize,
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
            // Read input from file or stdin
            let contents = match file.as_deref() {
                None | Some("-") => {
                    // Read from stdin
                    let mut buffer = String::new();
                    if let Err(e) = io::stdin().read_to_string(&mut buffer) {
                        eprintln!("Error reading from stdin: {}", e);
                        std::process::exit(1);
                    }
                    buffer
                }
                Some(path) => match fs::read_to_string(path) {
                    Ok(contents) => contents,
                    Err(e) => {
                        eprintln!("Error reading file: {}", e);
                        std::process::exit(1);
                    }
                },
            };

            let mut tree = match eure_parol::parse(&contents) {
                Ok(tree) => tree,
                Err(e) => {
                    eprintln!("Parse error: {:?}", e);
                    std::process::exit(1);
                }
            };

            if let Some(seed) = seed {
                unformat_with_seed(&mut tree, seed);
            } else {
                unformat(&mut tree);
            }

            let mut out = String::new();
            if let Err(e) = tree.write(&contents, &mut out) {
                eprintln!("Error writing output: {}", e);
                std::process::exit(1);
            }
            print!("{}", out);
        }
        Commands::Fmt(Fmt {
            file,
            check,
            indent_width,
        }) => {
            // Read input from file or stdin
            let contents = match file.as_deref() {
                None | Some("-") => {
                    // Read from stdin
                    let mut buffer = String::new();
                    if let Err(e) = io::stdin().read_to_string(&mut buffer) {
                        eprintln!("Error reading from stdin: {}", e);
                        std::process::exit(1);
                    }
                    buffer
                }
                Some(path) => match fs::read_to_string(path) {
                    Ok(contents) => contents,
                    Err(e) => {
                        eprintln!("Error reading file: {}", e);
                        std::process::exit(1);
                    }
                },
            };

            // Parse the input
            let mut tree = match eure_parol::parse(&contents) {
                Ok(tree) => tree,
                Err(e) => {
                    eprintln!("Parse error: {:?}", e);
                    std::process::exit(1);
                }
            };

            let config = eure_fmt::FmtConfig::new(indent_width);

            if check {
                // Check mode - just verify if formatting is needed
                match eure_fmt::check_formatting_with_config(&contents, &tree, config) {
                    Ok(errors) => {
                        if !errors.is_empty() {
                            eprintln!("File needs formatting ({} issues found)", errors.len());
                            std::process::exit(1);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error checking formatting: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                // Format mode - apply formatting
                if let Err(e) = eure_fmt::fmt_with_config(&contents, &mut tree, config) {
                    eprintln!("Error formatting: {}", e);
                    std::process::exit(1);
                }

                let mut out = String::new();
                if let Err(e) = tree.write(&contents, &mut out) {
                    eprintln!("Error writing output: {}", e);
                    std::process::exit(1);
                }
                print!("{}", out);
            }
        }
    }
}
