use serde_eure::to_string;

#[test]
fn test_array_with_binding_parses_successfully() {
    // Test 1: Array in a binding (should work)
    let eure_with_binding = r#"data = [1, 2, 3,]"#;

    let result = eure_parol::parse(eure_with_binding);
    assert!(
        result.is_ok(),
        "Array with binding should parse successfully"
    );
}

#[test]
fn test_bare_array_fails_to_parse() {
    // Test 2: Bare array (should fail to parse)
    let bare_array = r#"[1, 2, 3,]"#;

    let result = eure_parol::parse(bare_array);
    assert!(result.is_err(), "Bare array should fail to parse");
}

#[test]
fn test_serde_eure_vec_serialization() {
    // Test 3: What serde-eure produces for a Vec
    let vec = vec![1, 2, 3];
    let serialized = to_string(&vec).unwrap();

    // Verify the serialized output is not empty
    assert!(
        !serialized.is_empty(),
        "Serialized output should not be empty"
    );

    // The serialized Vec should parse successfully
    let result = eure_parol::parse(&serialized);
    assert!(
        result.is_ok(),
        "Serialized Vec should parse successfully. Output was: {serialized}"
    );
}

#[test]
fn test_parse_error_for_bare_array() {
    // Additional test to verify the error type for bare arrays
    let bare_array = r#"[1, 2, 3,]"#;

    match eure_parol::parse(bare_array) {
        Ok(_) => panic!("Expected parse error for bare array, but it parsed successfully"),
        Err(e) => {
            // Verify we get an error (the specific error message may vary)
            let error_string = e.to_string();
            assert!(
                !error_string.is_empty(),
                "Error message should not be empty"
            );
        }
    }
}

#[test]
fn test_different_array_formats() {
    // Test various array formats with bindings
    let test_cases = vec![
        (r#"data = []"#, true, "Empty array with binding"),
        (r#"data = [1]"#, true, "Single element array with binding"),
        (r#"data = [1, 2, 3]"#, true, "Array without trailing comma"),
        (r#"data = [1, 2, 3,]"#, true, "Array with trailing comma"),
        (r#"[]"#, false, "Bare empty array"),
        (r#"[1, 2, 3]"#, false, "Bare array without trailing comma"),
    ];

    for (input, should_succeed, description) in test_cases {
        let result = eure_parol::parse(input);
        if should_succeed {
            assert!(result.is_ok(), "{description} should parse successfully");
        } else {
            assert!(result.is_err(), "{description} should fail to parse");
        }
    }
}
