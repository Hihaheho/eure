use eure::IntoEure;

struct JumpAt;
struct JumpAtProxy;

#[derive(IntoEure)]
enum Cmd {
    Jump(usize, #[eure(via = "JumpAtProxy")] JumpAt),
}

fn main() {}
