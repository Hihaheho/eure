use eure::document::parse::ParseObjectKey;
use eure::document::value::ObjectKey;
use eure::ObjectKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, ObjectKey)]
#[eure(crate = ::eure::document)]
enum Direction {
    North,
    South,
    East,
    West,
}

#[test]
fn test_parse_from_string_key() {
    let key = ObjectKey::String("North".to_string());
    let dir: Direction = ParseObjectKey::from_object_key(&key).unwrap();
    assert_eq!(dir, Direction::North);
}

#[test]
fn test_parse_from_extension_ident() {
    let ident: eure::document::identifier::Identifier = "South".parse().unwrap();
    let dir: Direction = ParseObjectKey::from_extension_ident(&ident).unwrap();
    assert_eq!(dir, Direction::South);
}

#[test]
fn test_into_object_key() {
    let key: ObjectKey = Direction::East.into();
    assert_eq!(key, ObjectKey::String("East".to_string()));
}

#[test]
fn test_roundtrip() {
    for (variant, name) in [
        (Direction::North, "North"),
        (Direction::South, "South"),
        (Direction::East, "East"),
        (Direction::West, "West"),
    ] {
        let key: ObjectKey = variant.into();
        assert_eq!(key, ObjectKey::String(name.to_string()));

        let parsed: Direction = ParseObjectKey::from_object_key(&key).unwrap();
        assert_eq!(parsed, variant);
    }
}

#[test]
fn test_unknown_variant_error() {
    let key = ObjectKey::String("Unknown".to_string());
    let result: Result<Direction, _> = ParseObjectKey::from_object_key(&key);
    assert!(result.is_err());
}

#[test]
fn test_non_string_key_error() {
    let key = ObjectKey::Number(42.into());
    let result: Result<Direction, _> = ParseObjectKey::from_object_key(&key);
    assert!(result.is_err());
}
