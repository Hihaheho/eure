use eure::FromEure;

// Struct that allows unknown fields
#[derive(Debug, PartialEq, FromEure)]
#[eure(crate = ::eure::document, allow_unknown_fields)]
struct ConfigWithUnknown {
    name: String,
    version: i32,
}

// Default behavior: deny unknown fields
#[derive(Debug, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
struct ConfigStrict {
    name: String,
    version: i32,
}

#[test]
fn test_allow_unknown_fields_accepts_extra() {
    use eure::eure;
    let doc = eure!({
        name = "test",
        version = 1,
        extra_field = "ignored",
        another_extra = 42
    });

    let result = doc.parse::<ConfigWithUnknown>(doc.get_root_id());
    assert_eq!(
        result.unwrap(),
        ConfigWithUnknown {
            name: "test".to_string(),
            version: 1,
        }
    );
}

#[test]
fn test_allow_unknown_fields_works_without_extra() {
    use eure::eure;
    let doc = eure!({
        name = "test",
        version = 1
    });

    let result = doc.parse::<ConfigWithUnknown>(doc.get_root_id());
    assert_eq!(
        result.unwrap(),
        ConfigWithUnknown {
            name: "test".to_string(),
            version: 1,
        }
    );
}

#[test]
fn test_default_deny_unknown_fields() {
    use eure::eure;
    use eure::document::parse::ParseErrorKind;
    let doc = eure!({
        name = "test",
        version = 1,
        extra_field = "should fail"
    });

    let result = doc.parse::<ConfigStrict>(doc.get_root_id());
    assert!(matches!(
        result.unwrap_err().kind,
        ParseErrorKind::UnknownField(_)
    ));
}
