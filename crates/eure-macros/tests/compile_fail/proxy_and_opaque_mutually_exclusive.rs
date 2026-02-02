use eure::FromEure;

#[derive(FromEure)]
#[eure(proxy = "Foo", opaque = "Bar")]
struct Test {
    field: String,
}

fn main() {}
