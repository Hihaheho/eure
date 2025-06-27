#[cfg(test)]
mod tests {
    use crate::format::*;
    use eure_value::value::{Array, KeyCmpValue, Map, Value};

    #[test]
    fn test_format_simple_value() {
        // Test integer
        let value = Value::I64(42);
        let result = format_eure(&value);
        println!("Integer: '{result}'");
        assert!(!result.is_empty(), "Result should not be empty");
        assert!(result.contains("value"), "Should contain 'value' key");
        assert!(result.contains("="), "Should contain '=' sign");
        assert!(result.contains("42"), "Should contain the number");

        // Test string
        let value = Value::String("hello".to_string());
        let result = format_eure(&value);
        println!("String: {result}");

        // Test boolean
        let value = Value::Bool(true);
        let result = format_eure(&value);
        println!("Boolean: {result}");

        // Test null
        let value = Value::Null;
        let result = format_eure(&value);
        println!("Null: {result}");

        // Test empty array
        let value = Value::Array(Array(vec![]));
        let result = format_eure(&value);
        println!("Empty array: {result}");

        // Test array with values
        let value = Value::Array(Array(vec![Value::I64(1), Value::I64(2), Value::I64(3)]));
        let result = format_eure(&value);
        println!("Array: {result}");

        // Test object
        let mut map = ahash::AHashMap::new();
        map.insert(
            KeyCmpValue::String("name".to_string()),
            Value::String("Alice".to_string()),
        );
        map.insert(KeyCmpValue::String("age".to_string()), Value::I64(25));
        let value = Value::Map(Map(map));
        let result = format_eure(&value);
        println!("Object: {result}");
        assert!(result.contains("name"), "Should contain 'name' key");
        assert!(result.contains("Alice"), "Should contain 'Alice' value");
    }
}
