use eure::document::parse::ParseObjectKey;
use eure::document::value::ObjectKey;
use eure::ObjectKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, ObjectKey)]
#[eure(crate = ::eure::document, rename_all = "snake_case")]
enum Priority {
    #[eure(rename = "P0")]
    Critical,
    High,
    Low,
}

#[test]
fn test_rename_overrides_rename_all() {
    // Critical is renamed to "P0", not "critical"
    let key = ObjectKey::String("P0".to_string());
    let p: Priority = ParseObjectKey::from_object_key(&key).unwrap();
    assert_eq!(p, Priority::Critical);

    let key: ObjectKey = Priority::Critical.into();
    assert_eq!(key, ObjectKey::String("P0".to_string()));
}

#[test]
fn test_rename_all_still_applies_to_others() {
    let key = ObjectKey::String("high".to_string());
    let p: Priority = ParseObjectKey::from_object_key(&key).unwrap();
    assert_eq!(p, Priority::High);
}

#[test]
fn test_original_name_of_renamed_variant_does_not_match() {
    let key = ObjectKey::String("Critical".to_string());
    let result: Result<Priority, _> = ParseObjectKey::from_object_key(&key);
    assert!(result.is_err());

    let key = ObjectKey::String("critical".to_string());
    let result: Result<Priority, _> = ParseObjectKey::from_object_key(&key);
    assert!(result.is_err());
}
