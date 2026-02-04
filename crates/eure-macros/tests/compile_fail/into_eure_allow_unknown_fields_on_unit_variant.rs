use eure::IntoEure;

#[derive(IntoEure)]
enum Test {
    #[eure(allow_unknown_fields)]
    Unit,
}

fn main() {}
