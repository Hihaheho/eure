use eure_editor_support::completions::get_completions;
use eure_editor_support::schema_validation::SchemaManager;
use lsp_types::{CompletionItemKind, Position};

#[test]
fn test_completion_suggests_all_root_fields() {
    // Schema with multiple root-level fields
    let schema_text = r#"
@ script
$type = .string

@ name
$type = .string

@ version
$type = .number

@ settings {
    @ debug
    $type = .boolean
    
    @ timeout
    $type = .number
}

@ users
$array = .string
"#;

    // Test completion at root level
    let input = "@ ";
    let position = Position {
        line: 0,
        character: 2,
    }; // After "@ "

    let parse_result = eure_parol::parse_tolerant(input);
    let cst = parse_result.cst();

    // Create schema manager with schema
    let mut schema_manager = SchemaManager::new();
    let schema_parse_result = eure_parol::parse_tolerant(schema_text);
    let schema_cst = schema_parse_result.cst();

    schema_manager
        .load_schema("test://schema", schema_text, &schema_cst)
        .unwrap();
    schema_manager.set_document_schema("test.eure", "test://schema");

    let completions = get_completions(input, &cst, position, None, "test.eure", &schema_manager);

    // Should suggest all root-level fields
    assert!(
        !completions.is_empty(),
        "Should have completions for root fields"
    );

    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    assert!(
        labels.contains(&"script".to_string()),
        "Should suggest 'script'"
    );
    assert!(
        labels.contains(&"name".to_string()),
        "Should suggest 'name'"
    );
    assert!(
        labels.contains(&"version".to_string()),
        "Should suggest 'version'"
    );
    assert!(
        labels.contains(&"settings".to_string()),
        "Should suggest 'settings'"
    );
    assert!(
        labels.contains(&"users".to_string()),
        "Should suggest 'users'"
    );

    // All should be fields
    assert!(
        completions
            .iter()
            .all(|c| c.kind == Some(CompletionItemKind::FIELD)),
        "All completions should be fields"
    );
}

#[test]
fn test_partial_completion_filters_correctly() {
    // Schema with multiple root-level fields
    let schema_text = r#"
@ script
$type = .string

@ screen
$type = .string

@ settings
$type = .object

@ users
$type = .array
"#;

    // Test completion with partial text
    let input = "@ scr";
    let position = Position {
        line: 0,
        character: 5,
    }; // After "scr"

    let parse_result = eure_parol::parse_tolerant(input);
    let cst = parse_result.cst();

    // Create schema manager with schema
    let mut schema_manager = SchemaManager::new();
    let schema_parse_result = eure_parol::parse_tolerant(schema_text);
    let schema_cst = schema_parse_result.cst();

    schema_manager
        .load_schema("test://schema", schema_text, &schema_cst)
        .unwrap();
    schema_manager.set_document_schema("test.eure", "test://schema");

    let completions = get_completions(input, &cst, position, None, "test.eure", &schema_manager);

    // Should only suggest fields starting with "scr"
    assert!(
        !completions.is_empty(),
        "Should have completions for 'scr' prefix"
    );

    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    assert!(
        labels.contains(&"script".to_string()),
        "Should suggest 'script'"
    );
    assert!(
        labels.contains(&"screen".to_string()),
        "Should suggest 'screen'"
    );
    assert!(
        !labels.contains(&"settings".to_string()),
        "Should not suggest 'settings'"
    );
    assert!(
        !labels.contains(&"users".to_string()),
        "Should not suggest 'users'"
    );

    // Verify all start with "scr"
    assert!(
        labels.iter().all(|l| l.starts_with("scr")),
        "All completions should start with 'scr'"
    );
}
