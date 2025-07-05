use eure_editor_support::completions::get_completions;
use eure_editor_support::parser::{self, parse_document};
use eure_editor_support::schema_validation::SchemaManager;
use lsp_types::Position;

#[test]
fn test_completion_after_at_symbol() {
    let text = r#"@"#;
    let position = Position { line: 0, character: 1 };
    
    // Parse the document
    let parse_result = parse_document(text);
    let cst = match parse_result {
        parser::ParseResult::Ok(cst) => cst,
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
    };
    
    // Create a schema manager with a test schema
    let mut schema_manager = SchemaManager::new();
    let schema_text = r#"@ name
$type = .string

@ age
$type = .number

@ active
$type = .boolean"#;
    
    match parse_document(schema_text) {
        parser::ParseResult::Ok(schema_cst) => {
            match schema_manager.load_schema("test://schema", schema_text, &schema_cst) {
                Ok(_) => {
                    eprintln!("Schema loaded successfully");
                    schema_manager.set_document_schema("test://document", "test://schema");
                }
                Err(e) => eprintln!("Failed to load schema: {e}"),
            }
        }
        parser::ParseResult::ErrWithCst { error, .. } => {
            eprintln!("Failed to parse schema: {error:?}");
        }
    }
    
    // Get completions
    let completions = get_completions(
        text,
        &cst,
        position,
        Some("@".to_string()),
        "test://document",
        &schema_manager,
    );
    
    // Should have name, age, active fields
    assert!(!completions.is_empty());
    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    assert!(labels.contains(&"name".to_string()));
    assert!(labels.contains(&"age".to_string()));
    assert!(labels.contains(&"active".to_string()));
}

#[test]
fn test_completion_in_value_position() {
    let text = r#"active = "#;
    let position = Position { line: 0, character: 9 };
    
    // Parse the document
    let parse_result = parse_document(text);
    let cst = match parse_result {
        parser::ParseResult::Ok(cst) => cst,
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
    };
    
    let schema_manager = SchemaManager::new();
    
    // Get completions
    let completions = get_completions(
        text,
        &cst,
        position,
        None,
        "test://document",
        &schema_manager,
    );
    
    // Should have true, false, null
    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    assert!(labels.contains(&"true".to_string()));
    assert!(labels.contains(&"false".to_string()));
    assert!(labels.contains(&"null".to_string()));
}

#[test]
fn test_completion_variant_position() {
    let text = r#"$variant: "#;
    let position = Position { line: 0, character: 9 }; // Position at the colon
    
    // Parse the document
    let parse_result = parse_document(text);
    let cst = match parse_result {
        parser::ParseResult::Ok(cst) => cst,
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
    };
    
    let schema_manager = SchemaManager::new();
    
    // Get completions
    let completions = get_completions(
        text,
        &cst,
        position,
        Some(":".to_string()),
        "test://document",
        &schema_manager,
    );
    
    // Should have variant suggestions
    assert!(!completions.is_empty());
}

#[test]
fn test_string_only_vs_any_value_completion() {
    // Test after ":" - should not get boolean/null completions
    let text = r#"name: "#;
    let position = Position { line: 0, character: 6 };
    
    let parse_result = parse_document(text);
    let cst = match parse_result {
        parser::ParseResult::Ok(cst) => cst,
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
    };
    
    let schema_manager = SchemaManager::new();
    let completions = get_completions(
        text,
        &cst,
        position,
        Some(":".to_string()),
        "test://document",
        &schema_manager,
    );
    
    // Should NOT have boolean/null values after ":"
    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    assert!(!labels.contains(&"true".to_string()));
    assert!(!labels.contains(&"false".to_string()));
    assert!(!labels.contains(&"null".to_string()));
    
    // Test after "=" - should get boolean/null completions
    let text = r#"active = "#;
    let position = Position { line: 0, character: 9 };
    
    let parse_result = parse_document(text);
    let cst = match parse_result {
        parser::ParseResult::Ok(cst) => cst,
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
    };
    
    let completions = get_completions(
        text,
        &cst,
        position,
        Some("=".to_string()),
        "test://document",
        &schema_manager,
    );
    
    // Should have boolean/null values after "="
    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    assert!(labels.contains(&"true".to_string()));
    assert!(labels.contains(&"false".to_string()));
    assert!(labels.contains(&"null".to_string()));
}

#[test]
fn test_section_snippet_generation() {
    // Test that completing fields in a section with $prefer.section = true generates snippets
    let text = r#"user."#;
    let position = Position { line: 0, character: 5 }; // After the dot
    
    let parse_result = parse_document(text);
    let cst = match parse_result {
        parser::ParseResult::Ok(cst) => cst,
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
    };
    
    // Create a schema with section preference
    let mut schema_manager = SchemaManager::new();
    let schema_text = r#"@ user {
    $prefer.section = true
    
    @ name {
        @ first
        $type = .string
        
        @ last  
        $type = .string
        
        @ middle
        $type = .string
        $optional = true
    }
    
    @ age
    $type = .number
}"#;
    
    match parse_document(schema_text) {
        parser::ParseResult::Ok(schema_cst) => {
            match schema_manager.load_schema("test://schema", schema_text, &schema_cst) {
                Ok(_) => {
                    eprintln!("Schema loaded successfully");
                    schema_manager.set_document_schema("test://document", "test://schema");
                }
                Err(e) => eprintln!("Failed to load schema: {e}"),
            }
        }
        parser::ParseResult::ErrWithCst { error, .. } => {
            eprintln!("Failed to parse schema: {error:?}");
        }
    }
    
    // Get completions
    let completions = get_completions(
        text,
        &cst,
        position,
        Some(".".to_string()),
        "test://document",
        &schema_manager,
    );
    
    // Debug: print all completions
    eprintln!("Completions found: {}", completions.len());
    for completion in &completions {
        eprintln!("  - {}", completion.label);
    }
    
    // Find the "name" completion
    let name_completion = completions.iter()
        .find(|c| c.label == "name")
        .expect("Should have 'name' completion");
    
    // Check that it's a snippet
    assert_eq!(name_completion.insert_text_format, Some(lsp_types::InsertTextFormat::SNIPPET));
    
    // Check the snippet content
    let snippet = name_completion.insert_text.as_ref().expect("Should have insert text");
    eprintln!("Generated snippet:\n{snippet}");
    
    // Should include "user.name" and required fields (first, last) but not optional (middle)
    assert!(snippet.contains("user.name"));
    assert!(snippet.contains("first = ${") && snippet.contains(":!}"));
    assert!(snippet.contains("last = ${") && snippet.contains(":!}"));
    assert!(!snippet.contains("middle")); // Optional field should not be included
    assert!(snippet.contains("$0")); // Final cursor position
}

#[test]
fn test_no_snippet_for_non_object_fields() {
    // Test that non-object fields don't generate snippets
    let text = r#"user."#;
    let position = Position { line: 0, character: 5 };
    
    let parse_result = parse_document(text);
    let cst = match parse_result {
        parser::ParseResult::Ok(cst) => cst,
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
    };
    
    let mut schema_manager = SchemaManager::new();
    let schema_text = r#"@ user {
    $prefer.section = true
    
    @ age
    $type = .number
}"#;
    
    match parse_document(schema_text) {
        parser::ParseResult::Ok(schema_cst) => {
            match schema_manager.load_schema("test://schema", schema_text, &schema_cst) {
                Ok(_) => {
                    schema_manager.set_document_schema("test://document", "test://schema");
                }
                Err(e) => eprintln!("Failed to load schema: {e}"),
            }
        }
        parser::ParseResult::ErrWithCst { error, .. } => {
            eprintln!("Failed to parse schema: {error:?}");
        }
    }
    
    let completions = get_completions(
        text,
        &cst,
        position,
        Some(".".to_string()),
        "test://document",
        &schema_manager,
    );
    
    // Find the "age" completion
    let age_completion = completions.iter()
        .find(|c| c.label == "age")
        .expect("Should have 'age' completion");
    
    // Should NOT be a snippet since age is a number, not an object
    assert_eq!(age_completion.insert_text_format, None);
    assert_eq!(age_completion.insert_text, None);
}

#[test]
fn test_completion_with_types() {
    let text = r#"@"#;
    let position = Position { line: 0, character: 1 };
    
    // Parse the document
    let parse_result = parse_document(text);
    let cst = match parse_result {
        parser::ParseResult::Ok(cst) => cst,
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
    };
    
    // Create a schema manager with types
    let mut schema_manager = SchemaManager::new();
    let schema_text = r#"$types.Person {
    @ name
    $type = .string
    
    @ age
    $type = .number
}

@ user
$type = .$types.Person"#;
    
    match parse_document(schema_text) {
        parser::ParseResult::Ok(schema_cst) => {
            match schema_manager.load_schema("test://schema", schema_text, &schema_cst) {
                Ok(_) => {
                    eprintln!("Schema loaded successfully");
                    schema_manager.set_document_schema("test://document", "test://schema");
                }
                Err(e) => eprintln!("Failed to load schema: {e}"),
            }
        }
        parser::ParseResult::ErrWithCst { error, .. } => {
            eprintln!("Failed to parse schema: {error:?}");
        }
    }
    
    // Get completions
    let completions = get_completions(
        text,
        &cst,
        position,
        Some("@".to_string()),
        "test://document",
        &schema_manager,
    );
    
    // Should have user field and $types
    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    assert!(labels.contains(&"user".to_string()));
    assert!(labels.contains(&"$types".to_string()));
}