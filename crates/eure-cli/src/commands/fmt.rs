#[derive(clap::Args)]
pub struct Args {
    /// Path to Eure file to format (use '-' for stdin)
    pub file: Option<String>,
    /// Check mode - exit with non-zero status if formatting is needed
    #[arg(short, long)]
    pub check: bool,
    /// Indent width (default: 2)
    #[arg(short, long, default_value = "2")]
    pub indent_width: usize,
}

pub fn run(_args: Args) {
    eprintln!("Error: Formatting is not yet implemented.");
    eprintln!("The formatter API is currently under development.");
    eprintln!("Use `eure unformat` to remove all formatting instead.");
    std::process::exit(1);
}
