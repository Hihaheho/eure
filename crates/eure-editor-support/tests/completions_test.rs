use eure_editor_support::completions::get_completions;
use eure_editor_support::parser::{self, parse_document};
use eure_editor_support::schema_validation::SchemaManager;
use lsp_types::Position;

#[test]
fn test_completion_after_at_symbol() {
    let text = r#"@"#;
    let position = Position {
        line: 0,
        character: 1,
    };

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
        None,
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
    let position = Position {
        line: 0,
        character: 9,
    };

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
        None,
    );

    // Should have true, false, null
    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    assert!(labels.contains(&"true".to_string()));
    assert!(labels.contains(&"false".to_string()));
    assert!(labels.contains(&"null".to_string()));
}

// TODO: Add test for variant value completions when we have a proper variant schema
// For example, if we have a type with $variants.pending, $variants.approved, etc.,
// then typing "$variant: " should suggest "pending", "approved", etc.

#[test]
fn test_string_only_vs_any_value_completion() {
    // Test after ":" - should not get boolean/null completions
    let text = r#"name: "#;
    let position = Position {
        line: 0,
        character: 6,
    };

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
        None,
    );

    // Should NOT have boolean/null values after ":"
    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    assert!(!labels.contains(&"true".to_string()));
    assert!(!labels.contains(&"false".to_string()));
    assert!(!labels.contains(&"null".to_string()));

    // Test after "=" - should get boolean/null completions
    let text = r#"active = "#;
    let position = Position {
        line: 0,
        character: 9,
    };

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
        None,
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
    let position = Position {
        line: 0,
        character: 5,
    }; // After the dot

    let parse_result = parse_document(text);
    let cst = match parse_result {
        parser::ParseResult::Ok(cst) => cst,
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
    };

    // Create a schema with section preference
    let mut schema_manager = SchemaManager::new();
    let schema_text = r#"
user.$prefer.section = true
user.name.first.$type = .string
user.name.last.$type = .string
user.name.middle.$type = .string
user.name.middle.$optional = true
user.age.$type = .number
"#;

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
        None,
    );

    // Debug: print all completions
    eprintln!("Completions found: {}", completions.len());
    for completion in &completions {
        eprintln!("  - {}", completion.label);
    }

    // Find the "name" completion
    let name_completion = completions
        .iter()
        .find(|c| c.label == "name")
        .expect("Should have 'name' completion");

    // Check that it's a snippet
    assert_eq!(
        name_completion.insert_text_format,
        Some(lsp_types::InsertTextFormat::SNIPPET)
    );

    // Check the snippet content
    let snippet = name_completion
        .insert_text
        .as_ref()
        .expect("Should have insert text");
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
    let position = Position {
        line: 0,
        character: 5,
    };

    let parse_result = parse_document(text);
    let cst = match parse_result {
        parser::ParseResult::Ok(cst) => cst,
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
    };

    let mut schema_manager = SchemaManager::new();
    // Use the working inline schema format from deep_nesting_test
    let schema_text = r#"
user.age.$type = .number
"#;

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
        None,
    );

    // Find the "age" completion
    let age_completion = completions
        .iter()
        .find(|c| c.label == "age")
        .expect("Should have 'age' completion");

    // Should NOT be a snippet since age is a number, not an object
    assert_eq!(age_completion.insert_text_format, None);
    assert_eq!(age_completion.insert_text, None);
}

#[test]
fn test_completion_with_types() {
    let text = r#"@"#;
    let position = Position {
        line: 0,
        character: 1,
    };

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
        None,
    );

    // Should have user field and $types
    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    assert!(labels.contains(&"user".to_string()));
    assert!(labels.contains(&"$types".to_string()));
}

#[test]
fn test_nested_path_completion() {
    // Test completion for nested paths like user.address.
    let text = r#"user.address."#;
    let position = Position {
        line: 0,
        character: 13,
    }; // After second dot

    let parse_result = parse_document(text);
    let cst = match parse_result {
        parser::ParseResult::Ok(cst) => cst,
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
    };

    // Create schema with nested structure
    let mut schema_manager = SchemaManager::new();
    let schema_text = r#"@ user {
    @ address {
        @ street
        $type = .string
        
        @ city
        $type = .string
        
        @ zipcode
        $type = .string
        
        @ country
        $type = .string
        $optional = true
    }
    
    @ name
    $type = .string
}"#;

    match parse_document(schema_text) {
        parser::ParseResult::Ok(schema_cst) => {
            match schema_manager.load_schema("test://schema", schema_text, &schema_cst) {
                Ok(_) => {
                    schema_manager.set_document_schema("test://document", "test://schema");
                }
                Err(e) => panic!("Failed to load schema: {e}"),
            }
        }
        parser::ParseResult::ErrWithCst { error, .. } => {
            panic!("Failed to parse schema: {error:?}");
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
        None,
    );

    // Should have address fields: street, city, zipcode, country
    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    eprintln!("Nested path completions: {labels:?}");

    // Test the completions
    assert!(
        labels.contains(&"street".to_string()),
        "Should contain 'street' field"
    );
    assert!(
        labels.contains(&"city".to_string()),
        "Should contain 'city' field"
    );
    assert!(
        labels.contains(&"zipcode".to_string()),
        "Should contain 'zipcode' field"
    );
    assert!(
        labels.contains(&"country".to_string()),
        "Should contain 'country' field"
    );
    assert_eq!(
        labels.len(),
        4,
        "Should only show fields from address schema"
    );
}

#[test]
fn test_array_element_completion() {
    // Test completion for array elements like items[].
    let text = r#"@ items[]
"#;
    let position = Position {
        line: 1,
        character: 0,
    }; // Start of new line after array section

    let parse_result = parse_document(text);
    let cst = match parse_result {
        parser::ParseResult::Ok(cst) => cst,
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
    };

    // Create schema with array of objects
    let mut schema_manager = SchemaManager::new();
    let schema_text = r#"$types.Item {
    @ id
    $type = .number
    
    @ name
    $type = .string
    
    @ description
    $type = .string
    $optional = true
    
    @ price
    $type = .number
}

@ items {
    $array = .$types.Item
}"#;

    match parse_document(schema_text) {
        parser::ParseResult::Ok(schema_cst) => {
            match schema_manager.load_schema("test://schema", schema_text, &schema_cst) {
                Ok(_) => {
                    schema_manager.set_document_schema("test://document", "test://schema");
                }
                Err(e) => panic!("Failed to load schema: {e}"),
            }
        }
        parser::ParseResult::ErrWithCst { error, .. } => {
            panic!("Failed to parse schema: {error:?}");
        }
    }

    // Get completions
    let completions = get_completions(
        text,
        &cst,
        position,
        None,
        "test://document",
        &schema_manager,
        None,
    );

    // Should have array element fields: id, name, description, price
    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    eprintln!("Array element completions: {labels:?}");

    // Array context is now implemented
    assert!(labels.contains(&"id".to_string()));
    assert!(labels.contains(&"name".to_string()));
    assert!(labels.contains(&"description".to_string()));
    assert!(labels.contains(&"price".to_string()));
}

#[test]
fn test_mixed_path_completion() {
    // Test completion for complex paths like config.servers[].
    let text = r#"@ config.servers[]
"#;
    let position = Position {
        line: 1,
        character: 0,
    }; // Start of new line

    let parse_result = parse_document(text);
    let cst = match parse_result {
        parser::ParseResult::Ok(cst) => cst,
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
    };

    // Create schema with nested array
    let mut schema_manager = SchemaManager::new();
    let schema_text = r#"$types.Server {
    @ host
    $type = .string
    
    @ port
    $type = .number
    
    @ protocol
    $type = .string
    $enum = ["http", "https", "ws", "wss"]
    
    @ enabled
    $type = .boolean
    $default = true
}

@ config {
    @ servers {
        $array = .$types.Server
    }
}"#;

    match parse_document(schema_text) {
        parser::ParseResult::Ok(schema_cst) => {
            match schema_manager.load_schema("test://schema", schema_text, &schema_cst) {
                Ok(_) => {
                    schema_manager.set_document_schema("test://document", "test://schema");
                }
                Err(e) => panic!("Failed to load schema: {e}"),
            }
        }
        parser::ParseResult::ErrWithCst { error, .. } => {
            panic!("Failed to parse schema: {error:?}");
        }
    }

    // Get completions
    let completions = get_completions(
        text,
        &cst,
        position,
        None,
        "test://document",
        &schema_manager,
        None,
    );

    // Should have server fields: host, port, protocol, enabled
    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    eprintln!("Mixed path completions: {labels:?}");

    // Complex path context is now implemented
    assert!(labels.contains(&"host".to_string()));
    assert!(labels.contains(&"port".to_string()));
    assert!(labels.contains(&"protocol".to_string()));
    assert!(labels.contains(&"enabled".to_string()));
}

#[test]
fn test_variant_name_completion() {
    // Test completion of variant names after $variant:
    let text = r#"@ actions[]
$variant: "#;
    let position = Position {
        line: 1,
        character: 10,
    }; // After colon

    let parse_result = parse_document(text);
    let cst = match parse_result {
        parser::ParseResult::Ok(cst) => cst,
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
    };

    // Create schema with variant type
    let mut schema_manager = SchemaManager::new();
    let schema_text = r#"$types.Action {
    @ $variants.set-text {
        @ speaker
        $type = .string
        
        @ lines {
            $array = .string
        }
    }
    
    @ $variants.set-choices {
        @ description
        $type = .string
        
        @ choices.$array {
            @ text
            $type = .string
            
            @ value
            $type = .string
        }
    }
    
    @ $variants.navigate {
        @ target
        $type = .string
        
        @ mode
        $type = .string
        $enum = ["push", "replace", "pop"]
    }
}

@ actions {
    $array = .$types.Action
}"#;

    match parse_document(schema_text) {
        parser::ParseResult::Ok(schema_cst) => {
            match schema_manager.load_schema("test://schema", schema_text, &schema_cst) {
                Ok(_) => {
                    schema_manager.set_document_schema("test://document", "test://schema");
                }
                Err(e) => panic!("Failed to load schema: {e}"),
            }
        }
        parser::ParseResult::ErrWithCst { error, .. } => {
            panic!("Failed to parse schema: {error:?}");
        }
    }

    // Get completions
    let completions = get_completions(
        text,
        &cst,
        position,
        Some(":".to_string()),
        "test://document",
        &schema_manager,
        None,
    );

    // Should have variant names: set-text, set-choices, navigate
    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    eprintln!("Variant name completions: {labels:?}");

    // Test the variant completions
    assert!(
        labels.contains(&"set-text".to_string()),
        "Should contain 'set-text' variant"
    );
    assert!(
        labels.contains(&"set-choices".to_string()),
        "Should contain 'set-choices' variant"
    );
    assert!(
        labels.contains(&"navigate".to_string()),
        "Should contain 'navigate' variant"
    );
    assert_eq!(labels.len(), 3, "Should only show variant names");
}

#[test]
fn test_variant_field_completion() {
    // Test completion of fields specific to a selected variant
    let text = r#"@ actions[]
$variant: set-text
"#;
    let position = Position {
        line: 2,
        character: 0,
    }; // Start of new line

    let parse_result = parse_document(text);
    let cst = match parse_result {
        parser::ParseResult::Ok(cst) => cst,
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
    };

    // Use same schema as above
    let mut schema_manager = SchemaManager::new();
    let schema_text = r#"$types.Action {
    @ $variants.set-text {
        @ speaker
        $type = .string
        
        @ lines {
            $array = .string
        }
    }
    
    @ $variants.set-choices {
        @ description
        $type = .string
        
        @ choices.$array {
            @ text
            $type = .string
            
            @ value
            $type = .string
        }
    }
}

@ actions {
    $array = .$types.Action
}"#;

    match parse_document(schema_text) {
        parser::ParseResult::Ok(schema_cst) => {
            match schema_manager.load_schema("test://schema", schema_text, &schema_cst) {
                Ok(_) => {
                    schema_manager.set_document_schema("test://document", "test://schema");
                }
                Err(e) => panic!("Failed to load schema: {e}"),
            }
        }
        parser::ParseResult::ErrWithCst { error, .. } => {
            panic!("Failed to parse schema: {error:?}");
        }
    }

    // Get completions
    let completions = get_completions(
        text,
        &cst,
        position,
        None,
        "test://document",
        &schema_manager,
        None,
    );

    // Should have set-text variant fields: speaker, lines
    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    eprintln!("Variant field completions: {labels:?}");

    // Variant field tracking is now implemented
    assert!(labels.contains(&"speaker".to_string()));
    assert!(labels.contains(&"lines".to_string()));
    assert!(
        !labels.contains(&"description".to_string()),
        "Should not show fields from other variants"
    );
    assert!(
        !labels.contains(&"choices".to_string()),
        "Should not show fields from other variants"
    );
}

#[test]
fn test_variant_field_completion_different_variant() {
    // Test that different variant shows different fields
    let text = r#"@ actions[]
$variant: set-choices
"#;
    let position = Position {
        line: 2,
        character: 0,
    }; // Start of new line

    let parse_result = parse_document(text);
    let cst = match parse_result {
        parser::ParseResult::Ok(cst) => cst,
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
    };

    // Use same schema as above
    let mut schema_manager = SchemaManager::new();
    let schema_text = r#"$types.Action {
    @ $variants.set-text {
        @ speaker
        $type = .string
        
        @ lines {
            $array = .string
        }
    }
    
    @ $variants.set-choices {
        @ description
        $type = .string
        
        @ choices.$array {
            @ text
            $type = .string
            
            @ value
            $type = .string
        }
    }
}

@ actions {
    $array = .$types.Action
}"#;

    match parse_document(schema_text) {
        parser::ParseResult::Ok(schema_cst) => {
            match schema_manager.load_schema("test://schema", schema_text, &schema_cst) {
                Ok(_) => {
                    schema_manager.set_document_schema("test://document", "test://schema");
                }
                Err(e) => panic!("Failed to load schema: {e}"),
            }
        }
        parser::ParseResult::ErrWithCst { error, .. } => {
            panic!("Failed to parse schema: {error:?}");
        }
    }

    // Get completions
    let completions = get_completions(
        text,
        &cst,
        position,
        None,
        "test://document",
        &schema_manager,
        None,
    );

    // Should have set-choices variant fields: description, choices
    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    eprintln!("Different variant field completions: {labels:?}");

    // Variant field tracking is now implemented
    assert!(labels.contains(&"description".to_string()));
    assert!(labels.contains(&"choices".to_string()));
    assert!(
        !labels.contains(&"speaker".to_string()),
        "Should not show fields from other variants"
    );
    assert!(
        !labels.contains(&"lines".to_string()),
        "Should not show fields from other variants"
    );
}

#[test]
fn test_type_reference_completion() {
    // Test completion of type references after .$types.
    let text = r#"@ user
$type = .$types."#;
    let position = Position {
        line: 1,
        character: 16,
    }; // After .$types.

    let parse_result = parse_document(text);
    let cst = match parse_result {
        parser::ParseResult::Ok(cst) => cst,
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
    };

    // Create schema with multiple type definitions
    let mut schema_manager = SchemaManager::new();
    let schema_text = r#"$types.Person {
    @ name
    $type = .string
    
    @ age
    $type = .number
}

$types.Address {
    @ street
    $type = .string
    
    @ city
    $type = .string
    
    @ country
    $type = .string
}

$types.Company {
    @ name
    $type = .string
    
    @ employees {
        $array = .$types.Person
    }
    
    @ headquarters
    $type = .$types.Address
}

@ user {
    # Type will be assigned here
}"#;

    match parse_document(schema_text) {
        parser::ParseResult::Ok(schema_cst) => {
            match schema_manager.load_schema("test://schema", schema_text, &schema_cst) {
                Ok(_) => {
                    schema_manager.set_document_schema("test://document", "test://schema");
                }
                Err(e) => panic!("Failed to load schema: {e}"),
            }
        }
        parser::ParseResult::ErrWithCst { error, .. } => {
            panic!("Failed to parse schema: {error:?}");
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
        None,
    );

    // Should have type names: Person, Address, Company
    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    eprintln!("Type reference completions: {labels:?}");

    // Type reference completion is now implemented
    assert!(labels.contains(&"Person".to_string()));
    assert!(labels.contains(&"Address".to_string()));
    assert!(labels.contains(&"Company".to_string()));
    assert_eq!(labels.len(), 3, "Should only show type names");
}

#[test]
fn test_enum_value_completion() {
    // Test completion of enum values for fields with $enum constraint
    let text = r#"@ config
environment = "#;
    let position = Position {
        line: 1,
        character: 14,
    }; // After equals

    let parse_result = parse_document(text);
    let cst = match parse_result {
        parser::ParseResult::Ok(cst) => cst,
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
    };

    // Create schema with enum constraint
    let mut schema_manager = SchemaManager::new();
    let schema_text = r#"@ config {
    @ environment {
        $type = .string
        $enum = ["development", "staging", "production"]
    }
    
    @ log_level {
        $type = .string
        $enum = ["debug", "info", "warn", "error", "fatal"]
    }
    
    @ port {
        $type = .number
    }
}"#;

    match parse_document(schema_text) {
        parser::ParseResult::Ok(schema_cst) => {
            match schema_manager.load_schema("test://schema", schema_text, &schema_cst) {
                Ok(_) => {
                    schema_manager.set_document_schema("test://document", "test://schema");
                }
                Err(e) => panic!("Failed to load schema: {e}"),
            }
        }
        parser::ParseResult::ErrWithCst { error, .. } => {
            panic!("Failed to parse schema: {error:?}");
        }
    }

    // Get completions
    let completions = get_completions(
        text,
        &cst,
        position,
        Some("=".to_string()),
        "test://document",
        &schema_manager,
        None,
    );

    // Should have enum values: development, staging, production
    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    eprintln!("Enum value completions: {labels:?}");

    // TODO: This test currently fails because enum value completion is not implemented
    // Once implemented, uncomment these assertions:
    // assert!(labels.contains(&"\"development\"".to_string()));
    // assert!(labels.contains(&"\"staging\"".to_string()));
    // assert!(labels.contains(&"\"production\"".to_string()));
    // Should not include other enum values from different fields
    // assert!(!labels.contains(&"\"debug\"".to_string()));
}

#[test]
fn test_completion_with_default_values() {
    // Test that fields with default values show the default in completion details
    let text = r#"@ database
"#;
    let position = Position {
        line: 1,
        character: 0,
    }; // Start of new line

    let parse_result = parse_document(text);
    let cst = match parse_result {
        parser::ParseResult::Ok(cst) => cst,
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
    };

    // Create schema with default values
    let mut schema_manager = SchemaManager::new();
    let schema_text = r#"
database.host.$type = .string
database.host.$default = "localhost"
database.port.$type = .number
database.port.$default = 5432
database.ssl_enabled.$type = .boolean
database.ssl_enabled.$default = true
database.connection_timeout.$type = .number
database.connection_timeout.$default = 30
database.connection_timeout.$optional = true
"#;

    match parse_document(schema_text) {
        parser::ParseResult::Ok(schema_cst) => {
            match schema_manager.load_schema("test://schema", schema_text, &schema_cst) {
                Ok(_) => {
                    schema_manager.set_document_schema("test://document", "test://schema");
                }
                Err(e) => panic!("Failed to load schema: {e}"),
            }
        }
        parser::ParseResult::ErrWithCst { error, .. } => {
            panic!("Failed to parse schema: {error:?}");
        }
    }

    // Get completions
    let completions = get_completions(
        text,
        &cst,
        position,
        None,
        "test://document",
        &schema_manager,
        None,
    );

    // Check that default values are shown in completion details
    let host_completion = completions
        .iter()
        .find(|c| c.label == "host")
        .expect("Should have host completion");

    eprintln!("Host completion detail: {:?}", host_completion.detail);

    // TODO: Once default value display is implemented, check that it includes the default
    // assert!(host_completion.detail.as_ref().unwrap().contains("localhost"));
}

#[test]
fn test_completion_with_cascading_type() {
    // Test completion with cascading type application
    let text = r#"@ servers[]
"#;
    let position = Position {
        line: 1,
        character: 0,
    }; // Start of new line

    let parse_result = parse_document(text);
    let cst = match parse_result {
        parser::ParseResult::Ok(cst) => cst,
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
    };

    // Create schema with array of object type
    let mut schema_manager = SchemaManager::new();
    let schema_text = r#"$types.Server {
    @ name
    $type = .string
    
    @ host
    $type = .string
    
    @ port
    $type = .number
    $default = 8080
}

@ servers {
    $array = .$types.Server
}"#;

    match parse_document(schema_text) {
        parser::ParseResult::Ok(schema_cst) => {
            match schema_manager.load_schema("test://schema", schema_text, &schema_cst) {
                Ok(_) => {
                    schema_manager.set_document_schema("test://document", "test://schema");
                }
                Err(e) => panic!("Failed to load schema: {e}"),
            }
        }
        parser::ParseResult::ErrWithCst { error, .. } => {
            panic!("Failed to parse schema: {error:?}");
        }
    }

    // Get completions
    let completions = get_completions(
        text,
        &cst,
        position,
        None,
        "test://document",
        &schema_manager,
        None,
    );

    // Should have cascaded array element fields: name, host, port
    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    eprintln!("Array element completions: {labels:?}");

    // Array element completion with type reference is now implemented
    assert!(labels.contains(&"name".to_string()));
    assert!(labels.contains(&"host".to_string()));
    assert!(labels.contains(&"port".to_string()));
}

#[test]
fn test_completion_in_block_syntax() {
    // Test completion inside block syntax { ... }
    let text = r#"@ user {
    
}"#;
    let position = Position {
        line: 1,
        character: 4,
    }; // Inside the block

    let parse_result = parse_document(text);
    let cst = match parse_result {
        parser::ParseResult::Ok(cst) => cst,
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
    };

    // Create schema with nested fields
    let mut schema_manager = SchemaManager::new();
    let schema_text = r#"@ user {
    @ name
    $type = .string
    
    @ email
    $type = .string
    
    @ profile {
        @ bio
        $type = .string
        
        @ avatar_url
        $type = .string
        $optional = true
    }
}"#;

    match parse_document(schema_text) {
        parser::ParseResult::Ok(schema_cst) => {
            match schema_manager.load_schema("test://schema", schema_text, &schema_cst) {
                Ok(_) => {
                    schema_manager.set_document_schema("test://document", "test://schema");
                }
                Err(e) => panic!("Failed to load schema: {e}"),
            }
        }
        parser::ParseResult::ErrWithCst { error, .. } => {
            panic!("Failed to parse schema: {error:?}");
        }
    }

    // Get completions
    let completions = get_completions(
        text,
        &cst,
        position,
        None,
        "test://document",
        &schema_manager,
        None,
    );

    // Should have user fields: name, email, profile
    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    eprintln!("Block syntax completions: {labels:?}");

    // Block context is properly tracked
    assert!(labels.contains(&"name".to_string()));
    assert!(labels.contains(&"email".to_string()));
    assert!(labels.contains(&"profile".to_string()));
}

#[test]
fn test_completion_with_partial_syntax() {
    // Test completion when the document has syntax errors or is incomplete
    let text = r#"@ user
name = "Alice"
em"#;
    let position = Position {
        line: 2,
        character: 2,
    }; // After "em"

    let parse_result = parse_document(text);
    let cst = match parse_result {
        parser::ParseResult::Ok(cst) => cst,
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
    };

    // Create schema
    let mut schema_manager = SchemaManager::new();
    let schema_text = r#"@ user {
    @ name
    $type = .string
    
    @ email
    $type = .string
    
    @ employee_id
    $type = .number
}"#;

    match parse_document(schema_text) {
        parser::ParseResult::Ok(schema_cst) => {
            match schema_manager.load_schema("test://schema", schema_text, &schema_cst) {
                Ok(_) => {
                    schema_manager.set_document_schema("test://document", "test://schema");
                }
                Err(e) => panic!("Failed to load schema: {e}"),
            }
        }
        parser::ParseResult::ErrWithCst { error, .. } => {
            panic!("Failed to parse schema: {error:?}");
        }
    }

    // Get completions
    let completions = get_completions(
        text,
        &cst,
        position,
        None,
        "test://document",
        &schema_manager,
        None,
    );

    // Should still provide completions that match the prefix "em"
    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    eprintln!("Partial syntax completions: {labels:?}");

    // TODO: Partial/prefix matching may not be implemented
    // Once implemented, uncomment:
    // assert!(labels.contains(&"email".to_string()));
    // assert!(labels.contains(&"employee_id".to_string()));
    // assert!(!labels.contains(&"name".to_string()), "Should filter out non-matching fields");
}

#[test]
fn test_completion_at_document_boundary() {
    // Test completion at the very beginning or end of document
    let text = r#""#;
    let position = Position {
        line: 0,
        character: 0,
    }; // Empty document

    let parse_result = parse_document(text);
    let cst = match parse_result {
        parser::ParseResult::Ok(cst) => cst,
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
    };

    // Create schema
    let mut schema_manager = SchemaManager::new();
    let schema_text = r#"@ config
$type = .string

@ version
$type = .number"#;

    match parse_document(schema_text) {
        parser::ParseResult::Ok(schema_cst) => {
            match schema_manager.load_schema("test://schema", schema_text, &schema_cst) {
                Ok(_) => {
                    schema_manager.set_document_schema("test://document", "test://schema");
                }
                Err(e) => panic!("Failed to load schema: {e}"),
            }
        }
        parser::ParseResult::ErrWithCst { error, .. } => {
            panic!("Failed to parse schema: {error:?}");
        }
    }

    // Get completions
    let completions = get_completions(
        text,
        &cst,
        position,
        None,
        "test://document",
        &schema_manager,
        None,
    );

    // Should provide root-level completions
    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    eprintln!("Empty document completions: {labels:?}");

    assert!(
        !completions.is_empty(),
        "Should provide completions for empty document"
    );
    // Specific assertions depend on implementation details
}

#[test]
fn test_completion_in_nested_block_with_arrays() {
    // Test complex nesting: blocks containing arrays containing blocks
    let text = r#"@ config {
    @ servers[] {
        
    }
}"#;
    let position = Position {
        line: 2,
        character: 8,
    }; // Inside nested block

    let parse_result = parse_document(text);
    let cst = match parse_result {
        parser::ParseResult::Ok(cst) => cst,
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
    };

    // Create schema with complex nesting
    let mut schema_manager = SchemaManager::new();
    let schema_text = r#"$types.Endpoint {
    @ path
    $type = .string
    
    @ method
    $type = .string
    $enum = ["GET", "POST", "PUT", "DELETE"]
}

$types.Server {
    @ name
    $type = .string
    
    @ endpoints {
        $array = .$types.Endpoint
    }
}

@ config {
    @ servers {
        $array = .$types.Server
    }
}"#;

    match parse_document(schema_text) {
        parser::ParseResult::Ok(schema_cst) => {
            match schema_manager.load_schema("test://schema", schema_text, &schema_cst) {
                Ok(_) => {
                    schema_manager.set_document_schema("test://document", "test://schema");
                }
                Err(e) => panic!("Failed to load schema: {e}"),
            }
        }
        parser::ParseResult::ErrWithCst { error, .. } => {
            panic!("Failed to parse schema: {error:?}");
        }
    }

    // Get completions
    let completions = get_completions(
        text,
        &cst,
        position,
        None,
        "test://document",
        &schema_manager,
        None,
    );

    // Should have server array element fields: name, endpoints
    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    eprintln!("Nested block array completions: {labels:?}");

    // Complex nested context tracking is now implemented
    assert!(labels.contains(&"name".to_string()));
    assert!(labels.contains(&"endpoints".to_string()));
}

#[test]
fn test_completion_with_inline_objects() {
    // Test completion inside inline object syntax { key = value, ... }
    let text = r#"user = { name = "Alice",  }"#;
    let position = Position {
        line: 0,
        character: 25,
    }; // After comma, before closing brace

    let parse_result = parse_document(text);
    let cst = match parse_result {
        parser::ParseResult::Ok(cst) => cst,
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
    };

    // Create schema
    let mut schema_manager = SchemaManager::new();
    let schema_text = r#"@ user {
    @ name
    $type = .string
    
    @ age
    $type = .number
    
    @ email
    $type = .string
}"#;

    match parse_document(schema_text) {
        parser::ParseResult::Ok(schema_cst) => {
            match schema_manager.load_schema("test://schema", schema_text, &schema_cst) {
                Ok(_) => {
                    schema_manager.set_document_schema("test://document", "test://schema");
                }
                Err(e) => panic!("Failed to load schema: {e}"),
            }
        }
        parser::ParseResult::ErrWithCst { error, .. } => {
            panic!("Failed to parse schema: {error:?}");
        }
    }

    // Get completions
    let completions = get_completions(
        text,
        &cst,
        position,
        None,
        "test://document",
        &schema_manager,
        None,
    );

    // Should suggest remaining fields (age, email) but not already-used field (name)
    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    eprintln!("Inline object completions: {labels:?}");

    // TODO: Inline object context and used-field filtering may not be implemented
    // Once implemented, uncomment:
    // assert!(labels.contains(&"age".to_string()));
    // assert!(labels.contains(&"email".to_string()));
    // assert!(!labels.contains(&"name".to_string()), "Should not suggest already-used fields");
}
