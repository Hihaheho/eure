use eure::ObjectKey;

#[derive(ObjectKey)]
enum Bad {
    #[eure(unknown_option = "value")]
    A,
}

fn main() {}
