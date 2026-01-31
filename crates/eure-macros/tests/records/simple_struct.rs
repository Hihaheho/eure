use eure::FromEure;

#[derive(Debug, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
struct User {
    name: String,
    age: i32,
}

#[derive(Debug, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
struct Unit;

#[derive(Debug, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
struct Point(i32, i32);

#[derive(Debug, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
struct Name(String);

#[test]
fn test_parse_named_struct() {
    use eure::eure;
    let doc = eure!({ name = "Alice", age = 30 });
    assert_eq!(
        doc.parse::<User>(doc.get_root_id()).unwrap(),
        User {
            name: "Alice".to_string(),
            age: 30
        }
    );
}

#[test]
fn test_parse_unit_struct() {
    use eure::eure;
    let doc = eure!({ = () });
    assert_eq!(doc.parse::<Unit>(doc.get_root_id()).unwrap(), Unit);
}

#[test]
fn test_parse_tuple_struct() {
    use eure::eure;
    let doc = eure!({ = (10, 20) });
    assert_eq!(
        doc.parse::<Point>(doc.get_root_id()).unwrap(),
        Point(10, 20)
    );
}

#[test]
fn test_parse_newtype_struct() {
    use eure::eure;
    let doc = eure!({ = "Bob" });
    assert_eq!(
        doc.parse::<Name>(doc.get_root_id()).unwrap(),
        Name("Bob".to_string())
    );
}

#[test]
fn test_parse_named_struct_unknown_field_error() {
    use eure::eure;
    let doc = eure!({ name = "Alice", age = 30, extra = "field" });
    let result = doc.parse::<User>(doc.get_root_id());
    assert!(result.is_err());
}
