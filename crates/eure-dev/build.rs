use std::fs;
use std::path::Path;

fn main() {
    // Create tailwind.css if it doesn't exist
    let tailwind_css_path = Path::new("assets/tailwind.css");
    if !tailwind_css_path.exists() {
        fs::write(tailwind_css_path, "").expect("Failed to create tailwind.css");
    }

    // Tell Cargo to rerun this build script if tailwind.css changes
    println!("cargo:rerun-if-changed=assets/tailwind.css");
}