use eure::ObjectKey;

#[derive(ObjectKey)]
enum Bad {
    Ok,
    Named { field: i32 },
}

fn main() {}
