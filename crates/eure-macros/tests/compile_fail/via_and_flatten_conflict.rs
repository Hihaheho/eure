use eure::FromEure;

#[derive(FromEure)]
struct Test {
    #[eure(via = "SomeType", flatten)]
    field: String,
}

fn main() {}
