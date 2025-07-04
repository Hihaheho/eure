//! Debug test for tuple validation

use eure_schema::validate_self_describing;

#[test]
fn debug_tuple_validation() {
    let input = r#"
coordinates.$type = .array
coordinates[] = .number
coordinates = (10, "invalid", 5)
"#;

    let result = validate_self_describing(input).expect("Failed to parse");
    
    // Print all errors for debugging
    println!("Errors: {:?}", result.errors);
    println!("Schema is pure: {:?}", result.schema.is_pure_schema);
    
    // The validation should find a type error
    assert!(!result.errors.is_empty(), "Should have validation errors");
}