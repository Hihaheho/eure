use eure_editor_support::schema_validation::{SchemaManager, validate_document};

#[test]
fn test_diagnostic_span_excludes_whitespace() {
    // Schema that expects a number
    let schema_text = r#"
@ value
$type = .number
"#;

    // Document with invalid type (string instead of number)
    // Note the indentation and newlines
    let document = r#"
@ value
key1 = "valid"
key2 = "invalid type"
key3 = 123
"#;

    // Parse schema
    let schema_parse_result = eure_parol::parse_tolerant(schema_text);
    let schema_cst = schema_parse_result.cst();

    // Create schema manager and load schema
    let mut schema_manager = SchemaManager::new();
    schema_manager.load_schema("test://schema", schema_text, &schema_cst).unwrap();
    schema_manager.set_document_schema("test.eure", "test://schema");

    // Parse document
    let parse_result = eure_parol::parse_tolerant(document);
    let cst = parse_result.cst();

    // Validate and get diagnostics
    let diagnostics = validate_document("test.eure", document, &cst, &schema_manager);

    // Find diagnostic for key2
    let key2_diagnostic = diagnostics.iter()
        .find(|d| d.message.contains("key2") || d.message.contains("unexpected field"))
        .expect("Should have diagnostic for key2");

    // Extract the text covered by the diagnostic span
    let start_offset = position_to_offset(document, key2_diagnostic.range.start);
    let end_offset = position_to_offset(document, key2_diagnostic.range.end);
    let span_text = &document[start_offset..end_offset];

    println!("Diagnostic span text: {span_text:?}");

    // The span should not include leading whitespace or newlines
    assert!(!span_text.starts_with('\n'), "Span should not start with newline");
    assert!(!span_text.starts_with(' '), "Span should not start with spaces");
    assert!(!span_text.starts_with('\t'), "Span should not start with tabs");

    // The span should start with the actual key
    assert!(span_text.starts_with("key2"), "Span should start with the key name");
}

fn position_to_offset(text: &str, pos: lsp_types::Position) -> usize {
    let mut offset = 0;
    for (line_no, line) in text.lines().enumerate() {
        if line_no < pos.line as usize {
            offset += line.len() + 1; // +1 for newline
        } else if line_no == pos.line as usize {
            offset += pos.character.min(line.len() as u32) as usize;
            break;
        }
    }
    offset
}

#[test]
fn test_value_span_excludes_whitespace() {
    // Schema expecting number type
    let schema_text = r#"
@ myfield
$type = .number
"#;

    // Document with string value (type mismatch)
    let document = r#"@ myfield

  value = "string instead of number"
"#;

    // Parse and validate
    let schema_parse_result = eure_parol::parse_tolerant(schema_text);
    let schema_cst = schema_parse_result.cst();

    let mut schema_manager = SchemaManager::new();
    schema_manager.load_schema("test://schema", schema_text, &schema_cst).unwrap();
    schema_manager.set_document_schema("test.eure", "test://schema");

    let parse_result = eure_parol::parse_tolerant(document);
    let cst = parse_result.cst();

    let diagnostics = validate_document("test.eure", document, &cst, &schema_manager);

    // Should have a type mismatch diagnostic
    let type_diagnostic = diagnostics.iter()
        .find(|d| d.message.contains("type") || d.message.contains("expected"))
        .expect("Should have type mismatch diagnostic");

    // Extract span text
    let start_offset = position_to_offset(document, type_diagnostic.range.start);
    let end_offset = position_to_offset(document, type_diagnostic.range.end);
    let span_text = &document[start_offset..end_offset];

    println!("Type error span text: {span_text:?}");

    // The span should focus on the value, not include preceding whitespace
    assert!(!span_text.starts_with('\n'), "Span should not start with newline");
    assert!(!span_text.starts_with(' '), "Span should not start with spaces");
}
