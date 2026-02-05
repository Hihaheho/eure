use eure::FromEure;

mod external {
    pub struct Target;
}

#[derive(FromEure)]
#[eure(opaque = "external::Target")]
enum Proxy {
    A,
}

fn main() {}
