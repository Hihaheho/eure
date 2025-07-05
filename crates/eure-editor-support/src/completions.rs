use eure_schema::{DocumentSchema, KeyCmpValue, Type, ObjectSchema, FieldSchema};
use eure_value::value::PathSegment;
use eure_tree::{Cst, value_visitor::Values};
use lsp_types::{CompletionItem, CompletionItemKind, Documentation, InsertTextFormat, MarkupContent, MarkupKind, Position};
use indexmap::IndexMap;

use crate::schema_validation::SchemaManager;
use crate::completion_context_tracker::{CompletionContextTracker, CompletionContext};
use crate::completion_analyzer::CompletionAnalyzer;

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
    
    // Get the schema for this document
    let schema_uri = schema_manager.get_document_schema_uri(uri);
    let schema = match schema_uri.and_then(|uri| schema_manager.get_schema(uri)) {
        Some(s) => s,
        None => return vec![], // No schema, no completions
    };
    
    // Extract completion context using the new tracker
    let mut values = Values::default();
    let mut value_visitor = eure_tree::value_visitor::ValueVisitor::new(text, &mut values);
    let _ = cst.visit_from_root(&mut value_visitor);
    
    let context_tracker = CompletionContextTracker::new(text, &values, position);
    let context = match context_tracker.track_context(cst) {
        Some(ctx) => ctx,
        None => return vec![], // Couldn't determine context
    };
    
    generate_completions(&context, schema, trigger_character)
}

fn generate_completions(
    context: &CompletionContext,
    schema: &DocumentSchema,
    trigger_character: Option<String>,
) -> Vec<CompletionItem> {
    let mut completions = Vec::new();
    
    if context.is_in_key_position {
        // Get fields at the current path
        let fields = match lookup_fields_at_path(&context.path_segments, schema, &context.variant_contexts) {
            Some(fields) => fields,
            None => return vec![], // Invalid path, no completions
        };
        
        // Generate completion items for each field
        for (key, field_schema) in fields {
            let label = format_key(key);
            
            // Skip if field is already used
            if context.used_fields.contains(&label) {
                continue;
            }
            
            // Skip if partial field is specified and doesn't match
            if let Some(ref partial) = context.partial_field
                && !label.starts_with(partial) {
                    continue;
                }
            
            let documentation = field_schema.description.as_ref().map(|desc| {
                Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: desc.clone(),
                })
            });
            
            let mut completion_item = CompletionItem {
                label: label.clone(),
                kind: Some(CompletionItemKind::FIELD),
                detail: Some(format_type_detail(&field_schema.type_expr)),
                documentation,
                deprecated: Some(false),
                preselect: Some(false),
                ..Default::default()
            };
            
            // Add snippet for object fields with required children
            if trigger_character.as_deref() == Some(".")
                && let Type::Object(ref obj_schema) = field_schema.type_expr
                    && let Some(snippet) = generate_section_snippet(&label, obj_schema) {
                        completion_item.insert_text_format = Some(InsertTextFormat::SNIPPET);
                        completion_item.insert_text = Some(snippet);
                    }
            
            completions.push(completion_item);
        }
    } else if context.is_in_value_position {
        if context.is_variant_position {
            // Get variant completions for the current field
            let variant_type = find_variant_type_at_path(&context.path_segments, schema);
            if let Some(Type::Variants(variants)) = variant_type {
                for (variant_key, variant_obj) in &variants.variants {
                    if let KeyCmpValue::String(variant_name) = variant_key {
                        let field_info = if variant_obj.fields.is_empty() {
                            "No fields".to_string()
                        } else {
                            let field_names: Vec<String> = variant_obj.fields.keys()
                                .map(format_key)
                                .collect();
                            format!("Fields: {}", field_names.join(", "))
                        };
                        
                        completions.push(CompletionItem {
                            label: variant_name.clone(),
                            kind: Some(CompletionItemKind::ENUM_MEMBER),
                            detail: Some(format!("Variant: {variant_name}")),
                            documentation: Some(Documentation::MarkupContent(MarkupContent {
                                kind: MarkupKind::Markdown,
                                value: field_info,
                            })),
                            deprecated: Some(false),
                            preselect: Some(false),
                            ..Default::default()
                        });
                    }
                }
            }
        } else {
            // Generate value completions based on expected type
            let field_type = find_field_type_at_path(&context.path_segments, schema);
            completions.extend(generate_value_completions(field_type, context.is_string_only));
        }
    }
    
    completions
}

/// Navigate through the schema to find fields at the given path
fn lookup_fields_at_path<'a>(
    path: &[PathSegment],
    schema: &'a DocumentSchema,
    variant_contexts: &std::collections::HashMap<String, String>,
) -> Option<&'a IndexMap<KeyCmpValue, FieldSchema>> {
    let mut current_fields = &schema.root.fields;
    let mut current_path = Vec::new();
    
    for (i, segment) in path.iter().enumerate() {
        match segment {
            PathSegment::Ident(id) => {
                let field_name = id.as_ref();
                
                // Look up the field
                let field = current_fields.iter()
                    .find(|(key, _)| matches_key(key, field_name))
                    .map(|(_, field)| field)?;
                
                current_path.push(segment.clone());
                
                // Navigate based on field type
                match &field.type_expr {
                    Type::Object(obj) => {
                        current_fields = &obj.fields;
                    }
                    Type::Array(elem_type) => {
                        // If next segment is array access, handle it
                        if i + 1 < path.len() && matches!(path[i + 1], PathSegment::Array { .. }) {
                            // Skip to element type
                            match elem_type.as_ref() {
                                Type::Object(obj) => {
                                    current_fields = &obj.fields;
                                }
                                Type::TypeRef(type_name) => {
                                    current_fields = resolve_type_fields(type_name, schema, &current_path, variant_contexts)?;
                                }
                                Type::Variants(_) => {
                                    // Need variant context for array of variants
                                    let path_key = path_to_string(&current_path);
                                    let variant = variant_contexts.get(&path_key)?;
                                    current_fields = resolve_variant_fields(elem_type, variant, schema)?;
                                }
                                _ => return None,
                            }
                        } else {
                            return None; // Array field but no array access
                        }
                    }
                    Type::TypeRef(type_name) => {
                        current_fields = resolve_type_fields(type_name, schema, &current_path, variant_contexts)?;
                    }
                    Type::Variants(variants) => {
                        // Get variant context for this path
                        let path_key = path_to_string(&current_path);
                        let variant = variant_contexts.get(&path_key)?;
                        let variant_obj = variants.variants.get(&KeyCmpValue::String(variant.clone()))?;
                        current_fields = &variant_obj.fields;
                    }
                    _ => return None, // Primitive type, no nested fields
                }
            }
            PathSegment::Array { .. } => {
                // Array access already handled in previous iteration
                current_path.push(segment.clone());
            }
            PathSegment::Extension(ext) => {
                let field_name = format!("${}", ext.as_ref());
                
                // Look up extension field
                let field = current_fields.iter()
                    .find(|(key, _)| matches_key(key, &field_name))
                    .map(|(_, field)| field)?;
                
                current_path.push(segment.clone());
                
                // Navigate based on field type
                match &field.type_expr {
                    Type::Object(obj) => {
                        current_fields = &obj.fields;
                    }
                    Type::TypeRef(type_name) => {
                        current_fields = resolve_type_fields(type_name, schema, &current_path, variant_contexts)?;
                    }
                    _ => return None,
                }
            }
            PathSegment::MetaExt(meta) => {
                let field_name = format!("$${}", meta.as_ref());
                
                // Look up meta extension field
                let field = current_fields.iter()
                    .find(|(key, _)| matches_key(key, &field_name))
                    .map(|(_, field)| field)?;
                
                current_path.push(segment.clone());
                
                // Navigate based on field type
                match &field.type_expr {
                    Type::Object(obj) => {
                        current_fields = &obj.fields;
                    }
                    Type::TypeRef(type_name) => {
                        current_fields = resolve_type_fields(type_name, schema, &current_path, variant_contexts)?;
                    }
                    _ => return None,
                }
            }
            _ => return None, // Other segment types not supported in paths
        }
    }
    
    Some(current_fields)
}

/// Resolve fields for a type reference
fn resolve_type_fields<'a>(
    type_name: &KeyCmpValue,
    schema: &'a DocumentSchema,
    current_path: &[PathSegment],
    variant_contexts: &std::collections::HashMap<String, String>,
) -> Option<&'a IndexMap<KeyCmpValue, FieldSchema>> {
    let type_def = schema.types.get(type_name)?;
    
    match &type_def.type_expr {
        Type::Object(obj) => Some(&obj.fields),
        Type::Variants(variants) => {
            // Need variant context
            let path_key = path_to_string(current_path);
            let variant = variant_contexts.get(&path_key)?;
            let variant_obj = variants.variants.get(&KeyCmpValue::String(variant.clone()))?;
            Some(&variant_obj.fields)
        }
        _ => None,
    }
}

/// Resolve fields for a variant type
fn resolve_variant_fields<'a>(
    variant_type: &'a Type,
    variant_name: &str,
    schema: &'a DocumentSchema,
) -> Option<&'a IndexMap<KeyCmpValue, FieldSchema>> {
    match variant_type {
        Type::Variants(variants) => {
            let variant_obj = variants.variants.get(&KeyCmpValue::String(variant_name.to_string()))?;
            Some(&variant_obj.fields)
        }
        Type::TypeRef(type_name) => {
            let type_def = schema.types.get(type_name)?;
            resolve_variant_fields(&type_def.type_expr, variant_name, schema)
        }
        _ => None,
    }
}

/// Find the variant type at the current path (for $variant completion)
fn find_variant_type_at_path<'a>(path: &[PathSegment], schema: &'a DocumentSchema) -> Option<&'a Type> {
    if path.is_empty() {
        return None;
    }
    
    // Remove the last segment (which should be $variant)
    let parent_path = &path[..path.len() - 1];
    
    // Navigate to the parent and get its type
    let mut current_type: Option<&Type> = None;
    
    // Start from root if no parent path
    if parent_path.is_empty() {
        // Looking for $variant at root level
        let variant_field = schema.root.fields.iter()
            .find(|(key, _)| matches_key(key, "$variant"))
            .map(|(_, field)| field)?;
        return Some(&variant_field.type_expr);
    }
    
    // Navigate through the parent path
    let mut current_obj = &schema.root;
    
    for (i, segment) in parent_path.iter().enumerate() {
        match segment {
            PathSegment::Ident(id) => {
                let field_name = id.as_ref();
                let field = current_obj.fields.iter()
                    .find(|(key, _)| matches_key(key, field_name))
                    .map(|(_, field)| field)?;
                
                match &field.type_expr {
                    Type::Object(obj) => {
                        current_obj = obj;
                    }
                    Type::TypeRef(type_name) => {
                        let type_def = schema.types.get(type_name)?;
                        match &type_def.type_expr {
                            Type::Object(obj) => {
                                current_obj = obj;
                            }
                            Type::Variants(_) => {
                                // This is the variant type we're looking for
                                current_type = Some(&type_def.type_expr);
                            }
                            _ => return None,
                        }
                    }
                    Type::Array(elem_type) => {
                        // Check if next segment is array access
                        if i + 1 < parent_path.len() && matches!(parent_path[i + 1], PathSegment::Array { .. }) {
                            match elem_type.as_ref() {
                                Type::Object(obj) => {
                                    current_obj = obj;
                                }
                                Type::TypeRef(type_name) => {
                                    let type_def = schema.types.get(type_name)?;
                                    match &type_def.type_expr {
                                        Type::Object(obj) => {
                                            current_obj = obj;
                                        }
                                        Type::Variants(_) => {
                                            current_type = Some(&type_def.type_expr);
                                        }
                                        _ => return None,
                                    }
                                }
                                Type::Variants(_) => {
                                    current_type = Some(elem_type);
                                }
                                _ => return None,
                            }
                        } else {
                            return None;
                        }
                    }
                    Type::Variants(_) => {
                        // Found variant type
                        current_type = Some(&field.type_expr);
                    }
                    _ => return None,
                }
            }
            PathSegment::Array { .. } => {
                // Array access already handled above
                continue;
            }
            _ => {} // Skip other segments
        }
    }
    
    // If we found a variant type during traversal, return it
    if let Some(vtype) = current_type {
        return Some(vtype);
    }
    
    // Check if the field we're completing is a variant type
    if let Some(PathSegment::Extension(ext)) = path.last() {
        if ext.as_ref() == "variant" {
            // Look for the parent field that should be a variant
            let parent_field = current_obj.fields.iter()
                .find(|(_key, field)| {
                    matches!(field.type_expr, Type::Variants(_)) ||
                    matches!(&field.type_expr, Type::TypeRef(tn) if schema.types.get(tn).map(|t| matches!(&t.type_expr, Type::Variants(_))).unwrap_or(false))
                })
                .map(|(_, field)| field)?;
                
            match &parent_field.type_expr {
                Type::Variants(_) => Some(&parent_field.type_expr),
                Type::TypeRef(type_name) => {
                    let type_def = schema.types.get(type_name)?;
                    Some(&type_def.type_expr)
                }
                _ => None,
            }
        } else {
            None
        }
    } else {
        None
    }
}

/// Find the expected type for a field at the current path
fn find_field_type_at_path<'a>(path: &[PathSegment], schema: &'a DocumentSchema) -> Option<&'a Type> {
    if path.is_empty() {
        return None;
    }
    
    // The last segment is the field being assigned
    let field_path = &path[..path.len() - 1];
    let field_name = match path.last()? {
        PathSegment::Ident(id) => id.as_ref(),
        PathSegment::Extension(_ext) => return None, // Can't determine type for extensions here
        _ => return None,
    };
    
    // Navigate to the parent
    let fields = if field_path.is_empty() {
        &schema.root.fields
    } else {
        lookup_fields_at_path(field_path, schema, &Default::default())?
    };
    
    // Find the field
    let field = fields.iter()
        .find(|(key, _)| matches_key(key, field_name))
        .map(|(_, field)| field)?;
    
    Some(&field.type_expr)
}

/// Check if a key matches a field name
fn matches_key(key: &KeyCmpValue, field_name: &str) -> bool {
    match key {
        KeyCmpValue::String(s) => s == field_name,
        KeyCmpValue::I64(n) => n.to_string() == field_name,
        KeyCmpValue::U64(n) => n.to_string() == field_name,
        KeyCmpValue::Extension(e) => format!("${e}") == field_name,
        KeyCmpValue::MetaExtension(m) => format!("$${m}") == field_name,
        _ => false,
    }
}

/// Format a key for display
fn format_key(key: &KeyCmpValue) -> String {
    match key {
        KeyCmpValue::String(s) => s.clone(),
        KeyCmpValue::I64(n) => n.to_string(),
        KeyCmpValue::U64(n) => n.to_string(),
        KeyCmpValue::Extension(e) => format!("${e}"),
        KeyCmpValue::MetaExtension(m) => format!("$${m}"),
        KeyCmpValue::Bool(b) => b.to_string(),
        KeyCmpValue::Null => "null".to_string(),
        KeyCmpValue::Unit => "()".to_string(),
        KeyCmpValue::Tuple(_) => "<tuple>".to_string(),
        KeyCmpValue::Hole => "!".to_string(),
    }
}

/// Format type information for display
fn format_type_detail(type_expr: &Type) -> String {
    match type_expr {
        Type::String => "string".to_string(),
        Type::Number => "number".to_string(),
        Type::Boolean => "boolean".to_string(),
        Type::Object(_) => "object".to_string(),
        Type::Array(elem) => format!("array<{}>", format_type_detail(elem)),
        Type::TypeRef(name) => format!("type {}", format_key(name)),
        Type::Variants(_) => "variant".to_string(),
        Type::Code(lang) => {
            if let Some(lang) = lang {
                format!("code.{lang}")
            } else {
                "code".to_string()
            }
        },
        Type::Any => "any".to_string(),
        Type::Null => "null".to_string(),
        Type::Path => "path".to_string(),
        Type::Union(types) => format!("union<{}>", types.len()),
        Type::CascadeType(inner) => format!("cascade<{}>", format_type_detail(inner)),
    }
}

/// Generate value completions based on type
fn generate_value_completions(field_type: Option<&Type>, string_only: bool) -> Vec<CompletionItem> {
    let mut completions = Vec::new();
    
    match field_type {
        Some(Type::Boolean) if !string_only => {
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
        Some(Type::Null) if !string_only => {
            completions.push(CompletionItem {
                label: "null".to_string(),
                kind: Some(CompletionItemKind::VALUE),
                detail: Some("Null value".to_string()),
                ..Default::default()
            });
        }
        _ => {
            // Generic completions
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

/// Generate a snippet for a section with required fields
fn generate_section_snippet(field_name: &str, object_schema: &ObjectSchema) -> Option<String> {
    let required_fields: Vec<(&KeyCmpValue, &FieldSchema)> = object_schema
        .fields
        .iter()
        .filter(|(_, field)| !field.optional)
        .collect();
    
    if required_fields.is_empty() {
        return None;
    }
    
    let mut snippet = String::new();
    let mut tab_stop = 1;
    
    snippet.push_str(field_name);
    snippet.push('\n');
    
    for (key, _) in required_fields.iter() {
        let key_str = format_key(key);
        snippet.push_str(&format!("{key_str} = ${{{tab_stop}:!}}\n"));
        tab_stop += 1;
    }
    
    snippet.push_str("$0");
    
    Some(snippet)
}

/// Convert path segments to a string key
fn path_to_string(path: &[PathSegment]) -> String {
    path.iter()
        .map(|seg| match seg {
            PathSegment::Ident(id) => id.as_ref().to_string(),
            PathSegment::Extension(ext) => format!("${}", ext.as_ref()),
            PathSegment::MetaExt(meta) => format!("$${}", meta.as_ref()),
            PathSegment::Array { index, .. } => {
                if let Some(eure_value::value::Value::I64(idx)) = index {
                    format!("[{idx}]")
                } else {
                    "[]".to_string()
                }
            },
            PathSegment::Value(v) => format!("{v:?}"),
            PathSegment::TupleIndex(idx) => format!("[{idx}]"),
        })
        .collect::<Vec<_>>()
        .join(".")
}