//! Debug test for variant field extraction

use eure_schema::*;

#[test]
fn debug_variant_field_extraction() {
    // Schema with variant field definitions
    let schema_doc = r#"
@ $types.Action
@ $types.Action.$variants.create.name
$type = .string
@ $types.Action.$variants.delete.id
$type = .number
"#;
    
    let schema = extract_schema_from_value(schema_doc)
        .expect("Failed to extract schema");
    
    println!("Extracted types:");
    for (type_name, type_def) in &schema.document_schema.types {
        println!("  Type: {type_name:?}");
        
        if let eure_schema::Type::Variants(variant_schema) = &type_def.type_expr {
            println!("    Variants found: {}", variant_schema.variants.len());
            
            for (variant_name, variant_obj) in &variant_schema.variants {
                println!("    Variant: {variant_name:?}");
                println!("      Fields: {}", variant_obj.fields.len());
                
                for (field_name, field_schema) in &variant_obj.fields {
                    println!("        Field: {:?} -> Type: {:?}, Optional: {}", 
                        field_name, 
                        field_schema.type_expr,
                        field_schema.optional
                    );
                }
            }
        }
    }
}