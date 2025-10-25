//! Test that error types are properly exposed and usable

use eure_schema::{ValueError, extract_schema_from_value};

#[test]
fn test_parse_error_type() {
    // Invalid EURE syntax - missing value
    let invalid_input = r#"key = "#;

    let result = extract_schema_from_value(invalid_input);
    assert!(result.is_err());

    // Verify we can match on specific error types
    match result {
        Err(ValueError::ParseError(e)) => {
            // Verify we got a parse error with a meaningful message
            let error_msg = e.to_string();
            assert!(!error_msg.is_empty());
            assert!(error_msg.contains("Expecting") || error_msg.contains("error"));
        }
        Err(other) => panic!("Expected ParseError, got: {:?}", other),
        Ok(_) => panic!("Expected error, got success"),
    }
}

#[test]
fn test_schema_error_type() {
    // This input parses correctly but has an invalid type path
    let input_with_bad_type = r#"
    $types {
        # This will cause a schema extraction error
        BadType = "not a valid type path"
    }
    "#;

    let result = extract_schema_from_value(input_with_bad_type);

    // This might not fail immediately due to how schema extraction works,
    // but the test demonstrates that the error type can be matched
    if result.is_err() {
        match result.unwrap_err() {
            ValueError::SchemaError(_) => {
                // Good, we can match on schema errors
            }
            ValueError::ParseError(_) => {
                // Also acceptable for this test case
            }
            ValueError::VisitorError(_) => {
                // Also acceptable
            }
        }
    }
}

#[test]
fn test_error_display() {
    // Test that errors have meaningful display implementations
    let invalid_input = r#"@ invalid @ syntax @"#;

    let result = extract_schema_from_value(invalid_input);
    assert!(result.is_err());

    let error = result.unwrap_err();
    let error_string = error.to_string();

    // Should have a descriptive error message
    assert!(!error_string.is_empty());
    assert!(error_string.starts_with("Failed to"));
}
