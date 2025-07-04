use eure_schema::{DocumentSchema, KeyCmpValue, Type, ObjectSchema};
use eure_tree::Cst;
use lsp_types::{CompletionItem, CompletionItemKind, Documentation, InsertTextFormat, MarkupContent, MarkupKind, Position};

use crate::schema_validation::SchemaManager;

#[derive(Debug, Clone)]
pub struct CompletionContext {
    pub position: Position,
    pub trigger_character: Option<String>,
    pub current_path: Vec<String>,
    pub is_in_value_position: bool,
    pub is_in_key_position: bool,
    pub is_variant_position: bool,
    pub is_string_only: bool, // True after ":", false after "="
    pub parent_path: Option<String>, // e.g., "user" when completing "user."
}

pub fn get_completions(
    text: &str,
    cst: &Cst,
    position: Position,
    trigger_character: Option<String>,
    uri: &str,
    schema_manager: &SchemaManager,
) -> Vec<CompletionItem> {
    // First, analyze the context at the cursor position
    let context = analyze_context(text, cst, position, trigger_character);
    
    // Get the schema for this document
    let schema_uri = schema_manager.get_document_schema_uri(uri);
    let schema = schema_uri.and_then(|uri| schema_manager.get_schema(uri));
    
    if let Some(schema) = schema {
        generate_completions(&context, schema)
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
    _cst: &Cst,
    position: Position,
    trigger_character: Option<String>,
) -> CompletionContext {
    // Convert position to byte offset
    let _byte_offset = position_to_byte_offset(text, position);
    
    // Get the line at the cursor position
    let lines: Vec<&str> = text.lines().collect();
    if position.line as usize >= lines.len() {
        return CompletionContext {
            position,
            trigger_character: trigger_character.clone(),
            current_path: vec![],
            is_in_value_position: false,
            is_in_key_position: true,
            is_variant_position: false,
            is_string_only: false,
            parent_path: None,
        };
    }
    
    let current_line = lines[position.line as usize];
    let char_pos = position.character.min(current_line.len() as u32) as usize;
    let line_before_cursor = if char_pos > 0 {
        &current_line[..char_pos]
    } else {
        ""
    };
    
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
                    let parent_ident = before_dot.trim().split_whitespace().last();
                    parent_path = parent_ident.map(|s| s.to_string());
                }
            }
            ":" => {
                // Colon is for string-only binding
                is_in_value_position = true;
                is_string_only = true;
                // Check if we're after $variant
                // Remove the trigger character to check what came before it
                let line_before_trigger = if line_before_cursor.len() > 0 {
                    &line_before_cursor[..line_before_cursor.len() - 1]
                } else {
                    ""
                };
                if line_before_trigger.trim_end().ends_with("$variant") {
                    is_variant_position = true;
                }
            }
            "=" => {
                // Equals is for any value binding
                is_in_value_position = true;
                // Check if we're after $variant
                // Remove the trigger character to check what came before it
                let line_before_trigger = if line_before_cursor.len() > 0 {
                    &line_before_cursor[..line_before_cursor.len() - 1]
                } else {
                    ""
                };
                if line_before_trigger.trim_end().ends_with("$variant") {
                    is_variant_position = true;
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
            } else if trimmed.ends_with('=') {
                is_in_value_position = true;
                is_string_only = false;
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
    
    // TODO: Parse the current path in the document structure
    
    CompletionContext {
        position,
        trigger_character,
        current_path: vec![],
        is_in_value_position,
        is_in_key_position,
        is_variant_position,
        is_string_only,
        parent_path,
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
                KeyCmpValue::Extension(e) => format!("${}", e) == name,
                KeyCmpValue::MetaExtension(m) => format!("$${}", m) == name,
                _ => false,
            }
        })
        .map(|(_, field)| field)
}

fn generate_completions(context: &CompletionContext, schema: &DocumentSchema) -> Vec<CompletionItem> {
    let mut completions = Vec::new();
    
    // If we're in a key position, suggest available fields
    if context.is_in_key_position {
        // Determine which schema to use based on parent path
        let (fields_to_complete, parent_has_section_preference) = if let Some(ref parent) = context.parent_path {
            // Look up the parent field in the schema
            if let Some(parent_field) = lookup_field_by_name(&schema.root, parent) {
                // Check if parent has section preference
                let has_section_pref = parent_field.preferences.section.unwrap_or(false);
                
                // If parent is an object type, use its fields
                if let Type::Object(ref obj_schema) = parent_field.type_expr {
                    (&obj_schema.fields, has_section_pref)
                } else {
                    // Parent is not an object, no fields to complete
                    return completions;
                }
            } else {
                // Parent field not found
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
                KeyCmpValue::Extension(e) => format!("${}", e),
                KeyCmpValue::MetaExtension(m) => format!("$${}", m),
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
        // Check if we're completing a variant field
        if context.is_variant_position {
            // TODO: Once we have proper path tracking, we can find the specific variant schema
            // For now, provide a placeholder message
            completions.push(CompletionItem {
                label: "variant_name".to_string(),
                kind: Some(CompletionItemKind::ENUM_MEMBER),
                detail: Some("Variant name (context-aware suggestions coming soon)".to_string()),
                documentation: None,
                deprecated: Some(false),
                preselect: Some(false),
                ..Default::default()
            });
        } else if context.is_string_only {
            // After ":", only string values are allowed
            // TODO: Provide string-specific completions based on field type
        } else {
            // After "=", any value is allowed
            // TODO: Determine the field type based on context.current_path
            
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
        format!("{}.{}", parent_path, field_name)
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
                KeyCmpValue::Extension(e) => format!("${}", e),
                KeyCmpValue::MetaExtension(m) => format!("$${}", m),
                _ => continue, // Skip other types
            };
            
            // Add the field with a hole value
            snippet.push_str(&format!("{} = ${{{}:!}}\n", key_str, tab_stop));
            tab_stop += 1;
    }
    
    // Add final cursor position
    snippet.push_str("$0");
    
    Some(snippet)
}

