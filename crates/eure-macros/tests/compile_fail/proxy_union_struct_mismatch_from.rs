use eure::FromEure;

mod external {
    pub enum Target {
        S { a: u32 },
    }
}

#[derive(FromEure)]
#[eure(proxy = "external::Target")]
enum Proxy {
    S { a: u64 },
}

fn main() {}
