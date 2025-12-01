mod commands {
    automod::dir!(pub "src/commands");
}
mod util;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "eure", about = "Eure file utilities")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse and display Eure file syntax tree
    Inspect(commands::inspect::Args),
    /// Unformat Eure file
    Unformat(commands::unformat::Args),
    /// Format Eure file
    Fmt(commands::fmt::Args),
    /// Convert Eure to JSON
    ToJson(commands::to_json::Args),
    /// Convert JSON to Eure
    FromJson(commands::from_json::Args),
    /// Syntax highlight Eure file with colors
    Highlight(commands::highlight::Args),
    /// Export Eure file as HTML with syntax highlighting
    Html(commands::html::Args),
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Inspect(args) => commands::inspect::run(args),
        Commands::Unformat(args) => commands::unformat::run(args),
        Commands::Fmt(args) => commands::fmt::run(args),
        Commands::ToJson(args) => commands::to_json::run(args),
        Commands::FromJson(args) => commands::from_json::run(args),
        Commands::Highlight(args) => commands::highlight::run(args),
        Commands::Html(args) => commands::html::run(args),
    }
}
