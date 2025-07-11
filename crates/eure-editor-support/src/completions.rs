use eure_tree::{Cst, document::EureDocument};
use lsp_types::{CompletionItem, CompletionItemKind, InsertTextFormat, Position};

use crate::completion_analyzer::CompletionAnalyzer;
use crate::schema_validation::SchemaManager;
use eure_schema::{DocumentSchema, FieldSchema, ObjectSchema, Type, VariantSchema};
use eure_value::value::KeyCmpValue;
use eure_value::identifier::Identifier;
use std::collections::HashSet;
use std::str::FromStr;

pub fn get_completions(
    text: &str,
    _cst: &Cst,
    position: Position,
    _trigger_character: Option<String>,
    uri: &str,
    schema_manager: &SchemaManager,
    cached_document: Option<&EureDocument>,
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

    let _error_completions = analyzer.analyze();

    // Get the schema for this document
    let schema_uri = schema_manager.get_document_schema_uri(uri);
    let schema = match schema_uri.and_then(|uri| schema_manager.get_schema(uri)) {
        Some(s) => s,
        None => return _error_completions, // No schema, use error-based completions
    };

    // Analyze the context at cursor position
    let context = analyze_context_at_position(text, position, cached_document);
    
    let schema_completions = match context {
        CompletionContext::AfterAt { path, partial_field, prepend_at } => {
            let mut completions = generate_field_completions(&path, schema, partial_field.as_deref(), &HashSet::new());
            if prepend_at && path.is_empty() {
                // Prepend @ only for empty documents where user hasn't typed @ yet
                for completion in &mut completions {
                    completion.label = format!("@ {}", completion.label);
                    if let Some(insert_text) = &mut completion.insert_text {
                        *insert_text = format!("@ {}", insert_text);
                    } else {
                        completion.insert_text = Some(format!("@ {}", completion.label));
                    }
                }
            }
            completions
        }
        CompletionContext::AfterDot { path, partial_field } => {
            generate_field_completions(&path, schema, partial_field.as_deref(), &HashSet::new())
        }
        CompletionContext::AfterEquals { path, field_name } => {
            generate_value_completions(&path, &field_name, schema, false)
        }
        CompletionContext::AfterColon { path, field_name } => {
            // After colon, only string values are allowed
            if field_name == "$variant" {
                generate_variant_completions(&path, schema)
            } else {
                vec![] // No completions for string-only context
            }
        }
        CompletionContext::Unknown => vec![],
    };
    
    // For now, always prefer schema completions when we have a schema
    // The error-based completions don't properly track paths yet
    schema_completions
}

#[derive(Debug)]
enum CompletionContext {
    AfterAt { path: Vec<String>, partial_field: Option<String>, prepend_at: bool },
    AfterDot { path: Vec<String>, partial_field: Option<String> },
    AfterEquals { path: Vec<String>, field_name: String },
    AfterColon { path: Vec<String>, field_name: String },
    Unknown,
}

fn analyze_context_at_position(
    text: &str,
    position: Position,
    _cached_document: Option<&EureDocument>,
) -> CompletionContext {
    let lines: Vec<&str> = text.lines().collect();
    
    // Handle empty document
    if text.is_empty() || (lines.is_empty() && position.line == 0 && position.character == 0) {
        // Empty document - suggest root fields with @ prefix
        return CompletionContext::AfterAt { path: vec![], partial_field: None, prepend_at: true };
    }
    
    if position.line as usize >= lines.len() {
        return CompletionContext::Unknown;
    }
    
    let current_line = lines[position.line as usize];
    let char_pos = position.character.min(current_line.len() as u32) as usize;
    let line_before_cursor = &current_line[..char_pos];
    
    // Check what's immediately before cursor
    let trimmed = line_before_cursor.trim_end();
    
    // Handle empty line
    if trimmed.is_empty() && char_pos == 0 {
        // Beginning of line - suggest root fields with @ prefix
        return CompletionContext::AfterAt { path: vec![], partial_field: None, prepend_at: true };
    }
    
    if trimmed.ends_with('@') {
        // Extract path from line if present
        let path = extract_path_from_line(trimmed);
        // User already typed @, don't prepend
        CompletionContext::AfterAt { path, partial_field: None, prepend_at: false }
    } else if trimmed.ends_with('.') {
        // Extract path including the part before the dot
        let path = extract_path_from_line(trimmed);
        CompletionContext::AfterDot { path, partial_field: None }
    } else if trimmed.ends_with('=') {
        // Extract the field name and path
        let parts: Vec<&str> = trimmed.trim_end_matches('=').trim().split_whitespace().collect();
        if let Some(field_part) = parts.last() {
            let (path, field_name) = parse_field_reference(field_part);
            CompletionContext::AfterEquals { path, field_name }
        } else {
            CompletionContext::Unknown
        }
    } else if trimmed.ends_with(':') {
        // Extract the field name and path, accounting for context
        let parts: Vec<&str> = trimmed.trim_end_matches(':').trim().split_whitespace().collect();
        if let Some(field_part) = parts.last() {
            let (mut path, field_name) = parse_field_reference(field_part);
            
            // Special handling for array context - look backwards for @ section[]
            if path.is_empty() && field_name == "$variant" {
                // Look for array section context in previous lines
                if let Some(array_path) = find_array_context(text, position) {
                    path = array_path;
                }
            }
            
            CompletionContext::AfterColon { path, field_name }
        } else {
            CompletionContext::Unknown
        }
    } else if trimmed.contains('@') && !trimmed.contains('=') && !trimmed.contains(':') {
        // Partial field after @
        if let Some(at_pos) = trimmed.rfind('@') {
            let after_at = &trimmed[at_pos + 1..].trim();
            let path = extract_path_from_line(&trimmed[..at_pos]);
            let partial_field = if after_at.is_empty() { None } else { Some(after_at.to_string()) };
            // User already typed @, don't prepend
            CompletionContext::AfterAt { path, partial_field, prepend_at: false }
        } else {
            CompletionContext::Unknown
        }
    } else {
        CompletionContext::Unknown
    }
}

fn extract_path_from_line(line: &str) -> Vec<String> {
    // Extract path up to the final dot
    let trimmed = line.trim_start().trim_end_matches('.');
    
    
    if trimmed.is_empty() {
        vec![]
    } else if trimmed.starts_with('@') {
        let path = trimmed[1..].split('.').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        path
    } else {
        let path = trimmed.split('.').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        path
    }
}

fn parse_field_reference(field_ref: &str) -> (Vec<String>, String) {
    let parts: Vec<&str> = field_ref.split('.').collect();
    if parts.len() > 1 {
        let path = parts[..parts.len()-1].iter().map(|s| s.to_string()).collect();
        let field = parts.last().unwrap().to_string();
        (path, field)
    } else {
        (vec![], field_ref.to_string())
    }
}

fn generate_field_completions(
    path: &[String],
    schema: &DocumentSchema,
    partial_field: Option<&str>,
    used_fields: &HashSet<String>,
) -> Vec<CompletionItem> {
    
    
    // Find the schema at the current path
    let object_schema = match lookup_schema_at_path(path, &schema.root) {
        Some(obj) => obj,
        None => {
            
            return vec![];
        }
    };
    
    let mut completions = vec![];
    
    for (key, field_schema) in &object_schema.fields {
        let field_name = match key {
            KeyCmpValue::String(s) => s.clone(),
            KeyCmpValue::MetaExtension(s) => format!("${}", s),
            _ => continue,
        };
        
        // Skip already used fields
        if used_fields.contains(&field_name) {
            continue;
        }
        
        // Filter by partial field if present
        if let Some(partial) = partial_field {
            if !field_name.starts_with(partial) {
                continue;
            }
        }
        
        // Check if this field should generate a snippet
        let (insert_text, insert_format) = if should_generate_snippet(field_schema) {
            let snippet = generate_field_snippet(&field_name, field_schema, path);
            (Some(snippet), Some(InsertTextFormat::SNIPPET))
        } else {
            (None, None)
        };
        
        completions.push(CompletionItem {
            label: field_name,
            kind: Some(CompletionItemKind::FIELD),
            detail: field_schema.description.clone(),
            documentation: field_schema.description.as_ref().map(|desc| {
                lsp_types::Documentation::MarkupContent(lsp_types::MarkupContent {
                    kind: lsp_types::MarkupKind::Markdown,
                    value: desc.clone(),
                })
            }),
            insert_text,
            insert_text_format: insert_format,
            ..Default::default()
        });
    }
    
    
    completions
}

fn generate_value_completions(
    path: &[String],
    field_name: &str,
    schema: &DocumentSchema,
    is_string_only: bool,
) -> Vec<CompletionItem> {
    // For string-only contexts (after :), don't suggest boolean/null
    if is_string_only {
        return vec![];
    }
    
    // Find the field type
    if let Some(field_type) = get_field_type_at_path(path, field_name, &schema.root) {
        return generate_values_for_type(field_type);
    }
    
    // Default value completions for any type
    vec![
        CompletionItem {
            label: "true".to_string(),
            kind: Some(CompletionItemKind::VALUE),
            ..Default::default()
        },
        CompletionItem {
            label: "false".to_string(),
            kind: Some(CompletionItemKind::VALUE),
            ..Default::default()
        },
        CompletionItem {
            label: "null".to_string(),
            kind: Some(CompletionItemKind::VALUE),
            ..Default::default()
        },
    ]
}

fn generate_variant_completions(
    path: &[String],
    schema: &DocumentSchema,
) -> Vec<CompletionItem> {
    
    
    // If path is not empty, we might be looking at an array element
    // Need to check the parent path for the array type
    if !path.is_empty() {
        let parent_path = &path[..path.len()-1];
        let field_name = &path[path.len()-1];
        
        
        
        if let Some(parent_obj) = lookup_schema_at_path(parent_path, &schema.root) {
            
            
            // Look for the field in the parent
            let field_key = KeyCmpValue::String(field_name.clone());
            if let Some(field_schema) = parent_obj.fields.get(&field_key) {
                
                
                // Check if it's an array
                if let Type::Array(elem_type) = &field_schema.type_expr {
                    
                    
                    // Check if the element type is a variant type reference
                    if let Type::TypeRef(type_ref) = elem_type.as_ref() {
                        
                        
                        // Look up the referenced type
                        // First check if it's a path like .$types.Action
                        let type_key = if let KeyCmpValue::String(s) = type_ref {
                            if s.starts_with("$types.") {
                                // Extract the type name after $types.
                                KeyCmpValue::String(s[7..].to_string())
                            } else {
                                type_ref.clone()
                            }
                        } else {
                            type_ref.clone()
                        };
                        
                        
                        
                        if let Some(type_field) = schema.types.get(&type_key) {
                            
                            
                            
                            // Check if the type itself is a Variants type
                            if let Type::Variants(variant_schema) = &type_field.type_expr {
                                
                                return generate_variant_completion_items(variant_schema);
                            }
                            
                            // Or if it's an Object with $variants field
                            if let Type::Object(type_obj) = &type_field.type_expr {
                                
                                // Check if this type has $variants
                                let variants_key = KeyCmpValue::MetaExtension(Identifier::from_str("variants").unwrap());
                                if let Some(variants_field) = type_obj.fields.get(&variants_key) {
                                    
                                    
                                    if let Type::Variants(variant_schema) = &variants_field.type_expr {
                                        
                                        return generate_variant_completion_items(variant_schema);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Look up the variant schema at the path
    if let Some(variant_schema) = lookup_variant_schema_at_path(path, &schema.root) {
        return generate_variant_completion_items(variant_schema);
    }
    
    vec![]
}

fn generate_variant_completion_items(variant_schema: &VariantSchema) -> Vec<CompletionItem> {
    let mut completions = vec![];
    
    for variant_name in variant_schema.variants.keys() {
        let label = match variant_name {
            KeyCmpValue::String(s) => s.clone(),
            _ => continue,
        };
        
        completions.push(CompletionItem {
            label: label.clone(),
            kind: Some(CompletionItemKind::ENUM_MEMBER),
            detail: Some(format!("Variant: {}", label)),
            ..Default::default()
        });
    }
    
    
    completions
}

fn generate_values_for_type(field_type: &Type) -> Vec<CompletionItem> {
    match field_type {
        Type::Boolean => vec![
            CompletionItem {
                label: "true".to_string(),
                kind: Some(CompletionItemKind::VALUE),
                ..Default::default()
            },
            CompletionItem {
                label: "false".to_string(),
                kind: Some(CompletionItemKind::VALUE),
                ..Default::default()
            },
        ],
        Type::Null => vec![
            CompletionItem {
                label: "null".to_string(),
                kind: Some(CompletionItemKind::VALUE),
                ..Default::default()
            },
        ],
        Type::Union(types) => {
            // Generate completions for all types in the union
            let mut completions = vec![];
            let mut seen = HashSet::new();
            
            for t in types {
                for item in generate_values_for_type(t) {
                    if seen.insert(item.label.clone()) {
                        completions.push(item);
                    }
                }
            }
            
            completions
        }
        _ => {
            // For other types, provide generic completions
            vec![
                CompletionItem {
                    label: "true".to_string(),
                    kind: Some(CompletionItemKind::VALUE),
                    ..Default::default()
                },
                CompletionItem {
                    label: "false".to_string(),
                    kind: Some(CompletionItemKind::VALUE),
                    ..Default::default()
                },
                CompletionItem {
                    label: "null".to_string(),
                    kind: Some(CompletionItemKind::VALUE),
                    ..Default::default()
                },
            ]
        }
    }
}


fn should_generate_snippet(field_schema: &FieldSchema) -> bool {
    // Generate snippets for object fields that have required sub-fields
    if let Type::Object(obj) = &field_schema.type_expr {
        // Check if this object has any required fields
        let has_required_fields = obj.fields.values().any(|f| !f.optional);
        return has_required_fields;
    }
    false
}

fn generate_field_snippet(
    field_name: &str,
    field_schema: &FieldSchema,
    parent_path: &[String],
) -> String {
    if let Type::Object(obj) = &field_schema.type_expr {
        let mut snippet = String::new();
        
        // Build the full path for the section
        let mut path_parts = parent_path.to_vec();
        path_parts.push(field_name.to_string());
        let full_path = path_parts.join(".");
        
        snippet.push_str(&full_path);
        snippet.push_str(" {\n");
        
        // Add required fields
        let mut tab_index = 1;
        for (key, sub_field) in &obj.fields {
            if sub_field.optional {
                continue; // Skip optional fields in snippet
            }
            
            let sub_field_name = match key {
                KeyCmpValue::String(s) => s,
                _ => continue,
            };
            
            snippet.push_str("    ");
            snippet.push_str(sub_field_name);
            snippet.push_str(" = ${");
            snippet.push_str(&tab_index.to_string());
            snippet.push_str(":!}\n");
            tab_index += 1;
        }
        
        snippet.push_str("}\n$0"); // Final cursor position
        snippet
    } else {
        field_name.to_string()
    }
}

fn lookup_schema_at_path<'a>(
    path: &[String],
    root: &'a ObjectSchema,
) -> Option<&'a ObjectSchema> {
    let mut current = root;
    
    for segment in path {
        // Look up the field
        let key = KeyCmpValue::String(segment.clone());
        if let Some(field) = current.fields.get(&key) {
            match &field.type_expr {
                Type::Object(obj) => current = obj,
                Type::Array(elem_type) => {
                    // For arrays, look at the element type
                    if let Type::Object(obj) = elem_type.as_ref() {
                        current = obj;
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
    
    Some(current)
}

fn lookup_variant_schema_at_path<'a>(
    path: &[String],
    root: &'a ObjectSchema,
) -> Option<&'a VariantSchema> {
    let object = lookup_schema_at_path(path, root)?;
    
    // Look for $variants field
    let variants_key = KeyCmpValue::MetaExtension(Identifier::from_str("variants").ok()?);
    if let Some(field) = object.fields.get(&variants_key) {
        if let Type::Variants(variant_schema) = &field.type_expr {
            return Some(variant_schema);
        }
    }
    
    None
}

fn get_field_type_at_path<'a>(
    path: &[String],
    field_name: &str,
    root: &'a ObjectSchema,
) -> Option<&'a Type> {
    let object = lookup_schema_at_path(path, root)?;
    
    // Look up the field
    let key = if field_name.starts_with('$') {
        if let Ok(id) = Identifier::from_str(&field_name[1..]) {
            KeyCmpValue::MetaExtension(id)
        } else {
            return None;
        }
    } else {
        KeyCmpValue::String(field_name.to_string())
    };
    
    object.fields.get(&key).map(|field| &field.type_expr)
}

fn find_array_context(text: &str, position: Position) -> Option<Vec<String>> {
    let lines: Vec<&str> = text.lines().collect();
    
    // Look backwards from current position for @ section[] pattern
    for i in (0..=position.line as usize).rev() {
        if i >= lines.len() {
            continue;
        }
        
        let line = lines[i].trim();
        if line.starts_with('@') && line.ends_with("[]") {
            // Extract the section path
            let section_part = line[1..].trim_end_matches("[]").trim();
            let path: Vec<String> = section_part.split('.').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            return Some(path);
        }
    }
    
    None
}