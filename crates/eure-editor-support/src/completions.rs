use eure_schema::{DocumentSchema, KeyCmpValue, Type, ObjectSchema, FieldSchema};
use eure_value::value::{PathSegment, Value as EureValue};
use eure_value::identifier::Identifier;
use eure_tree::{Cst, value_visitor::Values};
use lsp_types::{CompletionItem, CompletionItemKind, Documentation, InsertTextFormat, MarkupContent, MarkupKind, Position};
use indexmap::IndexMap;
use std::str::FromStr;

use crate::schema_validation::SchemaManager;
use crate::path_context::PathContextExtractor;
use crate::completion_analyzer::CompletionAnalyzer;

#[derive(Debug, Clone)]
pub struct CompletionContext {
    pub position: Position,
    pub trigger_character: Option<String>,
    pub current_path: Vec<String>,
    pub path_segments: Vec<PathSegment>,
    pub variant_context: Option<String>,
    pub is_in_value_position: bool,
    pub is_in_key_position: bool,
    pub is_variant_position: bool,
    pub is_string_only: bool, // True after ":", false after "="
    pub parent_path: Option<String>, // e.g., "user" when completing "user."
    pub partial_field: Option<String>, // Partial field name being typed
}

pub fn get_completions(
    text: &str,
    cst: &Cst,
    position: Position,
    trigger_character: Option<String>,
    uri: &str,
    schema_manager: &SchemaManager,
) -> Vec<CompletionItem> {
    // First try error-based completion
    let parse_result = eure_parol::parse_tolerant(text);
    let analyzer = CompletionAnalyzer::new(
        text.to_string(),
        parse_result,
        position,
        schema_manager,
        uri,
    );
    
    let error_completions = analyzer.analyze();
    if !error_completions.is_empty() {
        return error_completions;
    }
    
    // Fall back to original logic
    let context = analyze_context(text, cst, position, trigger_character.clone());
    
    // Get the schema for this document
    let schema_uri = schema_manager.get_document_schema_uri(uri);
    let schema = schema_uri.and_then(|uri| schema_manager.get_schema(uri));
    
    if let Some(schema) = schema {
        let mut completions = generate_completions(&context, schema);
        
        // Filter by partial field if present
        if let Some(partial) = &context.partial_field {
            completions.retain(|c| c.label.starts_with(partial));
        }
        
        completions
    } else {
        // No schema available, but still provide basic completions for value positions
        if context.is_in_value_position {
            generate_value_completions(&context)
        } else {
            vec![]
        }
    }
}

fn analyze_context(
    text: &str,
    cst: &Cst,
    position: Position,
    trigger_character: Option<String>,
) -> CompletionContext {
    // Convert position to byte offset
    let _byte_offset = position_to_byte_offset(text, position);
    
    // eprintln!("analyze_context: trigger_character = {:?}", trigger_character);
    
    // Get the line at the cursor position
    let lines: Vec<&str> = text.lines().collect();
    let num_lines = if text.ends_with('\n') { lines.len() + 1 } else { lines.len() };
    // eprintln!("Text has {} lines (including trailing newline), cursor at line {}", num_lines, position.line);
    
    // Handle cursor on empty line after last line with content
    let (_current_line, line_before_cursor) = if position.line as usize >= lines.len() {
        if (position.line as usize) < num_lines {
            // We're on the empty line after the last line
            ("", "")
        } else {
            return CompletionContext {
                position,
                trigger_character: trigger_character.clone(),
                current_path: vec![],
                path_segments: vec![],
                variant_context: None,
                is_in_value_position: false,
                is_in_key_position: true,
                is_variant_position: false,
                is_string_only: false,
                parent_path: None,
                partial_field: None,
            };
        }
    } else {
        let current_line = lines[position.line as usize];
        // eprintln!("Current line: {:?}", current_line);
        let char_pos = position.character.min(current_line.len() as u32) as usize;
        let line_before_cursor = if char_pos > 0 {
            &current_line[..char_pos]
        } else {
            ""
        };
        (current_line, line_before_cursor)
    };
    // eprintln!("Line before cursor: {:?}", line_before_cursor);
    
    // Determine if we're in a key or value position
    let mut is_in_value_position = false;
    let mut is_in_key_position = false;
    let mut is_variant_position = false;
    let mut is_string_only = false;
    
    // Extract parent path if completing after a dot
    let mut parent_path = None;
    
    // Check trigger character
    if let Some(ref trigger) = trigger_character {
        match trigger.as_str() {
            "@" => is_in_key_position = true,
            "." => {
                is_in_key_position = true;
                // Extract the parent path before the dot
                if let Some(dot_pos) = line_before_cursor.rfind('.') {
                    let before_dot = &line_before_cursor[..dot_pos];
                    // Find the identifier before the dot
                    let parent_ident = before_dot.split_whitespace().last();
                    parent_path = parent_ident.map(|s| s.to_string());
                }
            }
            ":" => {
                // Colon is for string-only binding
                is_in_value_position = true;
                is_string_only = true;
                // Check if we're after $variant
                // Look for $variant before the colon
                if let Some(colon_pos) = line_before_cursor.rfind(':') {
                    let before_colon = &line_before_cursor[..colon_pos];
                    if before_colon.trim_end().ends_with("$variant") {
                        is_variant_position = true;
                    }
                }
            }
            "=" => {
                // Equals is for any value binding
                is_in_value_position = true;
                // Check if we're after $variant
                // Look for $variant before the equals
                if let Some(eq_pos) = line_before_cursor.rfind('=') {
                    let before_eq = &line_before_cursor[..eq_pos];
                    if before_eq.trim_end().ends_with("$variant") {
                        is_variant_position = true;
                    }
                }
            }
            _ => {}
        }
    }
    
    // If no trigger character, analyze the line content
    if !is_in_key_position && !is_in_value_position {
        // Check if we're after a colon or equals sign
        if line_before_cursor.contains(':') || line_before_cursor.contains('=') {
            // Check if the last non-whitespace character indicates value position
            let trimmed = line_before_cursor.trim_end();
            if trimmed.ends_with(':') {
                is_in_value_position = true;
                is_string_only = true;
                // Check if this is a variant position
                if trimmed.trim_end_matches(':').trim_end().ends_with("$variant") {
                    is_variant_position = true;
                }
            } else if trimmed.ends_with('=') {
                is_in_value_position = true;
                is_string_only = false;
                // Check if this is a variant position
                if trimmed.trim_end_matches('=').trim_end().ends_with("$variant") {
                    is_variant_position = true;
                }
            }
        } else if line_before_cursor.trim().is_empty() || line_before_cursor.ends_with('@') {
            // Empty line or after @ means key position
            is_in_key_position = true;
        } else {
            // Check for variant context
            if line_before_cursor.contains("$variant") && line_before_cursor.ends_with(':') {
                is_in_value_position = true;
                is_string_only = true;
                is_variant_position = true;
            } else if line_before_cursor.contains("$variant") && line_before_cursor.ends_with('=') {
                is_in_value_position = true;
                is_string_only = false;
                is_variant_position = true;
            } else {
                // Default to key position
                is_in_key_position = true;
            }
        }
    }
    
    // Try to extract path context from the CST
    let path_context = {
        // Create a temporary Values instance for path extraction
        let mut values = Values::default();
        let mut value_visitor = eure_tree::value_visitor::ValueVisitor::new(text, &mut values);
        let _ = cst.visit_from_root(&mut value_visitor);
        
        // Extract path context
        let extractor = PathContextExtractor::new(text, &values, position);
        extractor.extract_context(cst)
    };
    
    // Use extracted context if available, otherwise fall back to simple analysis
    if let Some(ctx) = path_context {
        // eprintln!("Path context: is_variant_position = {}, is_in_value_position = {}", ctx.is_variant_position, ctx.is_in_value_position);
        CompletionContext {
            position,
            trigger_character,
            current_path: ctx.path_segments.iter()
                .filter_map(|seg| match seg {
                    PathSegment::Ident(id) => Some(id.as_ref().to_string()),
                    _ => None,
                })
                .collect(),
            path_segments: ctx.path_segments,
            variant_context: ctx.variant_context,
            is_in_value_position: ctx.is_in_value_position,
            is_in_key_position: ctx.is_in_key_position,
            is_variant_position: ctx.is_variant_position,
            is_string_only: ctx.is_string_only,
            parent_path: ctx.parent_path.or(parent_path),
            partial_field: ctx.partial_field,
        }
    } else {
        // eprintln!("No path context extracted, using simple analysis");
        // eprintln!("Simple analysis: is_variant_position = {}, is_in_value_position = {}", is_variant_position, is_in_value_position);
        
        // Try to extract path from section headers in the text
        let mut extracted_path = vec![];
        let lines_up_to_cursor: Vec<&str> = text.lines().take(position.line as usize + 1).collect();
        
        // If we're inside a section (current line is empty/whitespace and previous line is section header)
        if position.line > 0 && line_before_cursor.trim().is_empty() {
            // eprintln!("Checking for section on previous line...");
            // Check the previous line for a section header
            if let Some(prev_line) = lines_up_to_cursor.get(position.line as usize - 1) {
                // eprintln!("Previous line: {:?}", prev_line);
                if prev_line.trim_start().starts_with("@ ") {
                    // We're inside this section
                    let section_part = prev_line.trim_start().trim_start_matches("@ ").trim();
                    // eprintln!("Section part: {:?}", section_part);
                    
                    // Parse dotted paths like "script.dependencies"
                    if section_part.contains('.') {
                        let parts: Vec<&str> = section_part.split('.').collect();
                        for part in parts {
                            if let Ok(ident) = Identifier::from_str(part) {
                                extracted_path.push(PathSegment::Ident(ident));
                            }
                        }
                        is_in_key_position = true;
                    } else if let Some(brace_pos) = section_part.find('{') {
                        // Handle "@ database {" style
                        let path_str = section_part[..brace_pos].trim();
                        if let Ok(ident) = Identifier::from_str(path_str) {
                            extracted_path.push(PathSegment::Ident(ident));
                            is_in_key_position = true;
                        }
                    } else if section_part.ends_with("[]") {
                        let name = &section_part[..section_part.len() - 2];
                        if let Ok(ident) = Identifier::from_str(name) {
                            extracted_path.push(PathSegment::Ident(ident));
                        }
                        extracted_path.push(PathSegment::Array { 
                            key: EureValue::Null,
                            index: None,
                        });
                        is_in_key_position = true;
                    } else {
                        // Just section name
                        if let Ok(ident) = Identifier::from_str(section_part) {
                            extracted_path.push(PathSegment::Ident(ident));
                            is_in_key_position = true;
                        }
                    }
                }
            }
        } else {
            // Original logic for other cases
            for line in lines_up_to_cursor.iter().rev() {
                if line.trim_start().starts_with("@ ") {
                    // Found a section header, extract the path
                    let section_part = line.trim_start().trim_start_matches("@ ");
                    if let Some(space_pos) = section_part.find(' ') {
                        let path_str = &section_part[..space_pos];
                        if path_str.ends_with("[]") {
                            let name = &path_str[..path_str.len() - 2];
                            if let Ok(ident) = Identifier::from_str(name) {
                                extracted_path.push(PathSegment::Ident(ident));
                            }
                            extracted_path.push(PathSegment::Array { 
                                key: EureValue::Null,
                                index: None,
                            });
                        } else {
                            if let Ok(ident) = Identifier::from_str(path_str) {
                                extracted_path.push(PathSegment::Ident(ident));
                            }
                        }
                    } else if section_part.ends_with("[]") {
                        let name = &section_part[..section_part.len() - 2];
                        if let Ok(ident) = Identifier::from_str(name) {
                            extracted_path.push(PathSegment::Ident(ident));
                        }
                        extracted_path.push(PathSegment::Array { 
                            key: EureValue::Null,
                            index: None,
                        });
                    } else {
                        if let Ok(ident) = Identifier::from_str(section_part) {
                            extracted_path.push(PathSegment::Ident(ident));
                        }
                    }
                    break;
                }
            }
        }
        
        // eprintln!("Extracted path from text: {:?}", extracted_path);
        
        // Fallback to simple context
        CompletionContext {
            position,
            trigger_character,
            current_path: extracted_path.iter()
                .filter_map(|seg| match seg {
                    PathSegment::Ident(id) => Some(id.as_ref().to_string()),
                    _ => None,
                })
                .collect(),
            path_segments: extracted_path,
            variant_context: None,
            is_in_value_position,
            is_in_key_position,
            is_variant_position,
            is_string_only,
            parent_path,
            partial_field: None,
        }
    }
}

/// Look up a field by name in an object schema
fn lookup_field_by_name<'a>(obj_schema: &'a ObjectSchema, name: &str) -> Option<&'a eure_schema::FieldSchema> {
    obj_schema.fields.iter()
        .find(|(key, _)| {
            match key {
                KeyCmpValue::String(s) => s == name,
                KeyCmpValue::I64(n) => n.to_string() == name,
                KeyCmpValue::U64(n) => n.to_string() == name,
                KeyCmpValue::Extension(e) => format!("${e}") == name,
                KeyCmpValue::MetaExtension(m) => format!("$${m}") == name,
                _ => false,
            }
        })
        .map(|(_, field)| field)
}

/// Look up schema at a compound path like "user.address"
fn lookup_schema_at_compound_path<'a>(path: &str, schema: &'a DocumentSchema) -> Option<&'a ObjectSchema> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current_schema = &schema.root;
    
    for part in parts {
        if let Some(field) = lookup_field_by_name(current_schema, part) {
            match &field.type_expr {
                Type::Object(obj_schema) => {
                    current_schema = obj_schema;
                }
                _ => return None,
            }
        } else {
            return None;
        }
    }
    
    Some(current_schema)
}

/// Look up field at a path and return (ObjectSchema, section_preference)
fn lookup_field_and_preference<'a>(path: &str, schema: &'a DocumentSchema) -> Option<(&'a ObjectSchema, bool)> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current_schema = &schema.root;
    let mut last_field: Option<&'a FieldSchema> = None;
    
    for part in parts {
        if let Some(field) = lookup_field_by_name(current_schema, part) {
            last_field = Some(field);
            match &field.type_expr {
                Type::Object(obj_schema) => {
                    current_schema = obj_schema;
                }
                _ => return None,
            }
        } else {
            return None;
        }
    }
    
    // Check if the last field has section preference
    let section_pref = last_field.map(|f| f.preferences.section.unwrap_or(false)).unwrap_or(false);
    Some((current_schema, section_pref))
}

fn generate_completions(context: &CompletionContext, schema: &DocumentSchema) -> Vec<CompletionItem> {
    let mut completions = Vec::new();
    
    
    // If we're in a key position, suggest available fields
    if context.is_in_key_position {
        // Use path segments to find the correct schema location
        let (fields_to_complete, parent_has_section_preference) = 
            if !context.path_segments.is_empty() {
                if let Some(fields) = lookup_fields_at_path(&context.path_segments, schema, context.variant_context.as_deref()) {
                    (fields, false)
                } else {
                        // If we have a parent path from dot completion, try that
                        if let Some(ref parent) = context.parent_path {
                            if let Some((obj_schema, section_pref)) = lookup_field_and_preference(parent, schema) {
                                (&obj_schema.fields, section_pref)
                            } else {
                                return completions;
                            }
                        } else {
                            (&schema.root.fields, false)
                        }
                }
            } else if let Some(ref parent) = context.parent_path {
                // Look up the parent path in the schema
                if let Some((obj_schema, section_pref)) = lookup_field_and_preference(parent, schema) {
                    (&obj_schema.fields, section_pref)
                } else {
                    // Parent path not found
                    return completions;
                }
            } else {
                // No parent, use root fields
                (&schema.root.fields, false)
            };
        
        // Check if we're completing after a dot (e.g., "user.")
        let completing_after_dot = context.trigger_character.as_deref() == Some(".");
        
        // Add fields from the appropriate schema
        for (key, field_schema) in fields_to_complete {
            let label = match key {
                KeyCmpValue::String(s) => s.clone(),
                KeyCmpValue::I64(n) => n.to_string(),
                KeyCmpValue::U64(n) => n.to_string(),
                KeyCmpValue::Extension(e) => format!("${e}"),
                KeyCmpValue::MetaExtension(m) => format!("$${m}"),
                _ => continue, // Skip other types like Null, Bool, Tuple, Unit
            };
            
            let documentation = field_schema.description.as_ref().map(|desc| {
                Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: desc.clone(),
                })
            });
            
            // Check if we should generate a snippet for section fields
            let mut completion_item = CompletionItem {
                label: label.clone(),
                kind: Some(CompletionItemKind::FIELD),
                detail: Some(format!("Type: {:?}", field_schema.type_expr)),
                documentation,
                deprecated: Some(false),
                preselect: Some(false),
                ..Default::default()
            };
            
            // If completing after dot and parent has section preference
            if completing_after_dot && parent_has_section_preference {
                // Check if this field is an object type
                if let Type::Object(ref obj_schema) = field_schema.type_expr {
                    // Generate snippet for required fields
                    let parent_prefix = context.parent_path.as_deref().unwrap_or("");
                    if let Some(snippet) = generate_section_snippet(parent_prefix, &label, obj_schema) {
                        completion_item.insert_text_format = Some(InsertTextFormat::SNIPPET);
                        completion_item.insert_text = Some(snippet);
                    }
                }
            }
            
            completions.push(completion_item);
        }
        
        // Add $types if we have type definitions
        if !schema.types.is_empty() {
            completions.push(CompletionItem {
                label: "$types".to_string(),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some("Type definitions".to_string()),
                documentation: None,
                deprecated: Some(false),
                preselect: Some(false),
                ..Default::default()
            });
        }
    }
    
    // If we're in a value position, suggest based on the field type
    if context.is_in_value_position {
        // eprintln!("In value position, is_variant_position = {}", context.is_variant_position);
        
        // Check if we're completing a variant field
        if context.is_variant_position {
            // Find the variant type at the current path
            if let Some(variant_completions) = get_variant_completions(&context.path_segments, schema) {
                completions.extend(variant_completions);
            }
        } else {
            // Try to determine field type for value completions
            if let Some(field_type) = find_field_type_at_cursor(context, schema) {
                completions.extend(get_value_completions_for_type(&field_type, context.is_string_only));
            } else {
                // Fallback to generic value completions
                if !context.is_string_only {
                    // Common boolean values
                    completions.push(CompletionItem {
                        label: "true".to_string(),
                        kind: Some(CompletionItemKind::VALUE),
                        detail: Some("Boolean value".to_string()),
                        documentation: None,
                        deprecated: Some(false),
                        preselect: Some(false),
                        ..Default::default()
                    });
                    
                    completions.push(CompletionItem {
                        label: "false".to_string(),
                        kind: Some(CompletionItemKind::VALUE),
                        detail: Some("Boolean value".to_string()),
                        documentation: None,
                        deprecated: Some(false),
                        preselect: Some(false),
                        ..Default::default()
                    });
                    
                    // Null value
                    completions.push(CompletionItem {
                        label: "null".to_string(),
                        kind: Some(CompletionItemKind::VALUE),
                        detail: Some("Null value".to_string()),
                        documentation: None,
                        deprecated: Some(false),
                        preselect: Some(false),
                        ..Default::default()
                    });
                }
            }
        }
    }
    
    completions
}

fn generate_value_completions(context: &CompletionContext) -> Vec<CompletionItem> {
    let mut completions = Vec::new();
    
    // Check if we're completing a variant field
    if context.is_variant_position {
        completions.push(CompletionItem {
            label: "variant_name".to_string(),
            kind: Some(CompletionItemKind::ENUM_MEMBER),
            detail: Some("Variant name".to_string()),
            documentation: None,
            deprecated: Some(false),
            preselect: Some(false),
            ..Default::default()
        });
    } else if context.is_string_only {
        // After ":", only string values are allowed
        // No specific completions for strings, but we could add common string patterns later
    } else {
        // After "=", any value is allowed
        // Common boolean values
        completions.push(CompletionItem {
            label: "true".to_string(),
            kind: Some(CompletionItemKind::VALUE),
            detail: Some("Boolean value".to_string()),
            documentation: None,
            deprecated: Some(false),
            preselect: Some(false),
            ..Default::default()
        });
        
        completions.push(CompletionItem {
            label: "false".to_string(),
            kind: Some(CompletionItemKind::VALUE),
            detail: Some("Boolean value".to_string()),
            documentation: None,
            deprecated: Some(false),
            preselect: Some(false),
            ..Default::default()
        });
        
        // Null value
        completions.push(CompletionItem {
            label: "null".to_string(),
            kind: Some(CompletionItemKind::VALUE),
            detail: Some("Null value".to_string()),
            documentation: None,
            deprecated: Some(false),
            preselect: Some(false),
            ..Default::default()
        });
    }
    
    completions
}

fn position_to_byte_offset(text: &str, position: Position) -> usize {
    let mut line = 0;
    let mut col = 0;
    let mut offset = 0;
    
    for ch in text.chars() {
        if line == position.line && col == position.character {
            return offset;
        }
        
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
        
        offset += ch.len_utf8();
    }
    
    offset
}

/// Generate a snippet for a section with required fields filled with hole values
/// For example, when completing "user.name", generates:
/// ```text
/// user.name
/// first = !
/// last = !
/// ```
/// Navigate through the schema based on path segments to find the fields at that location  
fn lookup_fields_at_path<'a>(
    path: &[PathSegment],
    schema: &'a DocumentSchema,
    variant_context: Option<&str>,
) -> Option<&'a IndexMap<KeyCmpValue, FieldSchema>> {
    if path.is_empty() {
        return Some(&schema.root.fields);
    }
    
    // Start at the root
    let mut current_fields = &schema.root.fields;
    
    // Process path segments to find the target schema
    let mut i = 0;
    while i < path.len() {
        match &path[i] {
            PathSegment::Ident(id) => {
                let field_name = id.as_ref();
                
                // Look up the field
                let field = current_fields.get(&KeyCmpValue::String(field_name.to_string()))?;
                
                // Check if this is an array field
                if i + 1 < path.len() && matches!(path[i + 1], PathSegment::Array { .. }) {
                    // Next segment is array access, we need to unwrap the array type
                    match &field.type_expr {
                        Type::Array(elem_type) => {
                            // Skip the array segment
                            i += 2;
                            
                            // Continue with the element type
                            match elem_type.as_ref() {
                                Type::Object(obj) => {
                                    current_fields = &obj.fields;
                                }
                                Type::TypeRef(type_name) => {
                                    // Resolve type reference
                                    let type_def = schema.types.get(type_name)?;
                                    match &type_def.type_expr {
                                        Type::Object(obj) => {
                                            current_fields = &obj.fields;
                                        }
                                        Type::Variants(variants) => {
                                            // Need variant context
                                            let variant_name = variant_context?;
                                            let variant = variants.variants.get(&KeyCmpValue::String(variant_name.to_string()))?;
                                            current_fields = &variant.fields;
                                        }
                                        _ => return None,
                                    }
                                }
                                Type::Variants(variants) => {
                                    // Need variant context
                                    let variant_name = variant_context?;
                                    let variant = variants.variants.get(&KeyCmpValue::String(variant_name.to_string()))?;
                                    current_fields = &variant.fields;
                                }
                                _ => return None,
                            }
                            continue;
                        }
                        _ => return None, // Expected array type
                    }
                } else {
                    // Regular field access
                    match &field.type_expr {
                        Type::Object(obj) => {
                            current_fields = &obj.fields;
                        }
                        Type::TypeRef(type_name) => {
                            // Resolve type reference
                            let type_def = schema.types.get(type_name)?;
                            match &type_def.type_expr {
                                Type::Object(obj) => {
                                    current_fields = &obj.fields;
                                }
                                Type::Variants(variants) => {
                                    // If this is the last segment, we can return the variant fields
                                    if i == path.len() - 1 && variant_context.is_some() {
                                        let variant_name = variant_context?;
                                        return variants.variants.get(&KeyCmpValue::String(variant_name.to_string()))
                                            .map(|v| &v.fields);
                                    }
                                    return None;
                                }
                                _ => return None,
                            }
                        }
                        Type::Variants(variants) => {
                            // If this is the last segment, we can return the variant fields
                            if i == path.len() - 1 && variant_context.is_some() {
                                let variant_name = variant_context?;
                                return variants.variants.get(&KeyCmpValue::String(variant_name.to_string()))
                                    .map(|v| &v.fields);
                            }
                            return None;
                        }
                        _ => return None,
                    }
                }
                i += 1;
            }
            PathSegment::Array { .. } => {
                // This shouldn't happen - array segments should be preceded by field names
                return None;
            }
            _ => {
                // Skip other segment types
                i += 1;
            }
        }
    }
    
    // Return the final fields
    Some(current_fields)
}

/// Get completion items for variant names
fn get_variant_completions(path: &[PathSegment], schema: &DocumentSchema) -> Option<Vec<CompletionItem>> {
    // eprintln!("get_variant_completions: path = {:?}", path);
    
    // We need to navigate through the full path to find the variant type
    let mut current_type = &Type::Object(schema.root.clone());
    
    for (_i, segment) in path.iter().enumerate() {
        let _type_desc = match current_type {
            Type::Object(_) => "Object".to_string(),
            Type::Array(_) => "Array".to_string(),
            Type::TypeRef(name) => format!("TypeRef({})", 
                match name {
                    KeyCmpValue::String(s) => s,
                    _ => "?",
                }
            ),
            Type::Variants(_) => "Variants".to_string(),
            _ => "Other".to_string(),
        };
        // eprintln!("  Processing segment {}: {:?}, current_type: {}", i, segment, type_desc);
        
        match segment {
            PathSegment::Ident(id) => {
                // Look up the field
                match current_type {
                    Type::Object(obj_schema) => {
                        let field_name = id.as_ref();
                        if let Some(field_schema) = lookup_field_by_name(obj_schema, field_name) {
                            current_type = &field_schema.type_expr;
                        } else {
                            // eprintln!("  Field '{}' not found in object", field_name);
                            return None;
                        }
                    }
                    Type::TypeRef(type_name) => {
                        // Resolve type reference first
                        if let Some(resolved_type) = schema.types.get(type_name) {
                            match &resolved_type.type_expr {
                                Type::Object(obj) => {
                                    // Now look up the field in the resolved object
                                    let field_name = id.as_ref();
                                    if let Some(field_schema) = lookup_field_by_name(obj, field_name) {
                                        current_type = &field_schema.type_expr;
                                    } else {
                                        return None;
                                    }
                                }
                                _ => return None,
                            }
                        } else {
                            return None;
                        }
                    }
                    _ => return None,
                }
            }
            PathSegment::Array { .. } => {
                // We're accessing an array element, get the element type
                match current_type {
                    Type::Array(elem_type) => {
                        current_type = elem_type;
                    }
                    _ => return None,
                }
            }
            _ => {} // Skip other segment types
        }
    }
    
    // Check if the current type is a variant type
    let variant_schema = match current_type {
        Type::Variants(v) => v,
        Type::TypeRef(type_name) => {
            // Resolve type reference
            if let Some(resolved_type) = schema.types.get(type_name) {
                if let Type::Variants(v) = &resolved_type.type_expr {
                    v
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
        _ => return None,
    };
    
    // Generate completion items for each variant
    let mut completions = Vec::new();
    for (variant_key, variant_obj) in &variant_schema.variants {
        if let KeyCmpValue::String(variant_name) = variant_key {
            let mut completion = CompletionItem {
                label: variant_name.clone(),
                kind: Some(CompletionItemKind::ENUM_MEMBER),
                detail: Some(format!("Variant: {}", variant_name)),
                documentation: None,
                deprecated: Some(false),
                preselect: Some(false),
                ..Default::default()
            };
            
            // Add documentation if available
            if !variant_obj.fields.is_empty() {
                let field_names: Vec<String> = variant_obj.fields.keys()
                    .filter_map(|k| match k {
                        KeyCmpValue::String(s) => Some(s.clone()),
                        _ => None,
                    })
                    .collect();
                let doc = format!("Fields: {}", field_names.join(", "));
                completion.documentation = Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: doc,
                }));
            }
            
            completions.push(completion);
        }
    }
    
    Some(completions)
}

/// Find the field type at the cursor position
fn find_field_type_at_cursor(_context: &CompletionContext, _schema: &DocumentSchema) -> Option<Type> {
    // TODO: Extract the field name being assigned and look up its type
    // For now, return None to use generic completions
    None
}

/// Get value completions for a specific type
fn get_value_completions_for_type(field_type: &Type, string_only: bool) -> Vec<CompletionItem> {
    let mut completions = Vec::new();
    
    match field_type {
        Type::Boolean => {
            if !string_only {
                completions.push(CompletionItem {
                    label: "true".to_string(),
                    kind: Some(CompletionItemKind::VALUE),
                    detail: Some("Boolean value".to_string()),
                    ..Default::default()
                });
                completions.push(CompletionItem {
                    label: "false".to_string(),
                    kind: Some(CompletionItemKind::VALUE),
                    detail: Some("Boolean value".to_string()),
                    ..Default::default()
                });
            }
        }
        Type::String => {
            // Could add common string patterns or enum values here
        }
        Type::Number => {
            // Could add common numbers
        }
        _ => {
            // For other types, provide generic completions
            if !string_only {
                completions.push(CompletionItem {
                    label: "null".to_string(),
                    kind: Some(CompletionItemKind::VALUE),
                    detail: Some("Null value".to_string()),
                    ..Default::default()
                });
            }
        }
    }
    
    completions
}

fn generate_section_snippet(
    parent_path: &str,
    field_name: &str,
    object_schema: &ObjectSchema,
) -> Option<String> {
    // Collect required fields (where optional is false)
    let required_fields: Vec<(&KeyCmpValue, &eure_schema::FieldSchema)> = object_schema
        .fields
        .iter()
        .filter(|(_, field)| !field.optional)
        .collect();
    
    // If no required fields, return None
    if required_fields.is_empty() {
        return None;
    }
    
    // Build the snippet
    let mut snippet = String::new();
    let mut tab_stop = 1;
    
    // Add the field being completed (e.g., "user.name")
    let prefix = if parent_path.is_empty() {
        field_name.to_string()
    } else {
        format!("{parent_path}.{field_name}")
    };
    
    // Generate the snippet with all required fields
    snippet.push_str(&prefix);
    snippet.push('\n');
    
    // Add all required fields with hole values
    for (key, _field) in required_fields.iter() {
            let key_str = match key {
                KeyCmpValue::String(s) => s.clone(),
                KeyCmpValue::I64(n) => n.to_string(),
                KeyCmpValue::U64(n) => n.to_string(),
                KeyCmpValue::Extension(e) => format!("${e}"),
                KeyCmpValue::MetaExtension(m) => format!("$${m}"),
                _ => continue, // Skip other types
            };
            
            // Add the field with a hole value
            snippet.push_str(&format!("{key_str} = ${{{tab_stop}:!}}\n"));
            tab_stop += 1;
    }
    
    // Add final cursor position
    snippet.push_str("$0");
    
    Some(snippet)
}

