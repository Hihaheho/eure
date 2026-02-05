use eure::IntoEure;

mod external {
    pub struct Target;
}

#[derive(IntoEure)]
#[eure(opaque = "external::Target")]
struct Proxy {
    a: u32,
}

fn main() {}
