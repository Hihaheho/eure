use eure::IntoEure;

mod external {
    pub struct Target;
}

#[derive(IntoEure)]
#[eure(crate = ::eure::document, opaque = "external::Target")]
struct Proxy {
    a: u32,
}

impl From<external::Target> for Proxy {
    fn from(_value: external::Target) -> Self {
        Self { a: 7 }
    }
}

#[test]
fn test_into_eure_opaque_named_struct() {
    use eure::document::constructor::DocumentConstructor;

    let mut c = DocumentConstructor::new();
    c.write_via::<Proxy, _>(external::Target).unwrap();

    assert_eq!(c.finish(), eure::eure!({ a = 7 }));
}
