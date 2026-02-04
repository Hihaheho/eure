use eure::FromEure;

#[derive(FromEure)]
struct Test {
    #[eure(default, flatten_ext)]
    field: String,
}

fn main() {}
