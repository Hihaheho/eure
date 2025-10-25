use eure_schema::*;

fn main() {
    // Test 1: Type definition with section
    let schema1 = r#"
@ $types.Item {
  text.$type = .string  
  value.$type = .string
}
"#;

    println!("Test 1: Type definition extraction");
    let extracted1 = extract_schema_from_value(schema1)
        .expect("Failed to extract schema")
        .document_schema;

    println!("Types: {:?}", extracted1.types.keys().collect::<Vec<_>>());
    println!(
        "Root fields: {:?}",
        extracted1.root.fields.keys().collect::<Vec<_>>()
    );

    if let Some(item) = extracted1
        .types
        .get(&eure_value::value::KeyCmpValue::String("Item".to_string()))
    {
        println!("Item type: {:?}", item.type_expr);
        if let Type::Object(obj) = &item.type_expr {
            println!("Item fields: {:?}", obj.fields.keys().collect::<Vec<_>>());
        }
    }

    // Test 2: Same but without section syntax
    let schema2 = r#"
@ $types.Item
text.$type = .string
value.$type = .string
"#;

    println!("\n\nTest 2: Type definition without braces");
    let extracted2 = extract_schema_from_value(schema2)
        .expect("Failed to extract schema")
        .document_schema;

    println!("Types: {:?}", extracted2.types.keys().collect::<Vec<_>>());
    println!(
        "Root fields: {:?}",
        extracted2.root.fields.keys().collect::<Vec<_>>()
    );

    if let Some(item) = extracted2
        .types
        .get(&eure_value::value::KeyCmpValue::String("Item".to_string()))
    {
        println!("Item type: {:?}", item.type_expr);
        if let Type::Object(obj) = &item.type_expr {
            println!("Item fields: {:?}", obj.fields.keys().collect::<Vec<_>>());
        }
    }

    return;

    // Simple array test
    let schema_input = r#"
@ $types.Item {
  text.$type = .string
  value.$type = .string
}

@ items {
  $array = .$types.Item
}
"#;

    let schema = extract_schema_from_value(schema_input)
        .expect("Failed to extract schema")
        .document_schema;

    println!(
        "Schema types: {:?}",
        schema.types.keys().collect::<Vec<_>>()
    );
    println!(
        "Root fields: {:?}",
        schema.root.fields.keys().collect::<Vec<_>>()
    );

    if let Some(item_type) = schema
        .types
        .get(&eure_value::value::KeyCmpValue::String("Item".to_string()))
    {
        println!("Item type: {:?}", item_type.type_expr);
        if let Type::Object(obj) = &item_type.type_expr {
            println!("Item fields: {:?}", obj.fields.keys().collect::<Vec<_>>());
        }
    }

    // Document with inline array
    let doc_input = r#"
items = [
  { text = "Option A", value = "a" },
  { text = "Option B", value = "b" }
]
"#;

    println!("Validating document with inline array...");
    let errors = validate_with_schema_value(doc_input, schema).expect("Failed to validate");

    println!("Validation errors: {}", errors.len());
    for error in &errors {
        println!("  - {:?}", error);
    }
}
