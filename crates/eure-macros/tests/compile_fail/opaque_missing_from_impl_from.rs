use eure::FromEure;

mod external {
    pub struct Target;
}

#[derive(FromEure)]
#[eure(opaque = "external::Target")]
struct Proxy {
    a: u32,
}

fn main() {}
