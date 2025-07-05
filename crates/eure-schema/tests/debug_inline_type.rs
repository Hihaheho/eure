//! Debug test for inline type annotation

use eure_schema::{extract_schema_from_value, validate_with_schema_value};

#[test]
fn debug_inline_type_schema() {
    // The schema that defines the Action type
    let schema_doc = r#"
@ $types.Action
@ $types.Action.$variants.create.name
$type = .string
@ $types.Action.$variants.delete.id
$type = .number
"#;
    
    let base_schema = extract_schema_from_value(schema_doc)
        .expect("Failed to extract schema")
        .document_schema;
    
    println!("Base schema types:");
    for (name, type_def) in &base_schema.types {
        println!("  Type: {name:?}");
        if let eure_schema::Type::Variants(variant_schema) = &type_def.type_expr {
            for (variant_name, variant_obj) in &variant_schema.variants {
                println!("    Variant: {variant_name:?}");
                for (field_name, field_schema) in &variant_obj.fields {
                    println!("      Field: {:?} -> {:?}", field_name, field_schema.type_expr);
                }
            }
        }
    }
    
    // The test document with inline type annotation
    let test_doc = r#"
@ action
$type = .$types.Action
$variant = "create"
name = "New Item"
"#;
    
    // Extract inline schema from the test document
    let inline_schema = extract_schema_from_value(test_doc)
        .expect("Failed to extract inline schema");
        
    println!("\nInline schema root fields:");
    for (name, field) in &inline_schema.document_schema.root.fields {
        println!("  Field: {:?} -> {:?}", name, field.type_expr);
    }
    
    // Merge schemas (what validate_with_inline does)
    let mut merged_schema = base_schema;
    for (name, type_def) in inline_schema.document_schema.types {
        merged_schema.types.insert(name, type_def);
    }
    for (name, field_schema) in inline_schema.document_schema.root.fields {
        merged_schema.root.fields.insert(name, field_schema);
    }
    
    println!("\nMerged schema root fields:");
    for (name, field) in &merged_schema.root.fields {
        println!("  Field: {:?} -> {:?}", name, field.type_expr);
        if let eure_schema::Type::TypeRef(type_name) = &field.type_expr {
            println!("    References type: {type_name:?}");
        }
    }
    
    // Now check what happens during validation
    let errors = validate_with_schema_value(test_doc, merged_schema)
        .expect("Failed to validate");
    
    println!("\nValidation errors:");
    for error in &errors {
        println!("  {:?}", error.kind);
    }
}