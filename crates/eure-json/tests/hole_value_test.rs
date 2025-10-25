use ahash::AHashMap;
use eure_json::value_to_json;
use eure_value::value::{Array, KeyCmpValue, Map, Value};

#[test]
fn test_hole_to_json_error() {
    // Test that holes cannot be converted to JSON
    let hole = Value::Hole;
    let result = value_to_json(&hole);

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Cannot convert hole value (!) to JSON")
    );
}

#[test]
fn test_hole_in_object_to_json() {
    let mut map = AHashMap::new();
    map.insert(
        KeyCmpValue::String("name".to_string()),
        Value::String("John".to_string()),
    );
    map.insert(KeyCmpValue::String("age".to_string()), Value::Hole);

    let obj = Value::Map(Map(map));
    let result = value_to_json(&obj);

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Cannot convert hole value (!) to JSON")
    );
}

#[test]
fn test_hole_in_array_to_json() {
    let arr = Value::Array(Array(vec![
        Value::String("first".to_string()),
        Value::Hole,
        Value::String("third".to_string()),
    ]));

    let result = value_to_json(&arr);

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Cannot convert hole value (!) to JSON")
    );
}

#[test]
fn test_hole_as_map_key() {
    let mut map = AHashMap::new();
    map.insert(KeyCmpValue::Hole, Value::String("value".to_string()));

    let obj = Value::Map(Map(map));
    let result = value_to_json(&obj);

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Hole keys cannot be converted to JSON")
    );
}

#[test]
fn test_nested_hole_to_json() {
    let mut inner_map = AHashMap::new();
    inner_map.insert(KeyCmpValue::String("street".to_string()), Value::Hole);
    inner_map.insert(
        KeyCmpValue::String("city".to_string()),
        Value::String("New York".to_string()),
    );

    let mut outer_map = AHashMap::new();
    outer_map.insert(
        KeyCmpValue::String("address".to_string()),
        Value::Map(Map(inner_map)),
    );

    let obj = Value::Map(Map(outer_map));
    let result = value_to_json(&obj);

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Cannot convert hole value (!) to JSON")
    );
}
