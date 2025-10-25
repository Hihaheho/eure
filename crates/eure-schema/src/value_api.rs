//! Value-based API for schema extraction and validation
//!
//! This module provides convenience functions that work with string input
//! and handle the parsing and document conversion internally.

use crate::document_schema::{document_to_schema, is_pure_schema_node};
use crate::document_validator::{ValidationError, validate_document};
use crate::error::ValueError;
use crate::schema::*;
use eure_tree::value_visitor::ValueVisitor;

/// Result of schema extraction from a value
#[derive(Debug)]
pub struct ExtractedSchema {
    pub document_schema: DocumentSchema,
    pub is_pure_schema: bool,
}

/// Result of validation, including the extracted schema
#[derive(Debug)]
pub struct ValidationResult {
    pub schema: ExtractedSchema,
    pub errors: Vec<ValidationError>,
}

/// Extract schema from a string value
pub fn extract_schema_from_value(input: &str) -> Result<ExtractedSchema, ValueError> {
    // Parse the input
    let tree = eure_parol::parse(input)?;

    // Create visitor and visit the tree
    let mut visitor = ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor)?;

    // Get the document
    let document = visitor.into_document();

    // Check if it's a pure schema
    let is_pure_schema = is_pure_schema_node(&document, document.get_root());

    // Extract schema
    let document_schema = document_to_schema(&document)?;

    Ok(ExtractedSchema {
        document_schema,
        is_pure_schema,
    })
}

/// Validate a string value against a schema
pub fn validate_with_schema_value(
    input: &str,
    schema: DocumentSchema,
) -> Result<Vec<ValidationError>, ValueError> {
    // Parse the input
    let tree = eure_parol::parse(input)?;

    // Create visitor and visit the tree
    let mut visitor = ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor)?;

    // Get the document
    let document = visitor.into_document();

    // Validate
    Ok(validate_document(&document, &schema))
}

/// Validate a document that contains its own schema reference
pub fn validate_self_describing(input: &str) -> Result<ValidationResult, ValueError> {
    // Parse the input
    let tree = eure_parol::parse(input)?;

    // Create visitor and visit the tree
    let mut visitor = ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor)?;

    // Get the document
    let document = visitor.into_document();

    // Check if it's a pure schema
    let is_pure_schema = is_pure_schema_node(&document, document.get_root());

    // Extract schema from the document itself
    let document_schema = document_to_schema(&document)?;

    // Validate
    let errors = validate_document(&document, &document_schema);

    Ok(ValidationResult {
        schema: ExtractedSchema {
            document_schema,
            is_pure_schema,
        },
        errors,
    })
}

/// Extract schema and validate a document in one step
///
/// This function combines schema extraction and validation, which is useful
/// for documents that contain both schema definitions and data that should
/// be validated against that schema.
pub fn validate_and_extract_schema(input: &str) -> Result<ValidationResult, ValueError> {
    // Parse the input
    let tree = eure_parol::parse(input)?;

    // Create visitor and visit the tree
    let mut visitor = ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor)?;

    // Get the document
    let document = visitor.into_document();

    // Check if it's a pure schema
    let is_pure_schema = is_pure_schema_node(&document, document.get_root());

    // Extract schema from the document
    let document_schema = document_to_schema(&document)?;

    // Validate the document against its own schema
    let errors = validate_document(&document, &document_schema);

    Ok(ValidationResult {
        schema: ExtractedSchema {
            document_schema,
            is_pure_schema,
        },
        errors,
    })
}

/// Validate using a tree (for compatibility)
pub fn validate_with_tree(
    tree: &eure_tree::Cst,
    input: &str,
    schema: DocumentSchema,
) -> Result<Vec<ValidationError>, ValueError> {
    // Create visitor and visit the tree
    let mut visitor = ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor)?;

    // Get the document
    let document = visitor.into_document();

    // Validate
    Ok(validate_document(&document, &schema))
}
