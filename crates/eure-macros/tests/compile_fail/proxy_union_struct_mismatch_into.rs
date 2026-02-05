use eure::IntoEure;

mod external {
    pub enum Target {
        S { a: u32 },
    }
}

#[derive(IntoEure)]
#[eure(proxy = "external::Target")]
enum Proxy {
    S { a: u64 },
}

fn main() {}
