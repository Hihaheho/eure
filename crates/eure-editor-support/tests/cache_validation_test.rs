//! Tests for document caching functionality during syntax errors

use eure_editor_support::{parser, schema_validation};
use eure_tree::value_visitor::ValueVisitor;

#[test]
fn test_cached_document_validation_with_syntax_error() {
    // First, create a valid document
    let valid_input = r#"name = "Alice"
age = 30"#;

    // Parse the valid document
    let parse_result = parser::parse_document(valid_input);
    let (cst, eure_document) = match parse_result {
        parser::ParseResult::Ok(cst) => {
            // Create EureDocument from valid CST
            let mut visitor = ValueVisitor::new(valid_input);
            cst.visit_from_root(&mut visitor).expect("Failed to visit tree");
            (Some(cst), Some(visitor.into_document()))
        }
        parser::ParseResult::ErrWithCst { .. } => panic!("Got parse error with CST"),
    };
    
    // Create schema manager and load a simple schema
    let schema_input = r#"name.$type = .string
age.$type = .number"#;
    
    let schema_parse_result = parser::parse_document(schema_input);
    let mut schema_manager = schema_validation::SchemaManager::new();
    if let parser::ParseResult::Ok(schema_cst) = schema_parse_result {
        schema_manager
            .load_schema("test://schema", schema_input, &schema_cst)
            .expect("Failed to load schema");
        schema_manager.set_document_schema("test://document", "test://schema");
    }
    
    // Validate with valid document - should have no errors
    let diagnostics = schema_validation::validate_document(
        "test://document",
        valid_input,
        &cst.unwrap(),
        &schema_manager,
        None,
    );
    
    // Should have no validation errors
    assert!(diagnostics.is_empty() || diagnostics.iter().all(|d| {
        match &d.code {
            Some(lsp_types::NumberOrString::String(s)) => s != "eure-schema-type",
            _ => true,
        }
    }));
    
    // Now create an invalid document with syntax error
    let invalid_input = r#"name = "Alice"
age = 30
invalid {
    # Syntax error: missing closing brace"#;
    
    // Parse the invalid document
    let parse_result_invalid = parser::parse_document(invalid_input);
    let invalid_cst = match parse_result_invalid {
        parser::ParseResult::ErrWithCst { cst, .. } => Some(cst),
        _ => panic!("Expected parse error with CST"),
    };
    
    // Validate with cached document - should use the cached valid document
    let diagnostics_with_cache = schema_validation::validate_document(
        "test://document",
        invalid_input,
        &invalid_cst.unwrap(),
        &schema_manager,
        eure_document.as_ref(),
    );
    
    // Should have the "using cached" diagnostic
    let has_cache_diagnostic = diagnostics_with_cache.iter().any(|d| {
        match &d.code {
            Some(lsp_types::NumberOrString::String(s)) => s == "eure-cached-validation",
            _ => false,
        }
    });
    assert!(has_cache_diagnostic, "Expected cache usage diagnostic");
    
    // Should still validate using the cached structure
    let schema_errors = diagnostics_with_cache.iter().filter(|d| {
        d.source.as_ref()
            .map(|s| s == "eure-schema")
            .unwrap_or(false)
            && match &d.code {
                Some(lsp_types::NumberOrString::String(s)) => s != "eure-cached-validation",
                _ => true,
            }
    }).count();
    assert_eq!(schema_errors, 0, "Expected no schema validation errors when using cached document");
}

#[test]
fn test_no_cached_document_fallback() {
    // Test that validation gracefully handles missing cached document
    let invalid_input = r#"
person {
    name = "Alice"
    # Missing closing brace
"#;
    
    let parse_result = parser::parse_document(invalid_input);
    let cst = match parse_result {
        parser::ParseResult::ErrWithCst { cst, .. } => cst,
        _ => panic!("Expected parse error with CST"),
    };
    
    let schema_manager = schema_validation::SchemaManager::new();
    
    // Validate without cached document
    let diagnostics = schema_validation::validate_document(
        "test://document",
        invalid_input,
        &cst,
        &schema_manager,
        None,
    );
    
    // Should not crash, but likely won't produce schema diagnostics
    // due to inability to create EureDocument
    assert!(diagnostics.is_empty() || !diagnostics.iter().any(|d| {
        match &d.code {
            Some(lsp_types::NumberOrString::String(s)) => s == "eure-cached-validation",
            _ => false,
        }
    }));
}