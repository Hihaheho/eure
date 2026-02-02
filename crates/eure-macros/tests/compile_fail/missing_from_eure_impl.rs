use eure::FromEure;

struct NoFromEure;

#[derive(FromEure)]
struct MyStruct {
    field: NoFromEure,
}

fn main() {}
