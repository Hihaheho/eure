use eure::IntoEure;

#[derive(IntoEure)]
struct Test {
    #[eure(flatten)]
    field: String,
}

fn main() {}
