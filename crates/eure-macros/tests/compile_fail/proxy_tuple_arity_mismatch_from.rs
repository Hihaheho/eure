use eure::FromEure;

mod external {
    pub struct Target(pub u32, pub u32);
}

#[derive(FromEure)]
#[eure(proxy = "external::Target")]
struct Proxy(pub u32);

fn main() {}
