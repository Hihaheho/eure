use eure::IntoEure;

mod external {
    pub enum Target {
        A(u32),
    }
}

#[derive(IntoEure)]
#[eure(proxy = "external::Target")]
enum Proxy {
    A(u64),
}

fn main() {}
