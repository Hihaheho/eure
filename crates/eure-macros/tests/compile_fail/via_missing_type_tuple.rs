use eure::FromEure;

struct JumpAt;

#[derive(FromEure)]
struct Steps(usize, #[eure(via = "MissingProxy")] JumpAt);

fn main() {}
