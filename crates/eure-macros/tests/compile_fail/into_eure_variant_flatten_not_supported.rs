use eure::IntoEure;

#[derive(IntoEure)]
enum Test {
    Struct {
        #[eure(flatten)]
        field: String,
    },
}

fn main() {}
