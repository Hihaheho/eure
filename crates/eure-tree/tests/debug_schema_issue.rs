use eure_tree::{value_visitor::ValueVisitor, document::{NodeValue, DocumentKey}};
use eure_value::identifier::Identifier;
use std::str::FromStr;

#[test]
fn debug_schema_parsing_issue() {
    let input = r#"@ config {
    @ environment
    $type = .string
}"#;

    println!("Input:\n{}", input);
    println!("\n=== Parsing ===");
    
    // Parse the EURE document
    let parsed = eure_parol::parse(input).expect("Failed to parse");
    println!("Parse successful!");
    
    // Create the value visitor
    let mut visitor = ValueVisitor::new(input);
    
    // Visit the tree
    match parsed.visit_from_root(&mut visitor) {
        Ok(_) => {
            println!("\n=== Visitor Success ===");
            let document = visitor.into_document();
            
            // Check the document structure
            let root = document.get_root();
            println!("\nRoot node: {:?}", root);
            
            // Check config node
            if let NodeValue::Map { entries, .. } = &root.content {
                if let Some((_, config_id)) = entries.iter().find(|(k, _)| {
                    matches!(k, DocumentKey::Ident(id) if id.to_string() == "config")
                }) {
                    let config_node = document.get_node(*config_id);
                    println!("\nConfig node: {:?}", config_node);
                    println!("Config extensions: {:?}", config_node.extensions);
                    
                    // Check environment node
                    if let NodeValue::Map { entries, .. } = &config_node.content {
                        if let Some((_, env_id)) = entries.iter().find(|(k, _)| {
                            matches!(k, DocumentKey::Ident(id) if id.to_string() == "environment")
                        }) {
                            let env_node = document.get_node(*env_id);
                            println!("\nEnvironment node: {:?}", env_node);
                            println!("Environment extensions: {:?}", env_node.extensions);
                            
                            // Check if $type extension exists
                            let type_ext = Identifier::from_str("type").unwrap();
                            if let Some(type_node_id) = env_node.extensions.get(&type_ext) {
                                let type_node = document.get_node(*type_node_id);
                                println!("\n$type extension node: {:?}", type_node);
                            } else {
                                println!("\n$type extension not found in environment node!");
                            }
                        }
                    }
                }
            }
            
            let value = document.to_value();
            println!("\nFinal value: {:#?}", value);
        }
        Err(e) => {
            println!("\n=== Visitor Error ===");
            println!("Error: {}", e);
            
            // Check the document state before error
            println!("\n=== Document State ===");
            let document = visitor.into_document();
            let root = document.get_root();
            println!("Root node: {:?}", root);
        }
    }
}