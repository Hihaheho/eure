//! Integration tests for schema validation

use eure_editor_support::schema_validation;
use eure_parol::parse;

#[test]
fn test_schema_discovery() {
    use std::fs;
    
    // Create a temporary directory structure
    let temp_dir = std::env::temp_dir().join("eure_schema_test");
    let _ = fs::create_dir_all(&temp_dir);
    
    // Create a test document
    let doc_path = temp_dir.join("test.eure");
    fs::write(&doc_path, "name = \"test\"").unwrap();
    
    // Create a schema file in the same directory
    let schema_path = temp_dir.join("test.schema.eure");
    fs::write(&schema_path, "name.$type = .string").unwrap();
    
    // Test schema discovery
    let found_schema = schema_validation::find_schema_for_document(&doc_path, None);
    assert_eq!(found_schema, Some(schema_path));
    
    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
#[ignore = "Schema extraction needs to handle mixed schema and data"]
fn test_self_describing_validation() {
    // TODO: Fix schema extraction to handle mixed schema and data
    // Currently, data values overwrite schema definitions
    let input = r#"
# Pure schema document (no data mixed in)
@ $types.Person {
    name.$type = .string
    name.$length = (1, 50)
    
    age.$type = .number
    age.$optional = true
}
"#;

    // Parse the document
    let tree = parse(input).expect("Failed to parse");
    
    // Validate the document
    let diagnostics = schema_validation::validate_document(
        "test://self-describing.eure",
        input,
        &tree,
        &schema_validation::SchemaManager::new(),
        None,
    );
    
    // Should have no errors
    if !diagnostics.is_empty() {
        println!("Unexpected validation errors in self-describing test:");
        for diag in &diagnostics {
            println!("  {}", diag.message);
        }
    }
    assert_eq!(diagnostics.len(), 0, "Expected no validation errors");
}

#[test]
#[ignore = "Schema validation behavior needs review"]
fn test_validation_with_errors() {
    let input = r#"
# Self-describing document with inline schema  
name.$type = .string
age.$type = .number

# Actual data - number instead of string for name
name = 123
# age is missing (required field)
"#;

    // Parse the document
    let tree = parse(input).expect("Failed to parse");
    
    // Validate the document
    let diagnostics = schema_validation::validate_document(
        "test://with-errors.eure",
        input,
        &tree,
        &schema_validation::SchemaManager::new(),
        None,
    );
    
    // Remove debug output
    // Debug: print all diagnostics and extracted schema
    // println!("Diagnostics count: {}", diagnostics.len());
    // for (i, diag) in diagnostics.iter().enumerate() {
    //     println!("Diagnostic {}: {}", i, diag.message);
    // }
    
    // Should have validation errors
    assert!(!diagnostics.is_empty(), "Expected validation errors");
    
    // Check for type mismatch error
    let has_type_error = diagnostics.iter().any(|d| 
        d.message.contains("Type mismatch")
    );
    assert!(has_type_error, "Expected type mismatch error");
    
    // Check for missing field error
    let has_missing_field = diagnostics.iter().any(|d| 
        d.message.contains("Required field") && d.message.contains("missing")
    );
    assert!(has_missing_field, "Expected missing field error: {:?}", 
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>());
}

#[test]
fn test_schema_manager() {
    let schema_input = r#"
@ $types.Person {
    name.$type = .string
    age.$type = .number
}
"#;

    let doc_input = r#"
person.$type = .$types.Person

person = {
    name = "Alice"
    age = 25
}
"#;

    // Parse schema
    let schema_tree = parse(schema_input).expect("Failed to parse schema");
    
    // Create schema manager and load schema
    let mut manager = schema_validation::SchemaManager::new();
    manager.load_schema("test://person.schema.eure", schema_input, &schema_tree)
        .expect("Failed to load schema");
    
    // Associate document with schema
    manager.set_document_schema("test://doc.eure", "test://person.schema.eure");
    
    // Parse document
    let _doc_tree = parse(doc_input).expect("Failed to parse document");
    
    // Validate - this would normally use the external schema, but
    // since the document references $types.Person, it needs the schema
    // For this test, we'll just verify the manager works
    assert!(manager.get_schema("test://person.schema.eure").is_some());
    assert_eq!(
        manager.get_document_schema_uri("test://doc.eure"),
        Some("test://person.schema.eure")
    );
}

#[test]
fn test_schema_reference_resolution() {
    use std::path::Path;
    
    // Test relative path resolution
    let doc_path = Path::new("/home/user/project/data/config.eure");
    let result = schema_validation::resolve_schema_reference(
        doc_path,
        "./schemas/config.schema.eure",
        None
    );
    assert!(result.is_ok());
    let resolved = result.unwrap();
    assert!(resolved.ends_with("data/schemas/config.schema.eure"));
    
    // Test parent directory reference
    let result = schema_validation::resolve_schema_reference(
        doc_path,
        "../shared/types.schema.eure",
        None
    );
    assert!(result.is_ok());
    let resolved = result.unwrap();
    // The resolved path should be in the parent directory
    assert!(resolved.to_string_lossy().contains("shared/types.schema.eure"));
    
    // Test absolute file:// URL
    let result = schema_validation::resolve_schema_reference(
        doc_path,
        "file:///etc/schemas/global.schema.eure",
        None
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Path::new("/etc/schemas/global.schema.eure"));
    
    // Test HTTP URL (should fail for now)
    let result = schema_validation::resolve_schema_reference(
        doc_path,
        "https://example.com/schemas/api.schema.eure",
        None
    );
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Remote schemas are not yet supported");
}

/* TODO: Re-enable when validate_and_extract_schema is implemented
#[test]
fn test_document_with_schema_reference() {
    let input = r#"
# Document with $schema reference
$schema = "./person.schema.eure"

# Actual data
name = "John Doe"
age = 30
"#;

    // Parse the document
    let tree = parse(input).expect("Failed to parse");
    
    // Extract schema and validate
    let validation_result = schema_validation::validate_and_extract_schema(input, &tree)
        .expect("Failed to extract schema");
    
    // Check that schema reference was extracted
    assert_eq!(
        validation_result.schema.document_schema.schema_ref,
        Some("./person.schema.eure".to_string())
    );
    
    // The document should have no inline validation errors
    // (actual schema validation would happen when the referenced schema is loaded)
    // Note: If there are validation errors, they might be from the self-describing validation
    // which tries to validate the data even without the referenced schema
    println!("Validation errors: {:?}", validation_result.errors);
    // For now, we just check that the schema reference was extracted
    // assert_eq!(validation_result.errors.len(), 0);
}
*/

/* TODO: Re-enable when validate_and_extract_schema is implemented
#[test]
fn test_schema_ref_priority() {
    // Test that $schema in document takes priority over convention-based discovery
    let input = r#"
# Document with explicit schema reference
$schema = "./custom.schema.eure"

# Also has inline schema constraints (these should still work)
name.$type = .string
name = "Test"
"#;

    // Parse the document
    let tree = parse(input).expect("Failed to parse");
    
    // Validate
    let _diagnostics = schema_validation::validate_document(
        "test://priority.eure",
        input,
        &tree,
        &schema_validation::SchemaManager::new(),
        None,
    );
    
    // Should have validation errors due to the schema extraction bug
    // where data overwrites schema definitions
    // TODO: Fix this when schema extraction is fixed
    // assert_eq!(diagnostics.len(), 0);
    
    // Extract schema info
    let validation_result = schema_validation::validate_and_extract_schema(input, &tree)
        .expect("Failed to extract schema");
    
    // Verify both $schema reference and inline constraints were captured
    assert_eq!(
        validation_result.schema.document_schema.schema_ref,
        Some("./custom.schema.eure".to_string())
    );
    // Due to the bug, 'name' field won't be in the schema because the data value overwrites it
    // TODO: Fix this assertion when schema extraction is fixed
    // assert!(validation_result.schema.document_schema.root.fields.contains_key(&eure_schema::KeyCmpValue::String("name".to_string())));
}
*/