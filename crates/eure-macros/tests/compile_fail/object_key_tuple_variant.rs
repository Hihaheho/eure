use eure::ObjectKey;

#[derive(ObjectKey)]
enum Bad {
    Ok,
    Tuple(i32),
}

fn main() {}
