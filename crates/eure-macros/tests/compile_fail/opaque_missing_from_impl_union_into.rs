use eure::IntoEure;

mod external {
    pub struct Target;
}

#[derive(IntoEure)]
#[eure(opaque = "external::Target")]
enum Proxy {
    A,
}

fn main() {}
