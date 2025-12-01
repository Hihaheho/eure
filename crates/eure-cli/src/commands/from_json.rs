use crate::util::VariantFormat;

#[derive(clap::Args)]
pub struct Args {
    /// Path to JSON file to convert (use - for stdin)
    pub file: String,
    /// Variant representation format
    #[arg(short = 'v', long, value_enum, default_value = "external")]
    pub variant: VariantFormat,
    /// Tag field name for internal/adjacent representations
    #[arg(short = 't', long, default_value = "type")]
    pub tag: String,
    /// Content field name for adjacent representation
    #[arg(short = 'c', long, default_value = "content")]
    pub content: String,
}

pub fn run(_args: Args) {
    eprintln!("Error: JSON to Eure conversion is not yet implemented.");
    eprintln!("The reverse conversion API is currently under development.");
    eprintln!("You can only convert Eure → JSON using `eure to-json`.");
    std::process::exit(1);
}
