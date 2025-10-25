use eure_schema::{Type, VariantSchema, document_to_schema, validate_document};
use eure_tree::value_visitor::ValueVisitor;
use eure_value::value::KeyCmpValue;

#[test]
fn debug_internally_tagged_variant() {
    // Test internally tagged - schema extraction defaults to "type" as tag field
    let schema_input = r#"
$types.Event {
  $variant-repr = { tag = "type" }
  @ $variants.click {
    x = .number
    y = .number
  }
  @ $variants.keypress {
    key = .string
  }
}

events.$array = .$types.Event
"#;

    let schema_tree = eure_parol::parse(schema_input).expect("Failed to parse schema");
    let mut schema_visitor = ValueVisitor::new(schema_input);
    schema_tree
        .visit_from_root(&mut schema_visitor)
        .expect("Failed to visit schema tree");
    let schema_doc = schema_visitor.into_document();
    let schema = document_to_schema(&schema_doc).expect("Failed to extract schema");

    // Check what tag field the schema is expecting
    if let Some(event_type) = schema.types.get(&KeyCmpValue::String("Event".to_string())) {
        if let Type::Variants(variant_schema) = &event_type.type_expr {
            println!(
                "Variant representation: {:?}",
                variant_schema.representation
            );
            // This should show InternallyTagged { tag: "type" }
        }
    }

    // Test document with "type" field (should work with hardcoded default)
    let doc1 = r#"
events = []
@ events[] {
  type = "click"
  x = 100
  y = 200
}
"#;

    let tree1 = eure_parol::parse(doc1).expect("Failed to parse document");
    let mut visitor1 = ValueVisitor::new(doc1);
    tree1
        .visit_from_root(&mut visitor1)
        .expect("Failed to visit tree");
    let document1 = visitor1.into_document();
    let errors1 = validate_document(&document1, &schema);

    println!("\nDocument with 'type' field:");
    for error in &errors1 {
        println!("  Error: {:?}", error);
    }

    // The error shows VariantDiscriminatorMissing because the schema extraction
    // sets the representation to InternallyTagged { tag: "type" } but then
    // the detection is failing
}
