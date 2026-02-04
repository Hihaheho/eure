use eure::document::EureDocument;
use eure_macros::IntoEure;

#[derive(IntoEure)]
struct Config {
    name: String,
}

fn main() {
    let _ = EureDocument::new();
}
