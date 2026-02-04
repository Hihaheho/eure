use eure::FromEure;

#[derive(FromEure)]
#[eure(parse_ext)]
struct Test {
    #[eure(flatten)]
    field: String,
}

fn main() {}
