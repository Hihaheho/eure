use eure_schema::{extract_schema, validate_with_schema};
use eure_parol::parse;

#[test]
fn debug_unexpected_fields_issue() {
    // Schema that defines a "script" section with "id" and "description" fields
    let schema_input = r#"
$schema = "assets/eure-schema.schema.eure"

@ script
id = .string
description = .string
description.$optional = true
"#;
    
    let schema_result = parse(schema_input);
    let schema_tree = match schema_result {
        Ok(tree) => tree,
        Err(e) => panic!("Failed to parse schema: {:?}", e),
    };
    
    // Extract schema
    let extracted = extract_schema(schema_input, &schema_tree);
    println!("Extracted schema: {:#?}", extracted);
    
    // Check if script field exists in root
    println!("\nRoot fields:");
    for (name, field) in &extracted.document_schema.root.fields {
        println!("  {}: {:?}", name, field.type_expr);
        
        // If it's an object, print its fields
        if let eure_schema::Type::Object(ref obj_schema) = field.type_expr {
            println!("    Fields in {}:", name);
            for (field_name, field_schema) in &obj_schema.fields {
                println!("      {}: {:?}", field_name, field_schema.type_expr);
            }
        }
    }
    
    // Document that uses the schema
    let doc_input = r#"
$schema = "./example.schema.eure"

@ script
id = "test-id"
description = "test description"
"#;
    
    let doc_result = parse(doc_input);
    let doc_tree = match doc_result {
        Ok(tree) => tree,
        Err(e) => panic!("Failed to parse document: {:?}", e),
    };
    
    // Validate
    let errors = validate_with_schema(doc_input, &doc_tree, extracted.document_schema);
    
    println!("\nValidation errors:");
    for error in &errors {
        println!("  {:?} at {:?}", error.kind, error.span);
    }
    
    // The test should pass with no unexpected field errors
    let unexpected_field_errors: Vec<_> = errors.iter()
        .filter(|e| matches!(e.kind, eure_schema::ValidationErrorKind::UnexpectedField { .. }))
        .collect();
    
    if !unexpected_field_errors.is_empty() {
        panic!("Found unexpected field errors: {:?}", unexpected_field_errors);
    }
}