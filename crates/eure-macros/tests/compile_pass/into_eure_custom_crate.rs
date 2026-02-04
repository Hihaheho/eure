use eure_document::document::EureDocument;
use eure_macros::IntoEure;

#[derive(IntoEure)]
#[eure(crate = ::eure_document)]
struct Config {
    name: String,
}

#[derive(IntoEure)]
#[eure(crate = ::eure_document)]
enum Status {
    Active,
    Inactive,
}

fn main() {
    let _ = EureDocument::new();
}
