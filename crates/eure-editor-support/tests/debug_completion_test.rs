use eure_editor_support::completions::get_completions;
use eure_editor_support::parser::{self, parse_document};
use eure_editor_support::schema_validation::SchemaManager;
use lsp_types::Position;

#[test]
fn test_nested_section_completion() {
    let schema_text = r#"@ script {
    @ name
    $type = .string
    
    @ version
    $type = .string
    
    @ dependencies {
        @ npm
        $type = .string
        
        @ cargo
        $type = .string
    }
}

@ database {
    @ host
    $type = .string
    
    @ port
    $type = .number
}"#;

    // Test 1: Completion inside nested section
    let text = r#"@ script.dependencies
"#;
    let position = Position { line: 1, character: 0 }; // Empty line after section header
    
    let parse_result = parse_document(text);
    let cst = match parse_result {
        parser::ParseResult::Ok(cst) => cst,
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
    };
    
    let mut schema_manager = SchemaManager::new();
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
    
    let completions = get_completions(
        text,
        &cst,
        position,
        None,
        "test://document",
        &schema_manager,
    );
    
    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    eprintln!("Nested section completions: {:?}", labels);
    
    assert!(labels.contains(&"npm".to_string()), "Should suggest npm");
    assert!(labels.contains(&"cargo".to_string()), "Should suggest cargo");
    assert!(!labels.contains(&"script".to_string()), "Should NOT suggest script (root field)");
}

#[test]
fn test_at_field_dot_completion() {
    let schema_text = r#"@ script {
    @ name
    $type = .string
    
    @ dependencies {
        @ npm
        $type = .string
    }
}"#;

    // Test 2: Completion after "@ script."
    let text = r#"@ script."#;
    let position = Position { line: 0, character: 9 }; // After the dot
    
    let parse_result = parse_document(text);
    let cst = match parse_result {
        parser::ParseResult::Ok(cst) => cst,
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
    };
    
    let mut schema_manager = SchemaManager::new();
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
    
    let completions = get_completions(
        text,
        &cst,
        position,
        Some(".".to_string()),
        "test://document",
        &schema_manager,
    );
    
    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    eprintln!("@ script. completions: {:?}", labels);
    
    assert!(labels.contains(&"name".to_string()), "Should suggest name");
    assert!(labels.contains(&"dependencies".to_string()), "Should suggest dependencies");
}