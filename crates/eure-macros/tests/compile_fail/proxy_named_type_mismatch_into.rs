use eure::IntoEure;

mod external {
    pub struct Target {
        pub a: u32,
    }
}

#[derive(IntoEure)]
#[eure(proxy = "external::Target")]
struct Proxy {
    a: u64,
}

fn main() {}
