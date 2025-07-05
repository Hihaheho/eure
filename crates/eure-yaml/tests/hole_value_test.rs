use eure_value::value::{Value, Map, Array, KeyCmpValue};
use eure_yaml::value_to_yaml;
use ahash::AHashMap;

#[test]
fn test_hole_to_yaml_error() {
    // Test that holes cannot be converted to YAML
    let hole = Value::Hole;
    let result = value_to_yaml(&hole);
    
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Cannot convert hole value (!) to YAML"));
}

#[test]
fn test_hole_in_object_to_yaml() {
    let mut map = AHashMap::new();
    map.insert(KeyCmpValue::String("name".to_string()), Value::String("John".to_string()));
    map.insert(KeyCmpValue::String("age".to_string()), Value::Hole);
    
    let obj = Value::Map(Map(map));
    let result = value_to_yaml(&obj);
    
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Cannot convert hole value (!) to YAML"));
}

#[test]
fn test_hole_in_array_to_yaml() {
    let arr = Value::Array(Array(vec![
        Value::String("first".to_string()),
        Value::Hole,
        Value::String("third".to_string()),
    ]));
    
    let result = value_to_yaml(&arr);
    
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Cannot convert hole value (!) to YAML"));
}

#[test]
fn test_hole_as_map_key() {
    let mut map = AHashMap::new();
    map.insert(KeyCmpValue::Hole, Value::String("value".to_string()));
    
    let obj = Value::Map(Map(map));
    let result = value_to_yaml(&obj);
    
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Cannot use hole value (!) as YAML map key"));
}

#[test]
fn test_nested_hole_to_yaml() {
    let mut inner_map = AHashMap::new();
    inner_map.insert(KeyCmpValue::String("street".to_string()), Value::Hole);
    inner_map.insert(KeyCmpValue::String("city".to_string()), Value::String("New York".to_string()));
    
    let mut outer_map = AHashMap::new();
    outer_map.insert(KeyCmpValue::String("address".to_string()), Value::Map(Map(inner_map)));
    
    let obj = Value::Map(Map(outer_map));
    let result = value_to_yaml(&obj);
    
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Cannot convert hole value (!) to YAML"));
}