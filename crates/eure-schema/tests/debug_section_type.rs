use eure_schema::*;

#[test]
fn debug_section_level_type() {
    // Define a schema with variant type
    let schema_doc = r#"
@ $types.Action
@ $types.Action.$variants.create.name
$type = .string
@ $types.Action.$variants.delete.id
$type = .number
"#;
    
    // Extract the schema
    let schema = extract_schema_from_value(schema_doc).expect("Failed to extract schema");
    println!("Extracted schema:");
    println!("  Types: {:?}", schema.document_schema.types.keys().collect::<Vec<_>>());
    
    // Test document with section-level $type
    let test_doc = r#"
@ action
$type = .$types.Action
$variant = "create"
name = "New Item"
"#;
    
    // Extract inline schema from test document
    let inline_schema = extract_schema_from_value(test_doc).expect("Failed to extract inline schema");
    println!("\nInline schema extraction:");
    println!("  Root fields: {:?}", inline_schema.document_schema.root.fields.keys().collect::<Vec<_>>());
    for (key, field) in &inline_schema.document_schema.root.fields {
        println!("    Field {:?}: type = {:?}", key, field.type_expr);
    }
    
    // Merge schemas
    let mut merged_schema = schema.document_schema;
    for (name, field_schema) in inline_schema.document_schema.root.fields {
        merged_schema.root.fields.insert(name, field_schema);
    }
    
    println!("\nMerged schema root fields:");
    for (key, field) in &merged_schema.root.fields {
        println!("  Field {:?}: type = {:?}", key, field.type_expr);
    }
    
    // Validate
    let errors = validate_with_schema_value(test_doc, merged_schema).expect("Failed to validate");
    println!("\nValidation errors: {}", errors.len());
    for error in &errors {
        println!("  - {:?}", error.kind);
    }
}