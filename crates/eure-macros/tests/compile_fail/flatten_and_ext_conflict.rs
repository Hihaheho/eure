use eure::FromEure;

#[derive(FromEure)]
struct Test {
    #[eure(flatten, ext)]
    field: String,
}

fn main() {}
