use eure::{FromEure, IntoEure};

/// Mock external module simulating types we don't own
mod external {
    #[derive(Debug, Clone, PartialEq)]
    pub enum Status {
        Active,
        Inactive,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum Value {
        Text(String),
        Number(i32),
    }
}

/// Proxy definition for external::Status (unit variants)
#[derive(Debug, PartialEq, FromEure, IntoEure)]
#[eure(crate = ::eure::document, proxy = "external::Status")]
#[allow(dead_code)]
enum StatusDef {
    Active,
    Inactive,
}

/// Proxy definition for external::Value (newtype variants)
#[derive(Debug, PartialEq, FromEure, IntoEure)]
#[eure(crate = ::eure::document, proxy = "external::Value")]
#[allow(dead_code)]
enum ValueDef {
    Text(String),
    Number(i32),
}

#[test]
fn test_parse_proxy_unit_variant() {
    use eure::eure;
    let doc = eure!({ = "Active" });
    let status: external::Status = doc.parse_via::<StatusDef, _>(doc.get_root_id()).unwrap();
    assert_eq!(status, external::Status::Active);

    let doc = eure!({ = "Inactive" });
    let status: external::Status = doc.parse_via::<StatusDef, _>(doc.get_root_id()).unwrap();
    assert_eq!(status, external::Status::Inactive);
}

#[test]
fn test_parse_proxy_newtype_variant() {
    use eure::eure;
    let doc = eure!({ = "hello" });
    let value: external::Value = doc.parse_via::<ValueDef, _>(doc.get_root_id()).unwrap();
    assert_eq!(value, external::Value::Text("hello".to_string()));

    let doc = eure!({ = 42 });
    let value: external::Value = doc.parse_via::<ValueDef, _>(doc.get_root_id()).unwrap();
    assert_eq!(value, external::Value::Number(42));
}

#[test]
fn test_write_proxy_unit_variant() {
    use eure::document::constructor::DocumentConstructor;
    use eure::document::write::IntoEure;

    // Write and verify via roundtrip
    let mut c = DocumentConstructor::new();
    StatusDef::write(external::Status::Active, &mut c).unwrap();
    let doc = c.finish();
    let parsed: external::Status = doc.parse_via::<StatusDef, _>(doc.get_root_id()).unwrap();
    assert_eq!(parsed, external::Status::Active);

    let mut c = DocumentConstructor::new();
    StatusDef::write(external::Status::Inactive, &mut c).unwrap();
    let doc = c.finish();
    let parsed: external::Status = doc.parse_via::<StatusDef, _>(doc.get_root_id()).unwrap();
    assert_eq!(parsed, external::Status::Inactive);
}

#[test]
fn test_write_proxy_newtype_variant() {
    use eure::document::constructor::DocumentConstructor;
    use eure::document::write::IntoEure;

    // Write and verify via roundtrip
    let mut c = DocumentConstructor::new();
    ValueDef::write(external::Value::Text("world".to_string()), &mut c).unwrap();
    let doc = c.finish();
    let parsed: external::Value = doc.parse_via::<ValueDef, _>(doc.get_root_id()).unwrap();
    assert_eq!(parsed, external::Value::Text("world".to_string()));

    let mut c = DocumentConstructor::new();
    ValueDef::write(external::Value::Number(123), &mut c).unwrap();
    let doc = c.finish();
    let parsed: external::Value = doc.parse_via::<ValueDef, _>(doc.get_root_id()).unwrap();
    assert_eq!(parsed, external::Value::Number(123));
}

#[test]
fn test_roundtrip_proxy_unit_variant() {
    use eure::document::constructor::DocumentConstructor;
    use eure::document::write::IntoEure;

    let original = external::Status::Active;
    let mut c = DocumentConstructor::new();
    StatusDef::write(original.clone(), &mut c).unwrap();
    let doc = c.finish();
    let parsed: external::Status = doc.parse_via::<StatusDef, _>(doc.get_root_id()).unwrap();
    assert_eq!(parsed, original);
}

#[test]
fn test_roundtrip_proxy_newtype_variant() {
    use eure::document::constructor::DocumentConstructor;
    use eure::document::write::IntoEure;

    let original = external::Value::Text("test".to_string());
    let mut c = DocumentConstructor::new();
    ValueDef::write(original.clone(), &mut c).unwrap();
    let doc = c.finish();
    let parsed: external::Value = doc.parse_via::<ValueDef, _>(doc.get_root_id()).unwrap();
    assert_eq!(parsed, original);
}
