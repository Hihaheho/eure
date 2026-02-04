use eure::FromEure;

#[derive(FromEure)]
struct Test {
    #[eure(flatten, flatten_ext)]
    field: String,
}

fn main() {}
