use eure_schema::extract_schema_from_value;

#[test]
fn debug_schema_extraction() {
    let input = r#"@ config {
    @ environment
    $type = .string
}"#;

    println!("Input:\n{}", input);
    
    match extract_schema_from_value(input) {
        Ok(schema) => {
            println!("\n=== Schema Extraction Success ===");
            println!("Is pure schema: {}", schema.is_pure_schema);
            println!("Schema: {:#?}", schema.document_schema);
        }
        Err(e) => {
            println!("\n=== Schema Extraction Error ===");
            println!("Error: {}", e);
            
            // Try to get more details
            if let Some(tree_err) = e.downcast_ref::<eure_tree::value_visitor::ValueVisitorError>() {
                println!("ValueVisitor error: {:?}", tree_err);
            }
        }
    }
}