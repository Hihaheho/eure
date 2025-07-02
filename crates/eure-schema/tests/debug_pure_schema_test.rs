use eure_schema::{extract_schema_from_value, is_pure_schema};
use eure_value::value::KeyCmpValue;

#[test]
fn debug_example_schema() {
    let schema_input = r#"
$schema = "assets/eure-schema.schema.eure"

$types.Action {
  @ $variants.set-text {
    speaker = .string
    lines.$array = .string
  }
}

@ script
id = .string
description = .string
actions.$array = .$types.Action
"#;

    // Extract schema using value-based API
    let extracted = extract_schema_from_value(schema_input)
        .expect("Failed to extract schema");
    
    println!("Is pure schema: {}", extracted.is_pure_schema);
    println!("Root fields: {:?}", extracted.document_schema.root.fields.keys().collect::<Vec<_>>());
    println!("Types: {:?}", extracted.document_schema.types.keys().collect::<Vec<_>>());
    
    if let Some(script_field) = extracted.document_schema.root.fields.get(&KeyCmpValue::String("script".to_string())) {
        println!("Script field type: {:?}", script_field.type_expr);
    }
    
    // The test is expecting this to be a pure schema
    assert!(extracted.is_pure_schema, "Should be detected as a pure schema");
}

#[test]
fn test_simple_pure_schema() {
    let schema_input = r#"
$types.Item {
  text = .string
  value = .number
}

@ config
name = .string
items.$array = .$types.Item
"#;

    let extracted = extract_schema_from_value(schema_input)
        .expect("Failed to extract schema");
    
    println!("Simple schema - is pure: {}", extracted.is_pure_schema);
    println!("Root fields: {:?}", extracted.document_schema.root.fields.keys().collect::<Vec<_>>());
    
    assert!(extracted.is_pure_schema, "Simple schema should be pure");
}

#[test] 
fn test_mixed_content_schema() {
    let schema_input = r#"
@ config
name = "test"
name.$type = .string
"#;

    let extracted = extract_schema_from_value(schema_input)
        .expect("Failed to extract schema");
    
    println!("Mixed content - is pure: {}", extracted.is_pure_schema);
    
    assert!(!extracted.is_pure_schema, "Mixed content should not be pure schema");
}

#[test]
fn test_exact_example_schema_file() {
    // Read the actual file
    let schema_input = std::fs::read_to_string("../../example.schema.eure")
        .expect("Failed to read example.schema.eure");
    
    println!("File content length: {}", schema_input.len());
    println!("File content (hex dump of first 100 chars):");
    for (i, ch) in schema_input.chars().take(100).enumerate() {
        print!("{:02x} ", ch as u32);
        if (i + 1) % 16 == 0 {
            println!();
        }
    }
    println!();
    
    let extracted = extract_schema_from_value(&schema_input)
        .expect("Failed to extract schema");
    
    println!("File schema - is pure: {}", extracted.is_pure_schema);
    println!("Root fields: {:?}", extracted.document_schema.root.fields.keys().collect::<Vec<_>>());
    
    // Debug the last part of the file
    println!("Last 50 chars of file:");
    let last_chars: String = schema_input.chars().rev().take(50).collect::<String>().chars().rev().collect();
    println!("{:?}", last_chars);
}

#[test]
fn test_optional_field_pure_schema() {
    let schema_input = r#"
@ config
name.$type = .string
name.$optional = true
value.$type = .number
"#;

    let extracted = extract_schema_from_value(schema_input)
        .expect("Failed to extract schema");
    
    println!("Schema with $optional - is pure: {}", extracted.is_pure_schema);
    
    // This should be a pure schema
    assert!(extracted.is_pure_schema, "Schema with $optional should be pure");
}

#[test]
fn test_debug_value_structure() {
    use eure_value::value::{Value, KeyCmpValue};
    
    let schema_input = r#"
@ config
name = .string
name.$optional = true
"#;

    // Parse to Value
    let tree = eure_parol::parse(schema_input).expect("Failed to parse");
    let mut values = eure_tree::value_visitor::Values::default();
    let mut visitor = eure_tree::value_visitor::ValueVisitor::new(schema_input, &mut values);
    
    use eure_tree::prelude::*;
    tree.visit_from_root(&mut visitor).expect("Failed to extract values");
    
    let doc_value = if let Ok(root_view) = tree.root_handle().get_view(&tree) {
        values.get_eure(&root_view.eure).expect("No document value found")
    } else {
        panic!("Invalid document structure");
    };
    
    println!("=== Value Structure Debug ===");
    if let Value::Map(map) = doc_value {
        for (key, val) in &map.0 {
            match key {
                KeyCmpValue::String(s) => println!("String key: '{}'", s),
                KeyCmpValue::Extension(s) => println!("Extension key: '${}'", s),
                KeyCmpValue::MetaExtension(s) => println!("MetaExtension key: '$${}'", s),
                _ => println!("Other key type"),
            }
            match val {
                Value::Map(m) => {
                    println!("  -> Map with {} entries", m.0.len());
                    for (k2, v2) in &m.0 {
                        match k2 {
                            KeyCmpValue::String(s) => println!("    String key: '{}'", s),
                            KeyCmpValue::Extension(s) => println!("    Extension key: '${}'", s),
                            KeyCmpValue::MetaExtension(s) => println!("    MetaExtension key: '$${}'", s),
                            _ => println!("    Other key type"),
                        }
                        println!("      -> {:?}", std::mem::discriminant(v2));
                    }
                }
                Value::Path(_) => println!("  -> Path"),
                Value::Bool(b) => println!("  -> Bool: {}", b),
                Value::String(s) => println!("  -> String: {}", s),
                _ => println!("  -> Other value type"),
            }
        }
    }
}