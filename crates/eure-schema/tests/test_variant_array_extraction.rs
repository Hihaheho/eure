//! Test variant array field extraction

use eure_schema::{extract_schema_from_value, KeyCmpValue, Type};

#[test]
fn test_variant_array_field_extraction() {
    // Schema with simple array field in variant
    let schema_input = r#"
$types.Action {
  @ $variants.set-text {
    speaker = .string
    lines.$array = .string
    code1 = .code.rust
    code2 = .code.rust
  }
  @ $variants.set-choices {
    description = .string
  }
  @ $variants.set-choices.choice.$array {
    text = .string
    value = .string
  }
}
"#;

    let extracted = extract_schema_from_value(schema_input)
        .expect("Failed to extract schema");

    // Check that Action type was extracted
    let action_type = extracted.document_schema.types
        .get(&KeyCmpValue::String("Action".to_string()))
        .expect("Action type not found");

    // Ensure it's a variant type
    let Type::Variants(variant_def) = &action_type.type_expr else {
        panic!("Expected Action to be a variant type");
    };

    // Check set-text variant
    let set_text = variant_def.variants
        .get(&KeyCmpValue::String("set-text".to_string()))
        .expect("set-text variant not found");

    // Verify all 4 fields are present
    assert_eq!(set_text.fields.len(), 4, "set-text should have 4 fields");

    // Check lines field specifically
    let lines_field = set_text.fields
        .get(&KeyCmpValue::String("lines".to_string()))
        .expect("lines field not found in set-text variant");

    // Verify it's an array type
    match &lines_field.type_expr {
        Type::Array(elem_type) => {
            // Verify element type is string
            match elem_type.as_ref() {
                Type::String => {
                    // Success!
                }
                _ => panic!("Expected lines array element type to be string, got: {elem_type:?}"),
            }
        }
        _ => panic!("Expected lines to be an array type, got: {:?}", lines_field.type_expr),
    }

    // Also verify other fields are present
    assert!(set_text.fields.contains_key(&KeyCmpValue::String("speaker".to_string())));
    assert!(set_text.fields.contains_key(&KeyCmpValue::String("code1".to_string())));
    assert!(set_text.fields.contains_key(&KeyCmpValue::String("code2".to_string())));
}

#[test]
fn test_complex_array_field_extraction() {
    // Schema with complex array field
    let schema_input = r#"
$types.ComplexType {
  @ $variants.with-array {
    items.$array = {
      name = .string
      value = .number
      optional_field = .boolean
    }
  }
}
"#;

    let extracted = extract_schema_from_value(schema_input)
        .expect("Failed to extract schema");

    let complex_type = extracted.document_schema.types
        .get(&KeyCmpValue::String("ComplexType".to_string()))
        .expect("ComplexType not found");

    let Type::Variants(variant_def) = &complex_type.type_expr else {
        panic!("Expected ComplexType to be a variant type");
    };

    let with_array = variant_def.variants
        .get(&KeyCmpValue::String("with-array".to_string()))
        .expect("with-array variant not found");

    let items_field = with_array.fields
        .get(&KeyCmpValue::String("items".to_string()))
        .expect("items field not found");

    // Verify it's an array of objects
    match &items_field.type_expr {
        Type::Array(elem_type) => {
            match elem_type.as_ref() {
                Type::Object(obj_schema) => {
                    // Verify object has the expected fields
                    assert_eq!(obj_schema.fields.len(), 3);
                    assert!(obj_schema.fields.contains_key(&KeyCmpValue::String("name".to_string())));
                    assert!(obj_schema.fields.contains_key(&KeyCmpValue::String("value".to_string())));
                    
                    let optional_field = obj_schema.fields
                        .get(&KeyCmpValue::String("optional_field".to_string()))
                        .expect("optional_field not found");
                    // In this test, optional_field is not marked as optional in the schema
                    assert!(!optional_field.optional, "optional_field should not be optional in this test");
                }
                _ => panic!("Expected array element to be an object"),
            }
        }
        _ => panic!("Expected items to be an array type"),
    }
}