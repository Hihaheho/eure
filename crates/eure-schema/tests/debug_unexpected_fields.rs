//! Debug test to understand why unexpected fields aren't being reported

use eure_schema::{extract_schema_from_value, validate_with_tree};

#[test]
fn debug_unexpected_fields_in_example() {
    println!("\n=== Debug Unexpected Fields Test ===\n");
    
    // Load the schema
    let schema_content = include_str!("../../../example.schema.eure");
    println!("Schema loaded");
    
    // Extract schema
    let extracted = extract_schema_from_value(schema_content)
        .expect("Failed to extract schema");
    println!("Schema extracted successfully");
    
    // Show what fields are defined for script
    println!("\nSchema definition for 'script':");
    if let Some(script_field) = extracted.document_schema.root.fields
        .get(&eure_schema::KeyCmpValue::String("script".to_string())) {
        println!("  Script field found in schema");
        if let eure_schema::Type::Object(obj_schema) = &script_field.type_expr {
            println!("  Script is an object with fields:");
            for (key, field) in &obj_schema.fields {
                println!("    - {:?} (optional: {})", key, field.optional);
            }
        }
    }
    
    // Load the document
    let doc_content = include_str!("../../../example.eure");
    
    // Parse the document
    let tree = eure_parol::parse(doc_content).expect("Failed to parse document");
    
    // Validate
    println!("\n=== Validation ===");
    let errors = validate_with_tree(doc_content, extracted.document_schema, &tree)
        .expect("Failed to validate");
    
    println!("\nFound {} errors:", errors.len());
    for (i, error) in errors.iter().enumerate() {
        println!("  {}. {:?}", i + 1, error.kind);
    }
    
    // Check what we expect
    println!("\n=== Expected vs Actual ===");
    println!("The document has these fields in @ script:");
    println!("  - id (✓ in schema)");
    println!("  - description (✓ in schema)");
    println!("  - text (✗ NOT in schema - should be unexpected!)");
    println!("  - aaa (✗ NOT in schema - should be unexpected!)");
    println!("  - actions (✓ in schema)");
    
    // Count unexpected field errors
    let unexpected_count = errors.iter()
        .filter(|e| matches!(&e.kind, eure_schema::ValidationErrorKind::UnexpectedField { .. }))
        .count();
    
    println!("\nUnexpected field errors found: {unexpected_count}");
    println!("Expected at least 2 (for 'text' and 'aaa')");
    
    // This will fail to show the issue
    assert!(unexpected_count >= 2, 
        "Should have at least 2 unexpected field errors, but found {unexpected_count}");
}