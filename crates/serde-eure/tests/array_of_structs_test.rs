use eure_value::value::{Array, Value};
use serde::{Deserialize, Serialize};
use serde_eure::{from_str, from_value, to_string, to_value};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Item {
    name: String,
    value: i32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Container {
    items: Vec<Item>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ComplexItem {
    id: u64,
    name: String,
    tags: Vec<String>,
    metadata: Metadata,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Metadata {
    created: String,
    modified: String,
    version: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ComplexContainer {
    title: String,
    items: Vec<ComplexItem>,
    total_count: usize,
}

#[test]
fn test_simple_array_of_structs() {
    let data = Container {
        items: vec![
            Item {
                name: "first".to_string(),
                value: 1,
            },
            Item {
                name: "second".to_string(),
                value: 2,
            },
            Item {
                name: "third".to_string(),
                value: 3,
            },
        ],
    };

    let serialized = to_string(&data).unwrap();
    assert!(serialized.contains("items"));
    assert!(serialized.contains("first"));
    assert!(serialized.contains("second"));
    assert!(serialized.contains("third"));

    let deserialized: Container = from_str(&serialized).unwrap();
    assert_eq!(data, deserialized);
}

#[test]
fn test_empty_array_of_structs() {
    let data = Container { items: vec![] };

    let serialized = to_string(&data).unwrap();
    assert!(serialized.contains("items"));

    let deserialized: Container = from_str(&serialized).unwrap();
    assert_eq!(data, deserialized);
    assert!(deserialized.items.is_empty());
}

#[test]
fn test_complex_nested_structs_in_array() {
    let data = ComplexContainer {
        title: "Test Collection".to_string(),
        items: vec![
            ComplexItem {
                id: 1,
                name: "Item One".to_string(),
                tags: vec!["important".to_string(), "featured".to_string()],
                metadata: Metadata {
                    created: "2024-01-01".to_string(),
                    modified: "2024-01-02".to_string(),
                    version: 1,
                },
            },
            ComplexItem {
                id: 2,
                name: "Item Two".to_string(),
                tags: vec!["draft".to_string()],
                metadata: Metadata {
                    created: "2024-01-03".to_string(),
                    modified: "2024-01-03".to_string(),
                    version: 2,
                },
            },
        ],
        total_count: 2,
    };

    let serialized = to_string(&data).unwrap();
    let deserialized: ComplexContainer = from_str(&serialized).unwrap();
    assert_eq!(data, deserialized);
}

#[test]
fn test_array_of_structs_with_options() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct OptionalItem {
        name: String,
        description: Option<String>,
        value: Option<i32>,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct OptionalContainer {
        items: Vec<OptionalItem>,
    }

    let data = OptionalContainer {
        items: vec![
            OptionalItem {
                name: "complete".to_string(),
                description: Some("A complete item".to_string()),
                value: Some(100),
            },
            OptionalItem {
                name: "partial".to_string(),
                description: None,
                value: Some(50),
            },
            OptionalItem {
                name: "minimal".to_string(),
                description: None,
                value: None,
            },
        ],
    };

    let serialized = to_string(&data).unwrap();
    let deserialized: OptionalContainer = from_str(&serialized).unwrap();
    assert_eq!(data, deserialized);
}

#[test]
fn test_direct_array_serialization() {
    // Test serializing Vec<Struct> directly without a container
    let items = vec![
        Item {
            name: "alpha".to_string(),
            value: 10,
        },
        Item {
            name: "beta".to_string(),
            value: 20,
        },
    ];

    let serialized = to_string(&items).unwrap();
    assert!(serialized.contains("alpha"));
    assert!(serialized.contains("beta"));

    let deserialized: Vec<Item> = from_str(&serialized).unwrap();
    assert_eq!(items, deserialized);
}

#[test]
fn test_array_of_structs_through_value() {
    let items = vec![
        Item {
            name: "one".to_string(),
            value: 1,
        },
        Item {
            name: "two".to_string(),
            value: 2,
        },
    ];

    // Convert to Value
    let value = to_value(&items).unwrap();

    // Verify it's an Array
    match &value {
        Value::Array(Array(elements)) => {
            assert_eq!(elements.len(), 2);
        }
        _ => panic!("Expected Array value"),
    }

    // Convert back
    let deserialized: Vec<Item> = from_value(value).unwrap();
    assert_eq!(items, deserialized);
}

#[test]
fn test_nested_arrays_of_structs() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Group {
        name: String,
        items: Vec<Item>,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Groups {
        groups: Vec<Group>,
    }

    let data = Groups {
        groups: vec![
            Group {
                name: "Group A".to_string(),
                items: vec![
                    Item {
                        name: "A1".to_string(),
                        value: 1,
                    },
                    Item {
                        name: "A2".to_string(),
                        value: 2,
                    },
                ],
            },
            Group {
                name: "Group B".to_string(),
                items: vec![
                    Item {
                        name: "B1".to_string(),
                        value: 10,
                    },
                    Item {
                        name: "B2".to_string(),
                        value: 20,
                    },
                ],
            },
        ],
    };

    let serialized = to_string(&data).unwrap();
    let deserialized: Groups = from_str(&serialized).unwrap();
    assert_eq!(data, deserialized);
}

#[test]
fn test_array_of_enum_structs() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    enum ItemType {
        Simple { name: String },
        Complex { name: String, value: i32 },
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct EnumContainer {
        items: Vec<ItemType>,
    }

    let data = EnumContainer {
        items: vec![
            ItemType::Simple {
                name: "simple1".to_string(),
            },
            ItemType::Complex {
                name: "complex1".to_string(),
                value: 42,
            },
            ItemType::Simple {
                name: "simple2".to_string(),
            },
        ],
    };

    let serialized = to_string(&data).unwrap();
    println!("Serialized: {serialized}");
    let deserialized: EnumContainer = from_str(&serialized).unwrap();
    assert_eq!(data, deserialized);
}
