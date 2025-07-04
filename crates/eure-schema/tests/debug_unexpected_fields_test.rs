//! Debug test to understand why unexpected fields aren't being reported

use eure_schema::{extract_schema_from_value, KeyCmpValue};
use eure_tree::prelude::*;
use eure_value::value::PathSegment;

#[test]
fn debug_unexpected_fields_in_example() {
    println!("\n=== DEBUG: Unexpected Fields Test ===\n");
    
    // Load the schema
    let schema_input = include_str!("../../../example.schema.eure");
    println!("Schema file contents:");
    for (i, line) in schema_input.lines().enumerate() {
        println!("{:3}: {}", i + 1, line);
    }
    
    let extracted = extract_schema_from_value(schema_input)
        .expect("Failed to extract schema from example.schema.eure");
    
    println!("\n\nExtracted schema root fields:");
    for (key, field) in &extracted.document_schema.root.fields {
        println!("  - {:?}: type={:?}, optional={}", 
            key, 
            field.type_expr,
            field.optional
        );
    }
    
    // Load and parse the document
    let doc_input = include_str!("../../../example.eure");
    println!("\n\nDocument file contents (relevant parts):");
    for (i, line) in doc_input.lines().enumerate() {
        if i < 15 || line.trim().starts_with("@") || !line.trim().is_empty() && i < 20 {
            println!("{:3}: {}", i + 1, line);
        }
    }
    
    let tree = eure_parol::parse(doc_input).expect("Failed to parse example.eure");
    
    // Extract values using the visitor pattern
    let mut values = eure_tree::value_visitor::Values::default();
    let mut value_visitor = eure_tree::value_visitor::ValueVisitor::new(doc_input, &mut values);
    
    // Visit from root
    tree.visit_from_root(&mut value_visitor).expect("Failed to extract values");
    
    // Create debug validator
    let mut validator = DebugValidator {
        _input: doc_input,
        schema: &extracted.document_schema,
        values: &values,
        errors: Vec::new(),
        current_path: Vec::new(),
        seen_fields: std::collections::HashSet::new(),
    };
    
    println!("\n\nStarting validation walk...");
    tree.visit_from_root(&mut validator).expect("Failed to validate");
    validator.finalize();
    
    // Analyze results
    println!("\n\n=== RESULTS ===");
    println!("Total errors found: {}", validator.errors.len());
    
    let unexpected_field_errors: Vec<_> = validator.errors.iter()
        .filter(|e| matches!(&e.kind, 
            eure_schema::ValidationErrorKind::UnexpectedField { .. }
        ))
        .collect();
        
    println!("\nUnexpected field errors: {}", unexpected_field_errors.len());
    for error in &unexpected_field_errors {
        if let eure_schema::ValidationErrorKind::UnexpectedField { field, path } = &error.kind {
            println!("  - Field: {:?}, Path: {:?}", field, path);
        }
    }
    
    // We expect to find 'aaa' and 'text' as unexpected fields
    let has_aaa_error = unexpected_field_errors.iter().any(|e| {
        matches!(&e.kind, 
            eure_schema::ValidationErrorKind::UnexpectedField { field, .. } 
            if matches!(field, KeyCmpValue::String(s) if s == "aaa")
        )
    });
    
    let has_text_error = unexpected_field_errors.iter().any(|e| {
        matches!(&e.kind, 
            eure_schema::ValidationErrorKind::UnexpectedField { field, .. } 
            if matches!(field, KeyCmpValue::String(s) if s == "text")
        )
    });
    
    println!("\nExpected unexpected fields:");
    println!("  - 'aaa' field reported: {}", has_aaa_error);
    println!("  - 'text' field reported: {}", has_text_error);
    
    // These assertions will fail, showing us the issue
    assert!(has_aaa_error, "Field 'aaa' should be reported as unexpected");
    assert!(has_text_error, "Field 'text' should be reported as unexpected");
}

// Simplified debug validator
struct DebugValidator<'a> {
    _input: &'a str,
    schema: &'a eure_schema::DocumentSchema,
    values: &'a eure_tree::value_visitor::Values,
    errors: Vec<eure_schema::ValidationError>,
    current_path: Vec<PathSegment>,
    seen_fields: std::collections::HashSet<KeyCmpValue>,
}

impl<'a> DebugValidator<'a> {
    fn finalize(&mut self) {
        // Check for missing required fields
        for (key, field_schema) in &self.schema.root.fields {
            if !field_schema.optional && !self.seen_fields.contains(key) {
                self.errors.push(eure_schema::ValidationError {
                    kind: eure_schema::ValidationErrorKind::RequiredFieldMissing {
                        field: key.clone(),
                        path: vec![],
                    },
                    severity: eure_schema::Severity::Error,
                    span: None,
                });
            }
        }
    }
}

impl<'a, F: CstFacade> CstVisitor<F> for DebugValidator<'a> {
    type Error = std::convert::Infallible;
    
    fn visit_binding(
        &mut self,
        handle: BindingHandle,
        view: BindingView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        if let Some(key_handles) = self.values.get_keys(&view.keys) {
            // Build path
            let mut path = Vec::new();
            for key_handle in key_handles {
                if let Some(segment) = self.values.get_path_segment(key_handle) {
                    path.push(segment.clone());
                }
            }
            
            println!("\n  Binding: {:?}", path);
            
            // Check if this is a root-level field
            if path.len() == 1 && self.current_path.is_empty() {
                if let PathSegment::Ident(ident) = &path[0] {
                    let key = KeyCmpValue::String(ident.as_ref().to_string());
                    println!("    -> Root field: {:?}", key);
                    
                    // Mark as seen
                    self.seen_fields.insert(key.clone());
                    
                    // Check if it exists in schema
                    if self.schema.root.fields.contains_key(&key) {
                        println!("    -> Found in schema");
                    } else {
                        // Check if it's an extension field
                        let is_extension = path.iter().any(|seg| matches!(seg, PathSegment::Extension(_)));
                        
                        if !is_extension {
                            println!("    -> NOT in schema! Should report as unexpected");
                            self.errors.push(eure_schema::ValidationError {
                                kind: eure_schema::ValidationErrorKind::UnexpectedField {
                                    field: key,
                                    path: self.current_path.clone(),
                                },
                                severity: eure_schema::Severity::Error,
                                span: None,
                            });
                        }
                    }
                }
            }
        }
        
        self.visit_binding_super(handle, view, tree)
    }
    
    fn visit_section(
        &mut self,
        handle: SectionHandle,
        view: SectionView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        if let Some(key_handles) = self.values.get_keys(&view.keys) {
            let mut path = Vec::new();
            for key_handle in key_handles {
                if let Some(segment) = self.values.get_path_segment(key_handle) {
                    path.push(segment.clone());
                }
            }
            
            println!("\n  Section: {:?}", path);
            
            // Track root-level sections
            if path.len() == 1 && self.current_path.is_empty() {
                if let PathSegment::Ident(ident) = &path[0] {
                    let key = KeyCmpValue::String(ident.as_ref().to_string());
                    println!("    -> Root section: {:?}", key);
                    self.seen_fields.insert(key);
                }
            }
        }
        
        self.visit_section_super(handle, view, tree)
    }
}