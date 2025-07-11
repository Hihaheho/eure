use eure_editor_support::completions::get_completions;
use lsp_types::{CompletionItem, Position, CompletionItemKind};
use eure_editor_support::schema_validation::SchemaManager;

fn get_completions_with_schema(input: &str, schema_text: &str) -> Vec<CompletionItem> {
    let lines: Vec<&str> = input.lines().collect();
    let line = lines.len().saturating_sub(1);
    let character = lines.last().map(|l| l.len()).unwrap_or(0) as u32;
    
    let position = Position { line: line as u32, character };
    
    // Parse the input
    let parse_result = eure_parol::parse_tolerant(input);
    let cst = parse_result.cst();
    
    // Create schema manager with schema
    let mut schema_manager = SchemaManager::new();
    let schema_parse_result = eure_parol::parse_tolerant(schema_text);
    let schema_cst = schema_parse_result.cst();
    
    schema_manager.load_schema("test://schema", schema_text, &schema_cst).unwrap();
    schema_manager.set_document_schema("test.eure", "test://schema");
    
    // Debug: print the loaded schema
    if let Some(schema) = schema_manager.get_schema("test://schema") {
        eprintln!("DEBUG test: Loaded schema has {} root fields", schema.root.fields.len());
        for (k, _) in &schema.root.fields {
            eprintln!("  - Root field: {:?}", k);
        }
    }
    
    get_completions(
        input,
        &cst,
        position,
        None,
        "test.eure",
        &schema_manager,
    )
}

#[test]
fn test_completion_after_equals() {
    let schema_text = r#"@ key
$type = .any"#;
    
    let input = "key = ";
    let completions = get_completions_with_schema(input, schema_text);
    
    // Should suggest basic value types
    assert!(completions.iter().any(|c| c.label == "true"), "Should suggest 'true'");
    assert!(completions.iter().any(|c| c.label == "false"), "Should suggest 'false'");
    assert!(completions.iter().any(|c| c.label == "null"), "Should suggest 'null'");
}

#[test]
fn test_completion_after_dot() {
    let schema_text = r#"@ script {
    @ name
    $type = .string
    
    @ version
    $type = .string
}"#;
    
    let input = "@ script.";
    let completions = get_completions_with_schema(input, schema_text);
    
    // Should suggest keys that can follow script
    assert!(!completions.is_empty(), "Should have completions after dot");
    assert!(completions.iter().all(|c| c.kind == Some(CompletionItemKind::FIELD)), 
            "All completions should be fields");
    
    // Should have actual fields from schema
    assert!(completions.iter().any(|c| c.label == "name"), "Should suggest 'name'");
    assert!(completions.iter().any(|c| c.label == "version"), "Should suggest 'version'");
    
    // Should not have dummy fields
    assert!(!completions.iter().any(|c| c.label == "field1" || c.label == "field2"), 
            "Should not have dummy fields");
}

#[test] 
fn test_completion_in_nested_section() {
    let schema_text = r#"@ a {
    @ b {
        @ c {
            @ key
            $type = .any
        }
    }
}"#;
    
    let input = r#"@ a.b.c
key = "#;
    let completions = get_completions_with_schema(input, schema_text);
    
    // Should suggest values in the context of a.b.c
    assert!(!completions.is_empty(), "Should have value completions");
    assert!(completions.iter().any(|c| c.label == "true"));
}

#[test]
fn test_partial_key_completion() {
    let schema_text = r#"@ script
$type = .string

@ screen
$type = .number

@ settings
$type = .object"#;
    
    let input = "@ scr";
    let completions = get_completions_with_schema(input, schema_text);
    
    // Should suggest keys starting with "scr"
    assert!(!completions.is_empty(), "Should have completions for partial key");
    assert!(completions.iter().any(|c| c.label == "script"), "Should suggest 'script'");
    assert!(completions.iter().any(|c| c.label == "screen"), "Should suggest 'screen'");
    assert!(!completions.iter().any(|c| c.label == "settings"), "Should not suggest 'settings' (doesn't start with 'scr')");
    assert!(completions.iter().all(|c| c.label.starts_with("scr")), "All completions should start with 'scr'");
}