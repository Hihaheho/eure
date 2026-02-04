use eure::IntoEure;

#[derive(IntoEure)]
struct Test {
    #[eure(flatten_ext)]
    field: String,
}

fn main() {}
