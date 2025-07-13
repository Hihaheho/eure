use eure_schema::{extract_schema_from_value, validate_with_tree};
use eure_tree::value_visitor::ValueVisitor;
use eure_value::value::KeyCmpValue;

#[test]
fn debug_dollar_field_test() {
    let schema = r#"
# Schema with a field that starts with $
$types.Config {
  "$variant" = .string      # This is a field named "$variant"
  name = .string
}

config = .$types.Config
"#;

    let document1 = r#"
# Document with extension field (this would be handled differently in real usage)
@ config
name = "test"
# Missing "$variant" field
"#;

    let extracted = extract_schema_from_value(schema).expect("Failed to extract schema");
    println!("Schema extracted successfully");
    
    // Let's look at the schema fields for Config type
    if let Some(config_type) = extracted.document_schema.types.get(&KeyCmpValue::String("Config".to_string())) {
        println!("Config type: {:#?}", config_type);
    }
    
    // Document 1 should have error - missing required field "$variant"
    let tree1 = eure_parol::parse(document1).expect("Failed to parse document1");
    println!("Document 1 parsed successfully");
    
    // Let's see what the document tree contains
    let mut visitor = ValueVisitor::new(document1);
    tree1.visit_from_root(&mut visitor).expect("Failed to visit tree");
    let doc = visitor.into_document();
    
    println!("Document tree value: {:#?}", doc.to_value());
    
    let errors1 = validate_with_tree(&tree1, document1, extracted.document_schema.clone())
        .expect("Validation failed");
    
    println!("Validation errors: {:?}", errors1);
    
    for error in &errors1 {
        println!("Error: {}", error);
    }
}