use serde::{Deserialize, Serialize};
use serde_eure::{from_str, from_value, to_string, to_value};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Person {
    name: String,
    age: u32,
    active: bool,
}

#[test]
fn test_struct_serialization() {
    let person = Person {
        name: "Alice".to_string(),
        age: 30,
        active: true,
    };

    // Test serialization to EURE string
    let eure_string = to_string(&person).unwrap();
    assert!(!eure_string.is_empty());
    assert!(eure_string.contains("name"));
    assert!(eure_string.contains("Alice"));
    assert!(eure_string.contains("age"));
    assert!(eure_string.contains("30"));
    assert!(eure_string.contains("active"));
    assert!(eure_string.contains("true"));

    // Test round-trip through string
    let deserialized: Person = from_str(&eure_string).unwrap();
    assert_eq!(person, deserialized);
}

#[test]
fn test_struct_value_round_trip() {
    let person = Person {
        name: "Alice".to_string(),
        age: 30,
        active: true,
    };

    // Serialize to Value
    let value = to_value(&person).unwrap();

    // Deserialize from Value
    let from_value: Person = from_value(value).unwrap();
    assert_eq!(person, from_value);
}

#[test]
fn test_struct_from_eure_string() {
    let simple_eure = r#"
name = "Bob"
age = 25
active = false
"#;

    let bob: Person = from_str(simple_eure).unwrap();
    assert_eq!(bob.name, "Bob");
    assert_eq!(bob.age, 25);
    assert!(!bob.active);

    // Test round-trip
    let serialized = to_string(&bob).unwrap();
    let deserialized: Person = from_str(&serialized).unwrap();
    assert_eq!(bob, deserialized);
}

#[test]
fn test_collections() {
    // Test Vec
    let numbers = vec![1, 2, 3, 4, 5];
    let numbers_eure = to_string(&numbers).unwrap();
    assert!(!numbers_eure.is_empty());

    // Test round-trip
    let deserialized: Vec<i32> = from_str(&numbers_eure).unwrap();
    assert_eq!(numbers, deserialized);

    // Test empty Vec
    let empty: Vec<i32> = vec![];
    let empty_eure = to_string(&empty).unwrap();
    let deserialized_empty: Vec<i32> = from_str(&empty_eure).unwrap();
    assert_eq!(empty, deserialized_empty);
}

#[test]
fn test_tuples() {
    // Test tuple with different types
    let tuple = (42, "hello", true);
    let tuple_eure = to_string(&tuple).unwrap();
    assert!(!tuple_eure.is_empty());
    // Verify it contains the expected format
    assert!(tuple_eure.contains("42"));
    assert!(tuple_eure.contains("hello"));
    assert!(tuple_eure.contains("true"));

    // Test direct round-trip
    let deserialized: (i32, String, bool) = from_str(&tuple_eure).unwrap();
    assert_eq!(deserialized.0, tuple.0);
    assert_eq!(deserialized.1, tuple.1);
    assert_eq!(deserialized.2, tuple.2);

    // Test value round-trip as well
    let value = to_value(&tuple).unwrap();
    let from_value: (i32, String, bool) = from_value(value).unwrap();
    assert_eq!(from_value.0, tuple.0);
    assert_eq!(from_value.1, tuple.1);
    assert_eq!(from_value.2, tuple.2);
}

#[test]
fn test_option_types() {
    // Test Some value
    let some_value: Option<i32> = Some(42);
    let some_eure = to_string(&some_value).unwrap();
    assert!(!some_eure.is_empty());

    // Test Some round-trip
    let deserialized_some: Option<i32> = from_str(&some_eure).unwrap();
    assert_eq!(some_value, deserialized_some);

    // Test None value
    let none_value: Option<i32> = None;
    let none_eure = to_string(&none_value).unwrap();
    assert!(!none_eure.is_empty());

    // Test None round-trip
    let deserialized_none: Option<i32> = from_str(&none_eure).unwrap();
    assert_eq!(none_value, deserialized_none);

    // Test Option<String>
    let some_string: Option<String> = Some("test".to_string());
    let some_string_eure = to_string(&some_string).unwrap();
    let deserialized_string: Option<String> = from_str(&some_string_eure).unwrap();
    assert_eq!(some_string, deserialized_string);
}

#[test]
fn test_complex_nested_structure() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Team {
        name: String,
        members: Vec<Person>,
        active: bool,
    }

    let team = Team {
        name: "Engineering".to_string(),
        members: vec![
            Person {
                name: "Alice".to_string(),
                age: 30,
                active: true,
            },
            Person {
                name: "Bob".to_string(),
                age: 25,
                active: false,
            },
        ],
        active: true,
    };

    // Test serialization
    let eure_string = to_string(&team).unwrap();
    assert!(!eure_string.is_empty());

    // Test round-trip
    let deserialized: Team = from_str(&eure_string).unwrap();
    assert_eq!(team, deserialized);

    // Test through Value
    let value = to_value(&team).unwrap();
    let from_value: Team = from_value(value).unwrap();
    assert_eq!(team, from_value);
}
