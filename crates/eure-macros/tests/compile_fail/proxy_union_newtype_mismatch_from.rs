use eure::FromEure;

mod external {
    pub enum Target {
        A(u32),
    }
}

#[derive(FromEure)]
#[eure(proxy = "external::Target")]
enum Proxy {
    A(u64),
}

fn main() {}
