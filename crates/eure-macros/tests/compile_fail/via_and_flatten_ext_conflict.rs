use eure::FromEure;

#[derive(FromEure)]
struct Test {
    #[eure(via = "SomeType", flatten_ext)]
    field: String,
}

fn main() {}
