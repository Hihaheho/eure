use eure::FromEure;

#[derive(FromEure)]
struct Test {
    #[eure(ext, flatten_ext)]
    field: String,
}

fn main() {}
