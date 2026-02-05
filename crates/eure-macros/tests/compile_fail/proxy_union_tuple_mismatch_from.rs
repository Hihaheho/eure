use eure::FromEure;

mod external {
    pub enum Target {
        T(u32, u32),
    }
}

#[derive(FromEure)]
#[eure(proxy = "external::Target")]
enum Proxy {
    T(u32, u64),
}

fn main() {}
