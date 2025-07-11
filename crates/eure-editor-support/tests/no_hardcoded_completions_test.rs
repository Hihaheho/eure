use eure_editor_support::completions::get_completions;
use lsp_types::Position;
use eure_editor_support::schema_validation::SchemaManager;

#[test]
fn test_no_hardcoded_completions_without_schema() {
    // Test that without a schema, we don't get hardcoded completions
    let input = "@ script.";
    let position = Position { line: 0, character: 9 }; // After the dot
    
    let parse_result = eure_parol::parse_tolerant(input);
    let cst = parse_result.cst();
    
    // Empty schema manager - no schema loaded
    let schema_manager = SchemaManager::new();
    
    let completions = get_completions(
        input,
        &cst,
        position,
        Some(".".to_string()),
        "test.eure",
        &schema_manager, None,
    );
    
    // Should not have any hardcoded values
    assert!(completions.is_empty(), "Should not have completions without schema");
    assert!(!completions.iter().any(|c| c.label == "commands"), "Should not have hardcoded 'commands'");
    assert!(!completions.iter().any(|c| c.label == "options"), "Should not have hardcoded 'options'");
    assert!(!completions.iter().any(|c| c.label == "field1"), "Should not have hardcoded 'field1'");
    assert!(!completions.iter().any(|c| c.label == "field2"), "Should not have hardcoded 'field2'");
}

#[test]
fn test_completions_come_from_schema() {
    // Test that completions come from the schema
    let schema_text = r#"@ myfield {
    @ subfield1
    $type = .string
    
    @ subfield2
    $type = .number
}"#;

    let input = "@ myfield.";
    let position = Position { line: 0, character: 10 }; // After the dot
    
    let parse_result = eure_parol::parse_tolerant(input);
    let cst = parse_result.cst();
    
    let mut schema_manager = SchemaManager::new();
    let schema_parse_result = eure_parol::parse_tolerant(schema_text);
    let schema_cst = schema_parse_result.cst();
    
    // Load the schema
    schema_manager.load_schema("test://schema", schema_text, &schema_cst).unwrap();
    schema_manager.set_document_schema("test.eure", "test://schema");
    
    let completions = get_completions(
        input,
        &cst,
        position,
        Some(".".to_string()),
        "test.eure",
        &schema_manager, None,
    );
    
    // Should have completions from schema
    assert_eq!(completions.len(), 2, "Should have 2 completions from schema");
    
    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    assert!(labels.contains(&"subfield1".to_string()), "Should have subfield1 from schema");
    assert!(labels.contains(&"subfield2".to_string()), "Should have subfield2 from schema");
    
    // Should not have any hardcoded values
    assert!(!labels.contains(&"commands".to_string()), "Should not have hardcoded 'commands'");
    assert!(!labels.contains(&"options".to_string()), "Should not have hardcoded 'options'");
}

#[test]
fn test_partial_key_without_schema_returns_empty() {
    // Test that partial key completion without schema returns empty
    let input = "@ scr";
    let position = Position { line: 0, character: 5 }; // After 'scr'
    
    let parse_result = eure_parol::parse_tolerant(input);
    let cst = parse_result.cst();
    
    // Empty schema manager - no schema loaded
    let schema_manager = SchemaManager::new();
    
    let completions = get_completions(
        input,
        &cst,
        position,
        None,
        "test.eure",
        &schema_manager, None,
    );
    
    // Should not have any completions without schema
    assert!(completions.is_empty(), "Should not have completions without schema");
    assert!(!completions.iter().any(|c| c.label == "script"), "Should not have hardcoded 'script'");
}