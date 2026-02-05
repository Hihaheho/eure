use eure::FromEure;

mod external {
    pub struct Target {
        pub a: u32,
        pub b: u32,
    }
}

#[derive(FromEure)]
#[eure(proxy = "external::Target")]
struct Proxy {
    a: u32,
}

fn main() {}
