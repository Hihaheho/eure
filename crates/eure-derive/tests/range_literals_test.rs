use eure_derive::Eure;
use eure_schema::{ObjectKey, ToEureSchema, Type};
use serde::{Deserialize, Serialize};

#[test]
fn test_range_with_integer_literals() {
    #[derive(Eure, Serialize, Deserialize)]
    struct TestStruct {
        #[eure(range(min = 0, max = 150))]
        age: u32,
    }

    let schema = TestStruct::eure_schema();

    if let Type::Object(obj_schema) = schema.type_expr {
        let age_field = obj_schema
            .fields
            .get(&ObjectKey::String("age".to_string()))
            .unwrap();
        assert_eq!(age_field.constraints.range, Some((Some(0.0), Some(150.0))));
    } else {
        panic!("Expected object schema");
    }
}

#[test]
fn test_range_with_float_literals() {
    #[derive(Eure, Serialize, Deserialize)]
    struct TestStruct {
        #[eure(range(min = 0.5, max = 99.9))]
        score: f64,
    }

    let schema = TestStruct::eure_schema();

    if let Type::Object(obj_schema) = schema.type_expr {
        let score_field = obj_schema
            .fields
            .get(&ObjectKey::String("score".to_string()))
            .unwrap();
        assert_eq!(score_field.constraints.range, Some((Some(0.5), Some(99.9))));
    } else {
        panic!("Expected object schema");
    }
}

#[test]
fn test_range_with_mixed_literals() {
    #[derive(Eure, Serialize, Deserialize)]
    struct TestStruct {
        #[eure(range(min = 0, max = 99.9))]
        value: f32,
    }

    let schema = TestStruct::eure_schema();

    if let Type::Object(obj_schema) = schema.type_expr {
        let value_field = obj_schema
            .fields
            .get(&ObjectKey::String("value".to_string()))
            .unwrap();
        assert_eq!(value_field.constraints.range, Some((Some(0.0), Some(99.9))));
    } else {
        panic!("Expected object schema");
    }
}

#[test]
fn test_range_with_negative_integers() {
    #[derive(Eure, Serialize, Deserialize)]
    struct TestStruct {
        #[eure(range(min = -100, max = 100))]
        temperature: i32,
    }

    let schema = TestStruct::eure_schema();

    if let Type::Object(obj_schema) = schema.type_expr {
        let temp_field = obj_schema
            .fields
            .get(&ObjectKey::String("temperature".to_string()))
            .unwrap();
        assert_eq!(
            temp_field.constraints.range,
            Some((Some(-100.0), Some(100.0)))
        );
    } else {
        panic!("Expected object schema");
    }
}
