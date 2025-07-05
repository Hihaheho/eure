//! Debug test to understand parsed structure

use eure_schema::*;

#[test]
fn debug_parsed_structure() {
    // First, let's understand how types with nested variant fields are structured
    let doc = r#"
@ $types.Action
@ $types.Action.$variants.create.name
$type = .string
"#;
    
    let schema = extract_schema_from_value(doc).expect("Failed to extract schema");
    
    // Print the raw type definition structure
    println!("=== Debug: Type definitions ===");
    for (type_name, type_def) in &schema.document_schema.types {
        println!("Type: {type_name:?}");
        match &type_def.type_expr {
            Type::Variants(variant_schema) => {
                println!("  Is a variant type with {} variants", variant_schema.variants.len());
                for (variant_name, variant_obj) in &variant_schema.variants {
                    println!("  Variant: {variant_name:?}");
                    for (field_key, field_schema) in &variant_obj.fields {
                        println!("    Field key: {:?} -> Type: {:?}", field_key, field_schema.type_expr);
                    }
                }
            }
            _ => {
                println!("  Type expression: {:?}", type_def.type_expr);
            }
        }
    }
    
    // Now let's trace through the schema extraction with more detail
    println!("\n=== Tracing variant field extraction ===");
    
    // The issue is that when processing:
    // @ $types.Action.$variants.create.name
    // $type = .string
    //
    // The field is being stored with key Extension("type") instead of String("name")
}