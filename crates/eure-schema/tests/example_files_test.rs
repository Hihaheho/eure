use eure_schema::{extract_schema, validate_with_schema};
use eure_parol::parse;
use std::fs;

#[test]
fn test_example_files_validation() {
    // Load and parse the actual schema file
    let schema_input = fs::read_to_string("../../example.schema.eure")
        .expect("Failed to read example.schema.eure");
    
    let schema_result = parse(&schema_input);
    let schema_tree = match schema_result {
        Ok(tree) => tree,
        Err(e) => panic!("Failed to parse schema: {:?}", e),
    };
    
    // Extract schema
    let extracted = extract_schema(&schema_input, &schema_tree);
    
    // Verify it's a pure schema
    assert!(extracted.is_pure_schema, "example.schema.eure should be a pure schema file");
    
    // Check that script field exists and is an object
    assert!(extracted.document_schema.root.fields.contains_key("script"));
    if let Some(script_field) = extracted.document_schema.root.fields.get("script") {
        match &script_field.type_expr {
            eure_schema::Type::Object(obj_schema) => {
                // Verify expected fields exist
                assert!(obj_schema.fields.contains_key("id"), "script should have 'id' field");
                assert!(obj_schema.fields.contains_key("description"), "script should have 'description' field");
                assert!(obj_schema.fields.contains_key("actions"), "script should have 'actions' field");
            }
            _ => panic!("script field should be an Object type"),
        }
    }
    
    // Load and parse the example document
    let doc_input = fs::read_to_string("../../example.eure")
        .expect("Failed to read example.eure");
    
    let doc_result = parse(&doc_input);
    let doc_tree = match doc_result {
        Ok(tree) => tree,
        Err(e) => panic!("Failed to parse document: {:?}", e),
    };
    
    // Validate - use the extracted schema directly (bypass $schema reference)
    let mut test_schema = extracted.document_schema.clone();
    test_schema.schema_ref = None; // Clear schema ref to avoid circular loading
    let errors = validate_with_schema(&doc_input, &doc_tree, test_schema);
    
    // Print errors for debugging
    if !errors.is_empty() {
        println!("Validation errors found:");
        for error in &errors {
            println!("  {:?}", error.kind);
        }
    }
    
    // Check that there are no unexpected field errors for id and description
    let unexpected_field_errors: Vec<_> = errors.iter()
        .filter(|e| {
            if let eure_schema::ValidationErrorKind::UnexpectedField { field, .. } = &e.kind {
                field == "id" || field == "description"
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