use eure::IntoEure;

mod external {
    pub struct Target {
        pub b: u32,
    }
}

#[derive(IntoEure)]
#[eure(proxy = "external::Target")]
struct Proxy {
    a: u32,
}

fn main() {}
