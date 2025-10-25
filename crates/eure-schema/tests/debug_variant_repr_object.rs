use eure_schema::{Type, document_to_schema};
use eure_tree::value_visitor::ValueVisitor;
use eure_value::value::KeyCmpValue;

#[test]
fn test_variant_repr_object_notation() {
    // Test 1: Internally tagged with object notation (as per docs)
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

    println!("Testing internally tagged with object notation: {{ tag = \"type\" }}");

    let schema_tree = eure_parol::parse(schema_input).expect("Failed to parse schema");
    let mut schema_visitor = ValueVisitor::new(schema_input);
    schema_tree
        .visit_from_root(&mut schema_visitor)
        .expect("Failed to visit schema tree");
    let schema_doc = schema_visitor.into_document();

    // This will likely fail or default to Tagged because the extraction
    // only handles string values, not objects
    let schema = document_to_schema(&schema_doc).expect("Failed to extract schema");

    if let Some(event_type) = schema.types.get(&KeyCmpValue::String("Event".to_string()))
        && let Type::Variants(variant_schema) = &event_type.type_expr
    {
        println!(
            "Extracted representation: {:?}",
            variant_schema.representation
        );
        // Expected: InternallyTagged { tag: "type" }
        // Actual: Probably Tagged (default)
    }

    // Test 2: Adjacently tagged with object notation
    let schema_input2 = r#"
$types.Message {
  $variant-repr = { tag = "kind", content = "data" }
  @ $variants.text {
    content = .string
  }
  @ $variants.image {
    url = .string
  }
}

messages.$array = .$types.Message
"#;

    println!(
        "\nTesting adjacently tagged with object notation: {{ tag = \"kind\", content = \"data\" }}"
    );

    let schema_tree2 = eure_parol::parse(schema_input2).expect("Failed to parse schema");
    let mut schema_visitor2 = ValueVisitor::new(schema_input2);
    schema_tree2
        .visit_from_root(&mut schema_visitor2)
        .expect("Failed to visit schema tree");
    let schema_doc2 = schema_visitor2.into_document();
    let schema2 = document_to_schema(&schema_doc2).expect("Failed to extract schema");

    if let Some(message_type) = schema2
        .types
        .get(&KeyCmpValue::String("Message".to_string()))
        && let Type::Variants(variant_schema) = &message_type.type_expr
    {
        println!(
            "Extracted representation: {:?}",
            variant_schema.representation
        );
        // Expected: AdjacentlyTagged { tag: "kind", content: "data" }
        // Actual: Probably Tagged (default)
    }

    // Test 3: String notation (should work)
    let schema_input3 = r#"
$types.Value {
  $variant-repr = "untagged"
  @ $variants.text {
    text = .string
  }
  @ $variants.number {
    value = .number
  }
}

values.$array = .$types.Value
"#;

    println!("\nTesting untagged with string notation: \"untagged\"");

    let schema_tree3 = eure_parol::parse(schema_input3).expect("Failed to parse schema");
    let mut schema_visitor3 = ValueVisitor::new(schema_input3);
    schema_tree3
        .visit_from_root(&mut schema_visitor3)
        .expect("Failed to visit schema tree");
    let schema_doc3 = schema_visitor3.into_document();
    let schema3 = document_to_schema(&schema_doc3).expect("Failed to extract schema");

    if let Some(value_type) = schema3.types.get(&KeyCmpValue::String("Value".to_string()))
        && let Type::Variants(variant_schema) = &value_type.type_expr
    {
        println!(
            "Extracted representation: {:?}",
            variant_schema.representation
        );
        // Expected: Untagged
        // Actual: Should work since it's a string
    }
}
