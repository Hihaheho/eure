use eure::ObjectKey;

#[derive(ObjectKey)]
enum Bad {
    #[eure(rename = "same")]
    First,
    #[eure(rename = "same")]
    Second,
}

fn main() {}
