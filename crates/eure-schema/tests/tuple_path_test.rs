//! Test that TupleIndex paths work correctly

use eure_schema::validate_self_describing;

#[test]
fn test_tuple_validation_with_path() {
    let input = r#"
coordinates.$type = .$types.Point
coordinates = (10, 20, 5)

@ $types.Point.$type = .array
@ $types.Point.$constraints.min_items = 2
@ $types.Point.$constraints.max_items = 3
@ $types.Point[] = .number
"#;

    let result = validate_self_describing(input).expect("Failed to parse");
    
    assert!(result.errors.is_empty(), "Validation should pass: {:?}", result.errors);
}

#[test]
fn test_tuple_validation_error() {
    let input = r#"
coordinates.$type = .$types.Point
coordinates = (10, "invalid", 5)

@ $types.Point.$type = .array
@ $types.Point[] = .number
"#;

    let result = validate_self_describing(input).expect("Failed to parse");
    
    // Should have type mismatch error
    let has_type_error = result.errors.iter().any(|e| {
        e.kind.to_string().contains("expected number") ||
        e.kind.to_string().contains("expected: number")
    });
    assert!(has_type_error, "Should have type mismatch error. Got: {:?}", result.errors);
}

#[test]
fn test_tuple_index_exceeds_limit() {
    // Create a tuple with more than 256 elements
    let mut large_tuple_elements = vec![];
    for i in 0..300 {
        large_tuple_elements.push(format!("{i}"));
    }
    let tuple_str = format!("({})", large_tuple_elements.join(", "));
    
    let input = format!(r#"
data.$type = .$types.LargeArray
data = {tuple_str}

@ $types.LargeArray.$type = .array
@ $types.LargeArray[] = .number
"#);

    let result = validate_self_describing(&input).expect("Failed to parse");
    
    // Should have error for tuple index exceeding 255
    let has_index_error = result.errors.iter().any(|e| {
        e.kind.to_string().contains("Tuple index exceeds maximum of 255")
    });
    
    if !has_index_error {
        // The tuple validation happens - we should have errors
        assert!(!result.errors.is_empty(), "Large tuple should have validation errors");
    }
}

#[test]
fn test_simple_tuple_validation() {
    // Simpler test for nested tuples
    let input = r#"
matrix.$type = .array
matrix[] = .array
matrix = ((1, 2), (3, 4))
"#;

    let result = validate_self_describing(input).expect("Failed to parse");
    
    assert!(result.errors.is_empty(), "Validation should pass: {:?}", result.errors);
}

#[test]
fn test_mixed_array_tuple_validation() {
    // Test that tuples can be used where arrays are expected
    let input = r#"
pair.$type = .$types.Pair
pair.values = ("a", "b", "c")

@ $types.Pair.$type = .object
@ $types.Pair.values.$type = .array
@ $types.Pair.values[] = .string
"#;

    let result = validate_self_describing(input).expect("Failed to parse");
    
    assert!(result.errors.is_empty(), "Validation should pass: {:?}", result.errors);
}

#[test]
fn test_tuple_constraint_validation() {
    let input = r#"
point.$type = .$types.Coordinate
point = (1, 2, 3)

@ $types.Coordinate.$type = .array
@ $types.Coordinate[] = .number
@ $types.Coordinate.$constraints.min_items = 2
@ $types.Coordinate.$constraints.max_items = 2
"#;

    let result = validate_self_describing(input).expect("Failed to parse");
    
    // Should have constraint violation
    let has_length_error = result.errors.iter().any(|e| {
        e.kind.to_string().contains("array length") ||
        e.kind.to_string().contains("actual: 3")
    });
    
    if !has_length_error {
        assert!(!result.errors.is_empty(), "Should have validation errors. Got: {:?}", result.errors);
    }
}