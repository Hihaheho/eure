use eure::ParseDocument;

// Enum with variant that allows unknown fields
#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
enum Message {
    Text(String),
    #[eure(allow_unknown_fields)]
    Data { id: i32, value: String },
    Strict { id: i32, value: String },
}

#[test]
fn test_variant_allow_unknown_fields_accepts_extra() {
    use eure::eure;
    let doc = eure!({
        %variant = "Data",
        id = 1,
        value = "test",
        extra = "ignored"
    });

    let result = doc.parse::<Message>(doc.get_root_id());
    assert_eq!(
        result.unwrap(),
        Message::Data {
            id: 1,
            value: "test".to_string(),
        }
    );
}

#[test]
fn test_variant_default_deny_unknown_fields() {
    use eure::eure;
    use eure::document::parse::ParseErrorKind;
    let doc = eure!({
        %variant = "Strict",
        id = 1,
        value = "test",
        extra = "should fail"
    });

    let result = doc.parse::<Message>(doc.get_root_id());
    assert!(matches!(
        result.unwrap_err().kind,
        ParseErrorKind::UnknownField(_)
    ));
}
