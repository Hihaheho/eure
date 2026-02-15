use eure::ObjectKey;

#[derive(ObjectKey)]
enum Bad<T> {
    A,
    B,
}

fn main() {}
