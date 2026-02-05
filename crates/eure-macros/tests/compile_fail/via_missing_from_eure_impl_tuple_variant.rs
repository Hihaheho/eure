use eure::FromEure;

struct JumpAt;
struct JumpAtProxy;

#[derive(FromEure)]
enum Cmd {
    Jump(usize, #[eure(via = "JumpAtProxy")] JumpAt),
}

fn main() {}
