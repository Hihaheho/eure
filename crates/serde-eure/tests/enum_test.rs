use serde::{Deserialize, Serialize};
use serde_eure::{from_str, to_string};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum SimpleEnum {
    Unit,
    Newtype(String),
    Struct { field: i32 },
}

#[test]
fn test_unit_variant() {
    let unit = SimpleEnum::Unit;
    let unit_eure = to_string(&unit).unwrap();
    eprintln!("Unit variant serialized as: {}", unit_eure);

    let unit_back: SimpleEnum = from_str(&unit_eure).unwrap();
    assert_eq!(unit, unit_back);
}

#[test]
fn test_newtype_variant() {
    // The serializer produces format: {$tag = "Newtype", $content = "hello"}
    // But the deserializer expects the newtype content directly, not wrapped in a map
    let newtype = SimpleEnum::Newtype("hello".to_string());
    let newtype_eure = to_string(&newtype).unwrap();

    let newtype_back: SimpleEnum = from_str(&newtype_eure).unwrap();
    assert_eq!(newtype, newtype_back);
}

#[test]
fn test_struct_variant() {
    let struct_var = SimpleEnum::Struct { field: 42 };
    let struct_eure = to_string(&struct_var).unwrap();

    let struct_back: SimpleEnum = from_str(&struct_eure).unwrap();
    assert_eq!(struct_var, struct_back);
}

#[test]
fn test_array_of_enums() {
    // Test array without newtype variant due to known issue
    let array = vec![SimpleEnum::Unit, SimpleEnum::Struct { field: 99 }];
    let array_eure = to_string(&array).unwrap();

    let array_back: Vec<SimpleEnum> = from_str(&array_eure).unwrap();
    assert_eq!(array, array_back);
}

#[test]
fn test_array_with_newtype_variant() {
    let array = vec![
        SimpleEnum::Unit,
        SimpleEnum::Newtype("test".to_string()),
        SimpleEnum::Struct { field: 99 },
    ];
    let array_eure = to_string(&array).unwrap();

    let array_back: Vec<SimpleEnum> = from_str(&array_eure).unwrap();
    assert_eq!(array, array_back);
}
