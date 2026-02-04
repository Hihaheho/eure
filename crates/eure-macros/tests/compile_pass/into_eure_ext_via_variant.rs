use eure::document::constructor::DocumentConstructor;
use eure::document::write::{IntoEure, WriteError};
use eure_macros::IntoEure;

struct DurationDef;

impl IntoEure<std::time::Duration> for DurationDef {
    fn write(
        _value: std::time::Duration,
        _c: &mut DocumentConstructor,
    ) -> Result<(), WriteError> {
        Ok(())
    }
}

#[derive(IntoEure)]
enum Event {
    Timed {
        name: String,
        #[eure(ext, via = "DurationDef")]
        timeout: std::time::Duration,
    },
}

fn main() {
    let _ = Event::Timed {
        name: String::new(),
        timeout: std::time::Duration::from_secs(0),
    };
}
