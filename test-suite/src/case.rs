use std::path::PathBuf;

use eure::value::code::Code;

pub struct Case {
    pub path: PathBuf,
    pub input_eure: Option<Code>,
    pub normalized: Option<Code>,
    pub output_json: Option<Code>,
}

impl Case {
    pub(crate) fn run(&self) -> eros::Result<()> {
        todo!()
    }
}
