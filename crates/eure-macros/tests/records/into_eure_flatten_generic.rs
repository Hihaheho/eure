use eure::{FromEure, IntoEure};

#[derive(Debug, PartialEq, FromEure, IntoEure)]
#[eure(crate = ::eure::document)]
struct Address {
    city: String,
    country: String,
}

#[derive(Debug, PartialEq, FromEure, IntoEure)]
#[eure(crate = ::eure::document)]
struct Envelope<T> {
    id: i32,
    #[eure(flatten)]
    payload: T,
}

#[test]
fn test_into_eure_flatten_generic_roundtrip() {
    use eure::document::constructor::DocumentConstructor;

    let value = Envelope {
        id: 1,
        payload: Address {
            city: "Tokyo".to_string(),
            country: "Japan".to_string(),
        },
    };

    let mut c = DocumentConstructor::new();
    c.write(value).unwrap();
    let doc = c.finish();

    let parsed = doc.parse::<Envelope<Address>>(doc.get_root_id()).unwrap();
    assert_eq!(
        parsed,
        Envelope {
            id: 1,
            payload: Address {
                city: "Tokyo".to_string(),
                country: "Japan".to_string(),
            },
        }
    );
}
