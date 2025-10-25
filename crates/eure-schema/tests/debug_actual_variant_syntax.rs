use eure_schema::{Type, document_to_schema, validate_document};
use eure_tree::value_visitor::ValueVisitor;
use eure_value::value::KeyCmpValue;

#[test]
fn test_actual_variant_repr_syntax() {
    // Test what ACTUALLY works based on the schema extraction code

    // Test 1: "internal" string (what the code expects)
    let schema1 = r#"
$types.Event {
  @ $variants.click {
    x = .number
    y = .number
  }
  @ $variants.keypress {
    key = .string
  }
}
$types.Event.$variant-repr = { tag = "type" }

events.$array = .$types.Event
"#;

    println!("Test 1: Using object notation for internally tagged");
    let tree1 = eure_parol::parse(schema1).expect("Failed to parse");
    let mut visitor1 = ValueVisitor::new(schema1);
    tree1
        .visit_from_root(&mut visitor1)
        .expect("Failed to visit");
    let doc1 = visitor1.into_document();
    let schema1 = document_to_schema(&doc1).expect("Failed to extract schema");

    if let Some(event_type) = schema1.types.get(&KeyCmpValue::String("Event".to_string())) {
        if let Type::Variants(variant_schema) = &event_type.type_expr {
            println!("  Result: {:?}", variant_schema.representation);
            // Should be: InternallyTagged { tag: "type" }
        }
    }

    // Now test validation with this schema
    let test_doc = r#"
events = []
@ events[] {
  type = "click"
  x = 100
  y = 200
}
"#;

    let test_tree = eure_parol::parse(test_doc).expect("Failed to parse doc");
    let mut test_visitor = ValueVisitor::new(test_doc);
    test_tree
        .visit_from_root(&mut test_visitor)
        .expect("Failed to visit doc");
    let test_document = test_visitor.into_document();
    let errors = validate_document(&test_document, &schema1);
    println!("  Validation errors: {}", errors.len());
    for error in &errors {
        println!("    - {:?}", error.kind);
    }

    // Test 2: "adjacent" string
    let schema2 = r#"
$types.Message {
  @ $variants.text {
    content = .string
  }
}
$types.Message.$variant-repr = { tag = "kind", content = "data" }

messages.$array = .$types.Message
"#;

    println!("\nTest 2: Using object notation for adjacently tagged");
    let tree2 = eure_parol::parse(schema2).expect("Failed to parse");
    let mut visitor2 = ValueVisitor::new(schema2);
    tree2
        .visit_from_root(&mut visitor2)
        .expect("Failed to visit");
    let doc2 = visitor2.into_document();
    let schema2 = document_to_schema(&doc2).expect("Failed to extract schema");

    if let Some(msg_type) = schema2
        .types
        .get(&KeyCmpValue::String("Message".to_string()))
    {
        if let Type::Variants(variant_schema) = &msg_type.type_expr {
            println!("  Result: {:?}", variant_schema.representation);
            // Should be: AdjacentlyTagged { tag: "type", content: "content" }
        }
    }

    // Test 3: "untagged" string (should work)
    let schema3 = r#"
$types.Value {
  @ $variants.text {
    text = .string
  }
  @ $variants.number {
    value = .number
  }
}
$types.Value.$variant-repr = "untagged"

values.$array = .$types.Value
"#;

    println!("\nTest 3: Using 'untagged' string");
    let tree3 = eure_parol::parse(schema3).expect("Failed to parse");
    let mut visitor3 = ValueVisitor::new(schema3);
    tree3
        .visit_from_root(&mut visitor3)
        .expect("Failed to visit");
    let doc3 = visitor3.into_document();
    let schema3 = document_to_schema(&doc3).expect("Failed to extract schema");

    if let Some(val_type) = schema3.types.get(&KeyCmpValue::String("Value".to_string())) {
        if let Type::Variants(variant_schema) = &val_type.type_expr {
            println!("  Result: {:?}", variant_schema.representation);
            // Should be: Untagged
        }
    }
}
