use eure_tree::{Cst, document::EureDocument, tree::{CstNodeData, CstNodeId, TerminalData, NonTerminalData, InputSpan}, node_kind::{NonTerminalKind, TerminalKind}};
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
    cst: &Cst,
    position: Position,
    _trigger_character: Option<String>,
    uri: &str,
    schema_manager: &SchemaManager,
    cached_document: Option<&EureDocument>,
) -> Vec<CompletionItem> {
    // Try handle-based completion first if we have a cached document
    if let Some(document) = cached_document {
        if let Some(handle_completions) = get_handle_based_completions(
            text, cst, position, uri, schema_manager, document
        ) {
            return handle_completions;
        }
    }
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
            let used_fields = find_used_fields_in_section(text, position, &path);
            let variant_context = find_variant_context(text, position, &path);
            
            let mut completions = if let Some(variant_name) = variant_context {
                // Check if the path leads to a variant type
                if is_variant_path(&path, schema) {
                    generate_variant_field_completions(&path, schema, &variant_name, partial_field.as_deref(), &used_fields)
                } else {
                    // Regular variant fields within an object
                    generate_variant_field_completions(&path, schema, &variant_name, partial_field.as_deref(), &used_fields)
                }
            } else {
                generate_field_completions(&path, schema, partial_field.as_deref(), &used_fields)
            };
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
            let used_fields = find_used_fields_in_section(text, position, &path);
            let variant_context = find_variant_context(text, position, &path);
            if let Some(variant_name) = variant_context {
                generate_variant_field_completions(&path, schema, &variant_name, partial_field.as_deref(), &used_fields)
            } else {
                generate_field_completions(&path, schema, partial_field.as_deref(), &used_fields)
            }
        }
        CompletionContext::AfterEquals { path, field_name } => {
            // Extensions like $variant need special handling since they're not schema fields
            if field_name == "$variant" {
                generate_variant_completions(&path, schema)
            } else {
                generate_value_completions(&path, &field_name, schema, false)
            }
        }
        CompletionContext::AfterColon { path, field_name } => {
            // After colon, check if it's a variant completion
            if field_name == "$variant" {
                generate_variant_completions(&path, schema)
            } else {
                // After colon, only string values are allowed for normal fields
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
    cached_document: Option<&EureDocument>,
) -> CompletionContext {
    let lines: Vec<&str> = text.lines().collect();
    
    // Handle empty document
    if text.is_empty() || (lines.is_empty() && position.line == 0 && position.character == 0) {
        // Empty document - suggest root fields with @ prefix
        return CompletionContext::AfterAt { path: vec![], partial_field: None, prepend_at: true };
    }
    
    // Handle position beyond last line (e.g., on empty line after content)
    if position.line as usize >= lines.len() {
        // Check if we're in a section context
        if let Some(section_path) = find_section_context(text, position) {
            return CompletionContext::AfterAt { 
                path: section_path, 
                partial_field: None, 
                prepend_at: false 
            };
        }
        return CompletionContext::Unknown;
    }
    
    let current_line = lines[position.line as usize];
    let char_pos = position.character.min(current_line.len() as u32) as usize;
    let line_before_cursor = &current_line[..char_pos];
    
    // Check what's immediately before cursor
    let trimmed = line_before_cursor.trim_end();
    
    // Handle empty line or line with only whitespace
    if trimmed.is_empty() {
        // Check if we're inside a section first
        if let Some(section_path) = find_section_context(text, position) {
            return CompletionContext::AfterAt { 
                path: section_path, 
                partial_field: None, 
                prepend_at: false 
            };
        }
        // Beginning of line - suggest root fields with @ prefix
        return CompletionContext::AfterAt { path: vec![], partial_field: None, prepend_at: char_pos == 0 };
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
            
            // If path is empty, check if we're inside a section context
            if path.is_empty() {
                if let Some(section_path) = find_section_context(text, position) {
                    path = section_path;
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
        // Check if we're inside a section by looking at previous lines
        if let Some(section_path) = find_section_context(text, position) {
            // We're inside a section on an empty line
            return CompletionContext::AfterAt { 
                path: section_path, 
                partial_field: None, 
                prepend_at: false 
            };
        }
        
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

fn generate_variant_field_completions(
    path: &[String],
    schema: &DocumentSchema,
    variant_name: &str,
    partial_field: Option<&str>,
    used_fields: &HashSet<String>,
) -> Vec<CompletionItem> {
    // First, find what type we're dealing with at this path
    let variant_schema = if !path.is_empty() {
        // Check if we're in an array element context
        let last_segment = &path[path.len() - 1];
        if last_segment.ends_with("[]") {
            // We're inside an array element
            let parent_path = &path[..path.len() - 1];
            let field_name = last_segment.trim_end_matches("[]");
            
            let parent_schema = if parent_path.is_empty() {
                &schema.root
            } else {
                match lookup_schema_at_path_with_context(parent_path, &schema.root, Some(schema)) {
                    Some(obj) => obj,
                    None => return vec![],
                }
            };
            
            // Get the array field
            let key = KeyCmpValue::String(field_name.to_string());
            if let Some(field_schema) = parent_schema.fields.get(&key) {
                if let Type::Array(elem_type) = &field_schema.type_expr {
                    // Check if element type is a type reference
                    if let Type::TypeRef(type_ref) = elem_type.as_ref() {
                        lookup_variant_schema(type_ref, schema)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            // Regular path - check if it's a variant type field
            let parent_path = &path[..path.len() - 1];
            let field_name = &path[path.len() - 1];
            
            let parent_schema = if parent_path.is_empty() {
                &schema.root
            } else {
                match lookup_schema_at_path_with_context(parent_path, &schema.root, Some(schema)) {
                    Some(obj) => obj,
                    None => return vec![],
                }
            };
            
            // Check if the field is a variant type
            let key = KeyCmpValue::String(field_name.clone());
            if let Some(field_schema) = parent_schema.fields.get(&key) {
                if let Type::TypeRef(type_ref) = &field_schema.type_expr {
                    lookup_variant_schema(type_ref, schema)
                } else {
                    None
                }
            } else {
                None
            }
        }
    } else {
        // At root - look for variant types in the current object
        None
    };
    
    if let Some(variant_schema) = variant_schema {
        // Look for the specific variant
        let variant_key = KeyCmpValue::String(variant_name.to_string());
        if let Some(variant_def) = variant_schema.variants.get(&variant_key) {
            
            let mut completions = vec![];
            
            // Generate completions from variant fields
            for (key, field_schema) in &variant_def.fields {
                let field_name = match key {
                    KeyCmpValue::String(s) => s.clone(),
                    KeyCmpValue::MetaExtension(s) => format!("${}", s),
                    _ => continue,
                };
                
                // Skip already used fields
                if used_fields.contains(&field_name) {
                    continue;
                }
                
                // Check if field matches partial
                if let Some(partial) = partial_field {
                    if !field_name.starts_with(partial) {
                        continue;
                    }
                }
                
                completions.push(CompletionItem {
                    label: field_name.clone(),
                    kind: Some(CompletionItemKind::FIELD),
                    detail: Some(format!("{:?}", field_schema.type_expr)),
                    documentation: field_schema.description.as_ref().map(|desc| {
                        lsp_types::Documentation::MarkupContent(lsp_types::MarkupContent {
                            kind: lsp_types::MarkupKind::Markdown,
                            value: desc.clone(),
                        })
                    }),
                    ..Default::default()
                });
            }
            
            return completions;
        }
    }
    
    // Fallback to regular field completions
    generate_field_completions(path, schema, partial_field, used_fields)
}

fn generate_field_completions(
    path: &[String],
    schema: &DocumentSchema,
    partial_field: Option<&str>,
    used_fields: &HashSet<String>,
) -> Vec<CompletionItem> {
    // Check if we're completing after .$types.
    if !path.is_empty() && path[path.len() - 1] == "$types" {
        return generate_type_completions(schema, partial_field);
    }
    
    // Find the schema at the current path
    let object_schema = match lookup_schema_at_path_with_context(path, &schema.root, Some(schema)) {
        Some(obj) => obj,
        None => {
            return vec![];
        }
    };
    
    let mut completions = vec![];
    
    // At root level, also add $types if there are any types defined
    if path.is_empty() && !schema.types.is_empty() {
        if partial_field.is_none() || "$types".starts_with(partial_field.unwrap_or("")) {
            completions.push(CompletionItem {
                label: "$types".to_string(),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some("Type definitions namespace".to_string()),
                documentation: Some(lsp_types::Documentation::MarkupContent(lsp_types::MarkupContent {
                    kind: lsp_types::MarkupKind::Markdown,
                    value: "Access type definitions using `$types.TypeName`".to_string(),
                })),
                ..Default::default()
            });
        }
    }
    
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

fn generate_type_completions(
    schema: &DocumentSchema,
    partial_type: Option<&str>,
) -> Vec<CompletionItem> {
    let mut completions = vec![];
    
    for (key, _type_field) in &schema.types {
        let type_name = match key {
            KeyCmpValue::String(s) => s.clone(),
            _ => continue,
        };
        
        // Filter by partial type if present
        if let Some(partial) = partial_type {
            if !type_name.starts_with(partial) {
                continue;
            }
        }
        
        completions.push(CompletionItem {
            label: type_name,
            kind: Some(CompletionItemKind::CLASS),
            detail: Some("Type definition".to_string()),
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
        let last_segment = &path[path.len()-1];
        
        // Check if we're in an array element context
        if last_segment.ends_with("[]") {
            // Strip array brackets to get field name
            let field_name = last_segment.trim_end_matches("[]");
            let parent_path = &path[..path.len()-1];
            
            let parent_obj = if parent_path.is_empty() {
                &schema.root
            } else {
                match lookup_schema_at_path_with_context(parent_path, &schema.root, Some(schema)) {
                    Some(obj) => obj,
                    None => {
                        return vec![];
                    }
                }
            };
            
            // Look for the field in the parent
            let field_key = KeyCmpValue::String(field_name.to_string());
            if let Some(field_schema) = parent_obj.fields.get(&field_key) {
                
                // Check if it's an array
                if let Type::Array(elem_type) = &field_schema.type_expr {
                    
                    // Check if the element type is a variant type reference
                    if let Type::TypeRef(type_ref) = elem_type.as_ref() {
                        
                        // Look up the referenced type
                        return lookup_and_generate_variant_completions(type_ref, schema);
                    }
                }
            }
        } else {
            // Regular path - check if it's a field with variant type
            let parent_path = &path[..path.len()-1];
            let field_name = last_segment;
            
            let parent_obj = if parent_path.is_empty() {
                &schema.root
            } else {
                match lookup_schema_at_path_with_context(parent_path, &schema.root, Some(schema)) {
                    Some(obj) => obj,
                    None => return vec![],
                }
            };
            
            // Look for the field in the parent
            let field_key = KeyCmpValue::String(field_name.clone());
            if let Some(field_schema) = parent_obj.fields.get(&field_key) {
                // Check if it's a type reference  
                if let Type::TypeRef(type_ref) = &field_schema.type_expr {
                    return lookup_and_generate_variant_completions(type_ref, schema);
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

fn lookup_and_generate_variant_completions(
    type_ref: &Identifier,
    schema: &DocumentSchema,
) -> Vec<CompletionItem> {
    // Look up the referenced type
    let type_str = type_ref.as_ref();
    
    // Handle both $types.TypeName and plain TypeName references
    let type_name = if type_str.starts_with("$types.") {
        &type_str[7..] // Skip "$types."
    } else {
        type_str
    };
    
    // Convert Identifier to KeyCmpValue for lookup
    let type_key = KeyCmpValue::String(type_name.to_string());
    
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

fn is_variant_path(path: &[String], schema: &DocumentSchema) -> bool {
    if path.is_empty() {
        return false;
    }
    
    let mut current = &schema.root;
    
    for (i, segment) in path.iter().enumerate() {
        let key = KeyCmpValue::String(segment.clone());
        
        if let Some(field) = current.fields.get(&key) {
            if i == path.len() - 1 {
                // Last segment - check if it's a variant type
                if let Type::TypeRef(type_ref) = &field.type_expr {
                    return lookup_variant_schema(type_ref, schema).is_some();
                }
                return false;
            }
            
            // Not last segment - continue traversing
            match &field.type_expr {
                Type::Object(obj) => current = obj,
                Type::Array(element_type) => {
                    if let Type::Object(obj) = element_type.as_ref() {
                        current = obj;
                    } else {
                        return false;
                    }
                }
                _ => return false,
            }
        } else {
            return false;
        }
    }
    
    false
}

fn lookup_variant_schema<'a>(
    type_ref: &Identifier,
    schema: &'a DocumentSchema,
) -> Option<&'a VariantSchema> {
    let type_str = type_ref.as_ref();
    
    // Handle both $types.TypeName and plain TypeName references
    let type_name = if type_str.starts_with("$types.") {
        &type_str[7..] // Skip "$types."
    } else {
        type_str
    };
    
    
    // Look up the type definition
    if let Some(field_schema) = schema.types.get(&KeyCmpValue::String(type_name.to_string())) {
        // Check if it's a variant type
        if let Type::Variants(variant_schema) = &field_schema.type_expr {
            return Some(variant_schema);
        } else {
        }
    } else {
    }
    None
}

fn lookup_schema_at_path<'a>(
    path: &[String],
    root: &'a ObjectSchema,
) -> Option<&'a ObjectSchema> {
    lookup_schema_at_path_with_context(path, root, None)
}

fn lookup_schema_at_path_with_context<'a>(
    path: &[String],
    root: &'a ObjectSchema,
    schema: Option<&'a DocumentSchema>,
) -> Option<&'a ObjectSchema> {
    let mut current = root;
    
    for segment in path {
        // Handle array syntax - strip [] to get the field name
        let field_name = segment.trim_end_matches("[]");
        let _is_array = segment.ends_with("[]");
        
        // Look up the field
        let key = KeyCmpValue::String(field_name.to_string());
        if let Some(field) = current.fields.get(&key) {
            match &field.type_expr {
                Type::Object(obj) => {
                    current = obj;
                }
                Type::Array(elem_type) => {
                    // For arrays, look at the element type
                    match elem_type.as_ref() {
                        Type::Object(obj) => {
                            current = obj;
                        }
                        Type::TypeRef(type_ref) => {
                            // Look up the referenced type
                            if let Some(schema) = schema {
                                if let Some(resolved_obj) = resolve_type_ref(type_ref, schema) {
                                    current = resolved_obj;
                                } else {
                                    return None;
                                }
                            } else {
                                return None;
                            }
                        }
                        _ => {
                            return None;
                        }
                    }
                }
                _ => {
                    return None;
                }
            }
        } else {
            return None;
        }
    }
    
    Some(current)
}

fn resolve_type_ref<'a>(type_ref: &Identifier, schema: &'a DocumentSchema) -> Option<&'a ObjectSchema> {
    let type_str = type_ref.as_ref();
    
    // Handle both $types.TypeName and plain TypeName references
    let type_name = if type_str.starts_with("$types.") {
        &type_str[7..] // Skip "$types."
    } else {
        type_str
    };
    
    // Look up the type definition
    if let Some(field_schema) = schema.types.get(&KeyCmpValue::String(type_name.to_string())) {
        // Check if it's an object type
        if let Type::Object(obj) = &field_schema.type_expr {
            return Some(obj);
        }
    }
    
    None
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
    
    // Handle extensions vs meta-extensions vs regular fields
    if field_name.starts_with("$$") {
        // Meta-extensions ($$name) are stored as KeyCmpValue::MetaExtension
        if let Ok(id) = Identifier::from_str(&field_name[2..]) {
            let key = KeyCmpValue::MetaExtension(id);
            object.fields.get(&key).map(|field| &field.type_expr)
        } else {
            None
        }
    } else if field_name.starts_with('$') {
        // Regular extensions ($variant) are stored in Node.extensions, not schema.fields
        // They should not be looked up here at all
        None
    } else {
        // Regular data fields use String keys
        let key = KeyCmpValue::String(field_name.to_string());
        object.fields.get(&key).map(|field| &field.type_expr)
    }
}

fn find_used_fields_in_section(text: &str, position: Position, section_path: &[String]) -> HashSet<String> {
    let mut used_fields = HashSet::new();
    let lines: Vec<&str> = text.lines().collect();
    
    // If we're at root level, scan entire document for top-level fields
    if section_path.is_empty() {
        for line in lines.iter() {
            let trimmed = line.trim();
            // Skip comments, empty lines, and section declarations
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('@') {
                continue;
            }
            // Look for field assignments at root level
            if let Some(eq_pos) = trimmed.find('=') {
                let field_name = trimmed[..eq_pos].trim();
                if !field_name.contains('.') {
                    used_fields.insert(field_name.to_string());
                }
            }
        }
        return used_fields;
    }
    
    // For section context, find the section start and scan until current position
    let mut in_section = false;
    // Handle array syntax in section paths (e.g., requests[] becomes requests)
    let normalized_path: Vec<String> = section_path.iter()
        .map(|s| s.replace("[]", ""))
        .collect();
    let section_pattern = format!("@ {}", normalized_path.join("."));
    
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        
        // Check if we've found our section - handle both exact match and array syntax
        // We need to match patterns like:
        // "@ requests.status" against "@ requests[].status"
        // So we normalize both patterns by removing array brackets for comparison
        let trimmed_normalized = trimmed.replace("[]", "");
        if trimmed_normalized == section_pattern {
            in_section = true;
            continue;
        }
        
        // If we're in the section and haven't reached cursor position yet
        if in_section && i <= position.line as usize {
            // Check for end of section (another @ declaration)
            if trimmed.starts_with('@') && trimmed != section_pattern {
                // We've exited our section
                break;
            }
            
            // Check for variant declaration
            if trimmed.starts_with("$variant:") {
                // Clear previously found fields when entering a variant context
                // because we only want fields specific to this variant
                used_fields.clear();
                continue;
            }
            
            // Look for field assignments
            if !trimmed.is_empty() && !trimmed.starts_with('#') && trimmed.contains('=') {
                if let Some(eq_pos) = trimmed.find('=') {
                    let field_name = trimmed[..eq_pos].trim();
                    // Make sure this is a direct field (not a nested path)
                    if !field_name.contains('.') {
                        used_fields.insert(field_name.to_string());
                    }
                }
            }
        }
        
        // Stop if we've passed the cursor position
        if i > position.line as usize {
            break;
        }
    }
    
    used_fields
}

fn find_variant_context(text: &str, position: Position, section_path: &[String]) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    
    // For variant context, find the section start and look for $variant declaration
    let mut in_section = false;
    let section_pattern = if section_path.is_empty() {
        None
    } else {
        // Handle array syntax in section paths
        let normalized_path: Vec<String> = section_path.iter()
            .map(|s| s.replace("[]", ""))
            .collect();
        Some(format!("@ {}", normalized_path.join(".")))
    };
    
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        
        // Check if we've found our section (or at root if no section pattern)
        if let Some(ref pattern) = section_pattern {
            // Check if this line matches our section pattern (with array syntax variations)
            let pattern_parts: Vec<&str> = pattern.trim_start_matches("@ ").split('.').collect();
            let line_parts: Vec<&str> = if trimmed.starts_with("@ ") {
                trimmed.trim_start_matches("@ ").split('.').collect()
            } else {
                vec![]
            };
            
            if !line_parts.is_empty() && pattern_parts.len() == line_parts.len() {
                let mut matches = true;
                for (pattern_part, line_part) in pattern_parts.iter().zip(line_parts.iter()) {
                    // Handle array syntax - requests[] matches requests
                    let normalized_line_part = line_part.trim_end_matches("[]");
                    if pattern_part != &normalized_line_part {
                        matches = false;
                        break;
                    }
                }
                if matches {
                    in_section = true;
                    continue;
                }
            }
        } else {
            // At root level
            in_section = true;
        }
        
        // If we're in the section and haven't reached cursor position yet
        if in_section && i <= position.line as usize {
            // Check for end of section (another @ declaration)
            if trimmed.starts_with('@') && section_pattern.is_some() {
                let matches_pattern = section_pattern.as_ref().map(|p| 
                    trimmed == p || trimmed == format!("{}[]", p) || trimmed.starts_with(&format!("{}", p))
                ).unwrap_or(false);
                
                if !matches_pattern {
                    break;
                }
            }
            
            // Look for $variant declaration
            if trimmed.starts_with("$variant:") || trimmed.starts_with("$variant =") {
                let variant_part = if trimmed.contains(':') {
                    trimmed.split(':').nth(1)
                } else {
                    trimmed.split('=').nth(1)
                };
                
                if let Some(variant_name) = variant_part {
                    let variant_name = variant_name.trim().trim_matches('"');
                    return Some(variant_name.to_string());
                }
            }
        }
        
        // Stop if we've passed the cursor position
        if i > position.line as usize {
            break;
        }
    }
    
    None
}

fn find_section_context(text: &str, position: Position) -> Option<Vec<String>> {
    let lines: Vec<&str> = text.lines().collect();
    let mut path_stack: Vec<Vec<String>> = vec![];
    let mut brace_depth: i32 = 0;
    
    // Scan from the beginning to track nested sections
    for i in 0..=position.line as usize {
        if i >= lines.len() {
            break;
        }
        
        let line = lines[i];
        let trimmed = line.trim();
        
        // Count braces
        for ch in line.chars() {
            match ch {
                '{' => brace_depth += 1,
                '}' => {
                    brace_depth = brace_depth.saturating_sub(1);
                    // Pop from path stack when we exit a block
                    if !path_stack.is_empty() && (brace_depth as usize) < path_stack.len() {
                        path_stack.pop();
                    }
                }
                _ => {}
            }
        }
        
        // Check for section declarations
        if trimmed.starts_with('@') && !trimmed.contains('=') {
            let section_part = trimmed.trim_start_matches('@').trim();
            let section_part = section_part.trim_end_matches('{').trim();
            
            if !section_part.is_empty() {
                let path_parts: Vec<String> = section_part
                    .split('.')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                
                if !path_parts.is_empty() {
                    // Determine at what level to insert this path
                    let level = if line.trim_start().starts_with('@') {
                        // Count leading spaces/tabs to determine nesting
                        let indent = line.len() - line.trim_start().len();
                        indent / 4 // Assuming 4 spaces per indent level
                    } else {
                        0
                    };
                    
                    // Adjust path stack based on nesting level
                    while path_stack.len() > level {
                        path_stack.pop();
                    }
                    
                    path_stack.push(path_parts);
                }
            }
        }
    }
    
    // Flatten the path stack into a single path
    if !path_stack.is_empty() {
        let mut result = vec![];
        for path in &path_stack {
            result.extend(path.clone());
        }
        Some(result)
    } else {
        None
    }
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

// CST-Based Position Analysis Functions

/// Convert LSP Position to byte offset in text
fn position_to_byte_offset(text: &str, position: Position) -> usize {
    let lines: Vec<&str> = text.lines().collect();
    let mut offset = 0;
    
    // Add bytes for complete lines before the target line
    for i in 0..(position.line as usize).min(lines.len()) {
        offset += lines[i].len() + 1; // +1 for newline character
    }
    
    // Add bytes for characters on the target line
    if let Some(line) = lines.get(position.line as usize) {
        offset += (position.character as usize).min(line.len());
    }
    
    // Subtract 1 if we counted a newline that doesn't exist (at end of file)
    if position.line as usize >= lines.len() && !text.is_empty() && !text.ends_with('\n') {
        offset = offset.saturating_sub(1);
    }
    
    offset.min(text.len())
}

/// Find the CST node that contains the given byte position
fn find_node_at_position(cst: &Cst, position_byte: usize) -> Option<CstNodeId> {
    find_node_at_position_recursive(cst, cst.root(), position_byte)
}

fn find_node_at_position_recursive(cst: &Cst, node_id: CstNodeId, position_byte: usize) -> Option<CstNodeId> {
    // Check if this node contains the position
    if let Some(span) = get_node_span(cst, node_id) {
        if position_byte < span.start as usize || position_byte > span.end as usize {
            return None; // Position is outside this node
        }
    }
    
    // Check all children to find the most specific node
    let mut best_match = Some(node_id);
    for child_id in cst.children(node_id) {
        if let Some(child_match) = find_node_at_position_recursive(cst, child_id, position_byte) {
            best_match = Some(child_match);
        }
    }
    
    best_match
}

/// Extract the InputSpan from a CST node if it has one
fn get_node_span(cst: &Cst, node_id: CstNodeId) -> Option<InputSpan> {
    if let Some(node_data) = cst.node_data(node_id) {
        match node_data {
            CstNodeData::Terminal { data: TerminalData::Input(span), .. } => Some(span),
            CstNodeData::NonTerminal { data: NonTerminalData::Input(span), .. } => Some(span),
            _ => None,
        }
    } else {
        None
    }
}

// CST-Based Context Analysis

#[derive(Debug, PartialEq)]
enum CompletionContextType {
    /// Completing extension names after $ (e.g., $variant)
    Extension,
    /// Completing meta-extension names after $$ (e.g., $$variants)
    MetaExtension,
    /// Completing field names in object context
    Field,
    /// Completing values after = or :
    Value,
    /// Unknown context
    Unknown,
}

/// Analyze the CST context at the given position to determine completion type
fn analyze_cst_context(cst: &Cst, position: Position, text: &str) -> (CompletionContextType, Vec<String>) {
    let position_byte = position_to_byte_offset(text, position);
    
    if let Some(node_id) = find_node_at_position(cst, position_byte) {
        // Walk up the CST to find the context
        let context_type = determine_context_type(cst, node_id);
        let path = extract_document_path_from_cst(cst, node_id);
        
        (context_type, path)
    } else {
        (CompletionContextType::Unknown, vec![])
    }
}

/// Determine the completion context type based on CST node
fn determine_context_type(cst: &Cst, node_id: CstNodeId) -> CompletionContextType {
    // Check this node and its ancestors for context clues
    let mut current_id = Some(node_id);
    
    while let Some(id) = current_id {
        if let Some(node_data) = cst.node_data(id) {
            match node_data {
                CstNodeData::NonTerminal { kind: NonTerminalKind::Ext, .. } => {
                    // We're in an extension context ($variant)
                    return CompletionContextType::Extension;
                }
                CstNodeData::NonTerminal { kind: NonTerminalKind::MetaExt, .. } => {
                    // We're in a meta-extension context ($$variants)
                    return CompletionContextType::MetaExtension;
                }
                CstNodeData::Terminal { kind: TerminalKind::Dollar, .. } => {
                    // Found a $ token, likely extension context
                    return CompletionContextType::Extension;
                }
                CstNodeData::Terminal { kind: TerminalKind::DollarDollar, .. } => {
                    // Found a $$ token, meta-extension context
                    return CompletionContextType::MetaExtension;
                }
                CstNodeData::Terminal { kind: TerminalKind::Bind, .. } => {
                    // Found = token, value context
                    return CompletionContextType::Value;
                }
                _ => {}
            }
        }
        
        // Move to parent
        current_id = cst.parent(id);
    }
    
    // Default to field completion if no specific context found
    CompletionContextType::Field
}

/// Extract document path from CST context (placeholder implementation)
fn extract_document_path_from_cst(cst: &Cst, node_id: CstNodeId) -> Vec<String> {
    // TODO: Implement proper path extraction from CST
    // For now, fall back to the existing string-based approach
    vec![]
}

/// Generate completions using CST-based context analysis
fn get_cst_based_completions(
    cst: &Cst,
    position: Position,
    text: &str,
    schema: &DocumentSchema,
) -> Vec<CompletionItem> {
    let (context_type, path) = analyze_cst_context(cst, position, text);
    
    
    match context_type {
        CompletionContextType::Extension => {
            // Generate extension completions (like $variant)
            generate_extension_completions(&path, schema)
        }
        CompletionContextType::MetaExtension => {
            // Generate meta-extension completions (like $$variants)
            generate_meta_extension_completions(&path, schema)
        }
        CompletionContextType::Field => {
            // Generate field completions
            generate_field_completions(&path, schema, None, &HashSet::new())
        }
        CompletionContextType::Value => {
            // Generate value completions - need to determine field name from CST
            // For now, fall back to empty
            vec![]
        }
        CompletionContextType::Unknown => {
            // Fall back to string-based approach
            vec![]
        }
    }
}

/// Generate extension completions (for $ context)
fn generate_extension_completions(path: &[String], schema: &DocumentSchema) -> Vec<CompletionItem> {
    let mut completions = vec![];
    
    // Add standard extensions
    completions.push(CompletionItem {
        label: "variant".to_string(),
        kind: Some(CompletionItemKind::PROPERTY),
        detail: Some("Extension: variant selection".to_string()),
        documentation: Some(lsp_types::Documentation::String(
            "Specify the variant for sum types".to_string()
        )),
        ..Default::default()
    });
    
    // Check if we're in a variant context and can provide variant names
    if let Some(variant_schema) = lookup_variant_schema_at_path(path, &schema.root) {
        // For $variant specifically, provide the variant names as values
        let variant_completions = generate_variant_completion_items(variant_schema);
        completions.extend(variant_completions);
    }
    
    completions
}

/// Generate meta-extension completions (for $$ context)
fn generate_meta_extension_completions(path: &[String], _schema: &DocumentSchema) -> Vec<CompletionItem> {
    let mut completions = vec![];
    
    // Add standard meta-extensions
    completions.push(CompletionItem {
        label: "variants".to_string(),
        kind: Some(CompletionItemKind::PROPERTY),
        detail: Some("Meta-extension: variant definitions".to_string()),
        ..Default::default()
    });
    
    completions
}

/// Handle-based completion using the EureDocument index
fn get_handle_based_completions(
    text: &str,
    cst: &Cst,
    position: Position,
    uri: &str,
    schema_manager: &SchemaManager,
    document: &EureDocument,
) -> Option<Vec<CompletionItem>> {
    // Convert position to byte offset
    let byte_offset = position_to_byte_offset(text, position);
    
    // Find CST node at cursor position
    let cst_node_id = find_node_at_position(cst, byte_offset)?;
    
    
    // Try to find the document node by CST handle
    if let Some(document_node) = document.get_node_by_cst_id(cst_node_id) {
        
        // Get the path to this node
        if let Some(path) = document.get_path_by_cst_id(cst_node_id) {
            
            // Get schema
            let schema_uri = schema_manager.get_document_schema_uri(uri)?;
            let schema = schema_manager.get_schema(schema_uri)?;
            
            // Convert PathSegment to String for compatibility
            let string_path: Vec<String> = path.iter().filter_map(|seg| {
                match seg {
                    eure_value::value::PathSegment::Ident(id) => Some(id.as_ref().to_string()),
                    _ => None, // For now, only handle identifier segments
                }
            }).collect();
            
            // Determine completion context based on position and CST
            let context = analyze_handle_completion_context(cst, cst_node_id, byte_offset, text);
            
            // Generate completions based on context
            match context {
                HandleCompletionContext::FieldPosition { parent_path: _ } => {
                    // Complete field names at the current location
                    Some(generate_field_completions(&string_path, schema, None, &HashSet::new()))
                }
                HandleCompletionContext::ValuePosition { field_path: _, field_name } => {
                    // Complete values for the specific field
                    if field_name == "variant" {
                        // Special case for variant completions
                        Some(generate_variant_completions(&string_path, schema))
                    } else {
                        Some(generate_value_completions(&string_path, &field_name, schema, false))
                    }
                }
                HandleCompletionContext::ExtensionPosition { parent_path: _ } => {
                    // Complete extension names like $variant
                    Some(generate_extension_completions(&string_path, schema))
                }
                HandleCompletionContext::Unknown => {
                    None
                }
            }
        } else {
            None
        }
    } else {
        
        // Try to find the nearest parent node and infer context
        let parent_cst_id = find_nearest_parent_with_document_node(cst, cst_node_id, document)?;
        
        if let Some(parent_path) = document.get_path_by_cst_id(parent_cst_id) {
            
            // Convert PathSegment to String for compatibility
            let string_path: Vec<String> = parent_path.iter().filter_map(|seg| {
                match seg {
                    eure_value::value::PathSegment::Ident(id) => Some(id.as_ref().to_string()),
                    _ => None, // For now, only handle identifier segments
                }
            }).collect();
            
            // Get schema
            let schema_uri = schema_manager.get_document_schema_uri(uri)?;
            let schema = schema_manager.get_schema(schema_uri)?;
            
            // Infer that we're in field position within the parent
            Some(generate_field_completions(&string_path, schema, None, &HashSet::new()))
        } else {
            None
        }
    }
}

#[derive(Debug)]
enum HandleCompletionContext {
    /// Completing field names at the given parent path
    FieldPosition { parent_path: Vec<String> },
    /// Completing values for a specific field
    ValuePosition { field_path: Vec<String>, field_name: String },
    /// Completing extension names (like $variant)
    ExtensionPosition { parent_path: Vec<String> },
    /// Unknown context
    Unknown,
}

/// Analyze the completion context based on CST structure and cursor position
fn analyze_handle_completion_context(
    cst: &Cst,
    node_id: CstNodeId,
    byte_offset: usize,
    text: &str,
) -> HandleCompletionContext {
    // Look at the cursor position in the text to understand context
    let line_start = text[..byte_offset].rfind('\n').map(|pos| pos + 1).unwrap_or(0);
    let line_end = text[byte_offset..].find('\n').map(|pos| byte_offset + pos).unwrap_or(text.len());
    let line = &text[line_start..line_end];
    let cursor_in_line = byte_offset - line_start;
    let before_cursor = &line[..cursor_in_line];
    
    
    // Check what's immediately before cursor
    let trimmed = before_cursor.trim_end();
    
    if trimmed.ends_with('=') || trimmed.ends_with(':') {
        // We're in value position
        // Extract field name from the line
        if let Some(field_name) = extract_field_name_before_operator(trimmed) {
            // For now, use empty path - we'd need more sophisticated analysis for nested paths
            HandleCompletionContext::ValuePosition { 
                field_path: vec![], 
                field_name 
            }
        } else {
            HandleCompletionContext::Unknown
        }
    } else if trimmed.ends_with('@') || trimmed.is_empty() || before_cursor.ends_with(' ') {
        // We're in field position
        HandleCompletionContext::FieldPosition { parent_path: vec![] }
    } else if trimmed.starts_with('$') {
        // We're potentially in extension position
        HandleCompletionContext::ExtensionPosition { parent_path: vec![] }
    } else {
        HandleCompletionContext::Unknown
    }
}

/// Extract field name from text before = or : operator
fn extract_field_name_before_operator(text: &str) -> Option<String> {
    let without_operator = if text.ends_with('=') {
        &text[..text.len()-1]
    } else if text.ends_with(':') {
        &text[..text.len()-1]
    } else {
        text
    };
    
    // Find the last word (field name)
    let parts: Vec<&str> = without_operator.trim().split_whitespace().collect();
    parts.last().map(|s| s.to_string())
}

/// Find the nearest parent CST node that has a corresponding document node
fn find_nearest_parent_with_document_node(
    cst: &Cst, 
    start_node: CstNodeId, 
    document: &EureDocument
) -> Option<CstNodeId> {
    let mut current = Some(start_node);
    
    while let Some(node_id) = current {
        if document.get_node_by_cst_id(node_id).is_some() {
            return Some(node_id);
        }
        current = cst.parent(node_id);
    }
    
    None
}