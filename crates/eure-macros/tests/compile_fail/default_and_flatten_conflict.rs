use eure::FromEure;

#[derive(FromEure)]
struct Test {
    #[eure(default, flatten)]
    field: String,
}

fn main() {}
