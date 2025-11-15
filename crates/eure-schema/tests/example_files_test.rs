use eure_schema::{extract_schema_from_value, validate_with_schema_value};
use eure_value::value::ObjectKey;
use std::fs;

#[test]
fn test_example_files_validation() {
    // Load the actual schema file
    println!("Current dir: {:?}", std::env::current_dir());
    let schema_path = "../../example.schema.eure";
    println!("Trying to read: {schema_path}");
    let schema_input = match fs::read_to_string(schema_path) {
        Ok(content) => {
            println!("Successfully read file, length: {}", content.len());
            content
        }
        Err(e) => {
            println!("Failed to read file: {e}");
            // Try alternative path
            let alt_path = "example.schema.eure";
            println!("Trying alternative path: {alt_path}");
            fs::read_to_string(alt_path)
                .expect("Failed to read example.schema.eure from alternative path")
        }
    };

    // Extract schema using value-based API
    println!(
        "Schema content preview: {}",
        &schema_input[..100.min(schema_input.len())]
    );

    // Check for the specific issue with description.$optional
    // Let's split the content by lines and check
    for (i, line) in schema_input.lines().enumerate() {
        if line.contains(".$optional") {
            println!("Line {}: {}", i + 1, line);
        }
    }

    let extracted = match extract_schema_from_value(&schema_input) {
        Ok(ex) => {
            println!("Schema extraction succeeded");
            println!("Is pure schema: {}", ex.is_pure_schema);
            ex
        }
        Err(e) => {
            println!("Schema extraction failed: {e:?}");
            panic!("Failed to extract schema");
        }
    };

    // Note: example.schema.eure contains both type definitions ($types) and
    // root-level field schemas (@ script section), so it's not a "pure" schema
    // in the strict sense. This is fine - it's a valid schema document.

    // Check that script field exists and is an object
    assert!(
        extracted
            .document_schema
            .root
            .fields
            .contains_key(&ObjectKey::String("script".to_string()))
    );
    if let Some(script_field) = extracted
        .document_schema
        .root
        .fields
        .get(&ObjectKey::String("script".to_string()))
    {
        match &script_field.type_expr {
            eure_schema::Type::Object(obj_schema) => {
                // Verify expected fields exist
                assert!(
                    obj_schema
                        .fields
                        .contains_key(&ObjectKey::String("id".to_string())),
                    "script should have 'id' field"
                );
                assert!(
                    obj_schema
                        .fields
                        .contains_key(&ObjectKey::String("description".to_string())),
                    "script should have 'description' field"
                );
                assert!(
                    obj_schema
                        .fields
                        .contains_key(&ObjectKey::String("actions".to_string())),
                    "script should have 'actions' field"
                );
            }
            _ => panic!("script field should be an Object type"),
        }
    }

    // Load the example document
    let doc_input = fs::read_to_string("../../example.eure").expect("Failed to read example.eure");

    // Validate - use the extracted schema directly (bypass $schema reference)
    let mut test_schema = extracted.document_schema.clone();
    test_schema.schema_ref = None; // Clear schema ref to avoid circular loading
    let errors =
        validate_with_schema_value(&doc_input, test_schema).expect("Failed to validate document");

    // Print errors for debugging
    if !errors.is_empty() {
        println!("Validation errors found:");
        for error in &errors {
            println!("  {:?}", error.kind);
        }
    }

    // Check that there are no unexpected field errors for id and description
    let unexpected_field_errors: Vec<_> = errors
        .iter()
        .filter(|e| {
            if let eure_schema::ValidationErrorKind::UnexpectedField { field, .. } = &e.kind {
                field == &eure_value::value::ObjectKey::String("id".to_string())
                    || field == &eure_value::value::ObjectKey::String("description".to_string())
            } else {
                false
            }
        })
        .collect();

    assert!(
        unexpected_field_errors.is_empty(),
        "Fields 'id' and 'description' should not be marked as unexpected in @ script section"
    );
}
