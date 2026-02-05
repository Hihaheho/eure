use eure::IntoEure;

mod external {
    pub struct Target(pub u32, pub u32);
}

#[derive(IntoEure)]
#[eure(proxy = "external::Target")]
struct Proxy(pub u32);

fn main() {}
