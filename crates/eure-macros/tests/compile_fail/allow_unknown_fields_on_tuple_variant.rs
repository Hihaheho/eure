use eure::FromEure;

#[derive(FromEure)]
enum Test {
    #[eure(allow_unknown_fields)]
    Tuple(String, i32),
}

fn main() {}
