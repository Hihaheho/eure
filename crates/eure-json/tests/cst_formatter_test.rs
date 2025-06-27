use eure_value::value::{Value, Map, Array, KeyCmpValue, Code, TypedString, Variant};
use eure_json::format_eure;
use ahash::AHashMap;

#[test]
fn test_cst_formatter_comprehensive() {
    // Test simple values
    let test_cases = vec![
        (Value::Null, "value = null"),
        (Value::Bool(true), "value = true"),
        (Value::Bool(false), "value = false"),
        (Value::I64(-42), "value = -42"),
        (Value::U64(42), "value = 42"),
        (Value::String("hello".to_string()), "value = \"hello\""),
    ];
    
    for (value, expected) in test_cases {
        let result = format_eure(&value);
        assert_eq!(result.trim(), expected);
    }
    
    // Test arrays
    let array = Value::Array(Array(vec![
        Value::I64(1),
        Value::I64(2),
        Value::I64(3),
    ]));
    let result = format_eure(&array);
    assert_eq!(result.trim(), "value = [1, 2, 3]");
    
    // Test empty array
    let empty_array = Value::Array(Array(vec![]));
    let result = format_eure(&empty_array);
    assert_eq!(result.trim(), "value = []");
    
    // Test nested array
    let nested = Value::Array(Array(vec![
        Value::Array(Array(vec![Value::I64(1), Value::I64(2)])),
        Value::Array(Array(vec![Value::I64(3), Value::I64(4)])),
    ]));
    let result = format_eure(&nested);
    assert_eq!(result.trim(), "value = [[1, 2], [3, 4]]");
    
    // Test object
    let mut map = AHashMap::new();
    map.insert(KeyCmpValue::String("x".to_string()), Value::I64(1));
    map.insert(KeyCmpValue::String("y".to_string()), Value::I64(2));
    let object = Value::Map(Map(map));
    let result = format_eure(&object);
    // Note: HashMap order may vary
    assert!(result.contains("x = 1"));
    assert!(result.contains("y = 2"));
    
    // Test empty object
    let empty_map = AHashMap::new();
    let empty_object = Value::Map(Map(empty_map));
    let result = format_eure(&empty_object);
    assert_eq!(result.trim(), "value = {}");
    
    // Test code block
    let code = Value::Code(Code {
        language: "rust".to_string(),
        content: "fn main() {\n    println!(\"Hello\");\n}".to_string(),
    });
    let result = format_eure(&code);
    assert!(result.contains("```rust"));
    assert!(result.contains("fn main()"));
    
    // Test typed string
    let typed_str = Value::TypedString(TypedString {
        type_name: "Email".to_string(),
        value: "test@example.com".to_string(),
    });
    let result = format_eure(&typed_str);
    assert_eq!(result.trim(), r#"value = Email"test@example.com""#);
    
    // Test variant
    let variant = Value::Variant(Variant {
        tag: "Success".to_string(),
        content: Box::new(Value::String("OK".to_string())),
    });
    let result = format_eure(&variant);
    println!("Variant result: '{}'", result);
    assert!(result.contains("value = {")); // It's wrapped in a "value" binding
    assert!(result.contains("$variant")); // Contains variant key
    assert!(result.contains("Success")); // Contains the tag
    assert!(result.contains("content")); // Contains content key
}

#[test]
fn test_cst_formatter_special_keys() {
    let mut map = AHashMap::new();
    
    // Test string key that needs escaping
    map.insert(
        KeyCmpValue::String("key with spaces".to_string()),
        Value::I64(1),
    );
    
    // Test integer keys
    map.insert(KeyCmpValue::I64(42), Value::String("forty-two".to_string()));
    map.insert(KeyCmpValue::U64(100), Value::String("hundred".to_string()));
    
    let value = Value::Map(Map(map));
    let result = format_eure(&value);
    
    assert!(result.contains("\"key with spaces\" = 1"));
    assert!(result.contains("42 = \"forty-two\""));
    assert!(result.contains("100 = \"hundred\""));
}

#[test]
fn test_cst_formatter_complex_structure() {
    let mut inner_map = AHashMap::new();
    inner_map.insert(KeyCmpValue::String("nested".to_string()), Value::Bool(true));
    
    let mut outer_map = AHashMap::new();
    outer_map.insert(
        KeyCmpValue::String("array".to_string()),
        Value::Array(Array(vec![Value::I64(1), Value::I64(2), Value::I64(3)])),
    );
    outer_map.insert(
        KeyCmpValue::String("object".to_string()),
        Value::Map(Map(inner_map)),
    );
    outer_map.insert(
        KeyCmpValue::String("string".to_string()),
        Value::String("hello world".to_string()),
    );
    
    let value = Value::Map(Map(outer_map));
    let result = format_eure(&value);
    
    // Check that all parts are present
    assert!(result.contains("array = [1, 2, 3]"));
    assert!(result.contains("object = {nested = true}"));
    assert!(result.contains("string = \"hello world\""));
}