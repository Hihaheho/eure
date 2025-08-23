use serde::{Deserialize, Serialize};
use serde_eure::{to_string, from_str, to_value, from_value};

#[test]
fn test_empty_tuple() {
    let empty: () = ();
    let serialized = to_string(&empty).unwrap();
    assert_eq!(serialized, "value = ()\n");
    
    let deserialized: () = from_str(&serialized).unwrap();
    assert_eq!(deserialized, empty);
}

#[test]
fn test_single_element_tuple() {
    let single = (42,);
    let serialized = to_string(&single).unwrap();
    assert_eq!(serialized, "value = (42,)\n");
    
    let deserialized: (i32,) = from_str(&serialized).unwrap();
    assert_eq!(deserialized, single);
}

#[test]
fn test_mixed_type_tuple() {
    let mixed = (1, "hello", 3.14, true);
    let serialized = to_string(&mixed).unwrap();
    
    let deserialized: (i32, String, f64, bool) = from_str(&serialized).unwrap();
    assert_eq!(deserialized.0, mixed.0);
    assert_eq!(deserialized.1, mixed.1);
    assert_eq!(deserialized.2, mixed.2);
    assert_eq!(deserialized.3, mixed.3);
}

#[test]
fn test_nested_tuples() {
    let nested = ((1, 2), (3, 4));
    let serialized = to_string(&nested).unwrap();
    
    let deserialized: ((i32, i32), (i32, i32)) = from_str(&serialized).unwrap();
    assert_eq!(deserialized, nested);
}

#[test]
fn test_tuple_in_struct() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct WithTuple {
        data: (String, i32),
        flag: bool,
    }
    
    let obj = WithTuple {
        data: ("test".to_string(), 42),
        flag: true,
    };
    
    let serialized = to_string(&obj).unwrap();
    let deserialized: WithTuple = from_str(&serialized).unwrap();
    assert_eq!(deserialized, obj);
}

#[test]
fn test_tuple_in_array() {
    let array_of_tuples = vec![(1, "a"), (2, "b"), (3, "c")];
    let serialized = to_string(&array_of_tuples).unwrap();
    
    let deserialized: Vec<(i32, String)> = from_str(&serialized).unwrap();
    assert_eq!(deserialized.len(), array_of_tuples.len());
    for (i, (num, letter)) in deserialized.iter().enumerate() {
        assert_eq!(*num, array_of_tuples[i].0);
        assert_eq!(*letter, array_of_tuples[i].1);
    }
}

#[test]
fn test_tuple_with_options() {
    let with_options = (Some(42), None::<String>, Some("test"));
    let serialized = to_string(&with_options).unwrap();
    
    let deserialized: (Option<i32>, Option<String>, Option<String>) = from_str(&serialized).unwrap();
    assert_eq!(deserialized.0, with_options.0);
    assert_eq!(deserialized.1, with_options.1);
    assert_eq!(deserialized.2.as_deref(), with_options.2);
}

#[test]
fn test_large_tuple() {
    let large = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
    let serialized = to_string(&large).unwrap();
    
    let deserialized: (i32, i32, i32, i32, i32, i32, i32, i32, i32, i32) = from_str(&serialized).unwrap();
    assert_eq!(deserialized, large);
}

#[test]
fn test_tuple_struct() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Point(f64, f64);
    
    let point = Point(3.14, 2.71);
    let serialized = to_string(&point).unwrap();
    
    let deserialized: Point = from_str(&serialized).unwrap();
    assert_eq!(deserialized, point);
}

#[test]
fn test_tuple_variant() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    enum MyEnum {
        TupleVariant(i32, String),
    }
    
    let variant = MyEnum::TupleVariant(42, "test".to_string());
    let serialized = to_string(&variant).unwrap();
    
    let deserialized: MyEnum = from_str(&serialized).unwrap();
    assert_eq!(deserialized, variant);
}

#[test]
fn test_integer_to_float_conversion() {
    // Serialize integers
    let int_tuple = (1, 2, 3);
    let serialized = to_string(&int_tuple).unwrap();
    
    // Deserialize as floats (should convert)
    let float_tuple: (f64, f64, f64) = from_str(&serialized).unwrap();
    assert_eq!(float_tuple, (1.0, 2.0, 3.0));
}