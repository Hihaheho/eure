use eure::IntoEure;

#[derive(IntoEure)]
enum Test {
    Struct {
        #[eure(flatten_ext)]
        field: String,
    },
}

fn main() {}
