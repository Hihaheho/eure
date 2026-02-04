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

/// Opaque definition for external::Status (unit variants)
/// With opaque, we provide From conversions both ways
#[derive(Debug, PartialEq, FromEure, IntoEure)]
#[eure(crate = ::eure::document, opaque = "external::Status")]
enum StatusDef {
    Active,
    Inactive,
}

impl From<external::Status> for StatusDef {
    fn from(status: external::Status) -> Self {
        match status {
            external::Status::Active => StatusDef::Active,
            external::Status::Inactive => StatusDef::Inactive,
        }
    }
}

impl From<StatusDef> for external::Status {
    fn from(def: StatusDef) -> Self {
        match def {
            StatusDef::Active => external::Status::Active,
            StatusDef::Inactive => external::Status::Inactive,
        }
    }
}

/// Opaque definition for external::Value (newtype variants)
#[derive(Debug, PartialEq, FromEure, IntoEure)]
#[eure(crate = ::eure::document, opaque = "external::Value")]
enum ValueDef {
    Text(String),
    Number(i32),
}

impl From<external::Value> for ValueDef {
    fn from(value: external::Value) -> Self {
        match value {
            external::Value::Text(s) => ValueDef::Text(s),
            external::Value::Number(n) => ValueDef::Number(n),
        }
    }
}

impl From<ValueDef> for external::Value {
    fn from(def: ValueDef) -> Self {
        match def {
            ValueDef::Text(s) => external::Value::Text(s),
            ValueDef::Number(n) => external::Value::Number(n),
        }
    }
}

#[test]
fn test_parse_opaque_unit_variant() {
    use eure::eure;
    let doc = eure!({ = "Active" });
    let status: external::Status = doc.parse_via::<StatusDef, _>(doc.get_root_id()).unwrap();
    assert_eq!(status, external::Status::Active);

    let doc = eure!({ = "Inactive" });
    let status: external::Status = doc.parse_via::<StatusDef, _>(doc.get_root_id()).unwrap();
    assert_eq!(status, external::Status::Inactive);
}

#[test]
fn test_parse_opaque_newtype_variant() {
    use eure::eure;
    let doc = eure!({ = "hello" });
    let value: external::Value = doc.parse_via::<ValueDef, _>(doc.get_root_id()).unwrap();
    assert_eq!(value, external::Value::Text("hello".to_string()));

    let doc = eure!({ = 42 });
    let value: external::Value = doc.parse_via::<ValueDef, _>(doc.get_root_id()).unwrap();
    assert_eq!(value, external::Value::Number(42));
}

#[test]
fn test_write_opaque_unit_variant() {
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
fn test_write_opaque_newtype_variant() {
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
fn test_roundtrip_opaque_unit_variant() {
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
fn test_roundtrip_opaque_newtype_variant() {
    use eure::document::constructor::DocumentConstructor;
    use eure::document::write::IntoEure;

    let original = external::Value::Text("test".to_string());
    let mut c = DocumentConstructor::new();
    ValueDef::write(original.clone(), &mut c).unwrap();
    let doc = c.finish();
    let parsed: external::Value = doc.parse_via::<ValueDef, _>(doc.get_root_id()).unwrap();
    assert_eq!(parsed, original);
}
