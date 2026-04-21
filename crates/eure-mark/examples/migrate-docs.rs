use std::path::PathBuf;

fn main() {
    let docs_root = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("docs"));

    let generated =
        eure_mark::migrate_markdown_guides_in_place(&docs_root).unwrap_or_else(|error| {
            panic!("failed to migrate docs at {}: {error}", docs_root.display())
        });

    for path in generated {
        println!("{}", path.display());
    }
}
