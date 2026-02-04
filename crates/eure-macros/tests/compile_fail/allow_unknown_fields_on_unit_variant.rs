use eure::FromEure;

#[derive(FromEure)]
enum Test {
    #[eure(allow_unknown_fields)]
    Unit,
}

fn main() {}
