use eure_schema::*;

fn main() {
    let schema_input = r#"
@ $types.Action {
  @ $variants.set-text {
    speaker.$type = .string
    lines.$array = .string
  }
  @ $variants.set-choices {
    description.$type = .string
  }
}

@ script {
  $type = .object
  @ actions {
    $array = .$types.Action
  }
}
"#;

    let schema = extract_schema_from_value(schema_input).expect("Failed to extract schema").document_schema;

    // Simple document with just the first action
    let doc_input = r#"
@ script
@ script.actions[]
$variant: set-text
speaker = "Alice"
lines = ["Hello", "World"]
"#;

    println!("Validating simple document...");
    let errors = validate_with_schema_value(doc_input, schema.clone()).expect("Failed to validate");
    
    println!("Validation errors: {}", errors.len());
    for error in &errors {
        println!("  - {:?}", error);
    }
    
    // Try without the array syntax
    let doc_input2 = r#"
@ script
@ script.actions
$variant: set-text
speaker = "Alice"  
lines = ["Hello", "World"]
"#;

    println!("\n\nValidating without array syntax...");
    let errors2 = validate_with_schema_value(doc_input2, schema).expect("Failed to validate");
    
    println!("Validation errors: {}", errors2.len());
    for error in &errors2 {
        println!("  - {:?}", error);
    }
}