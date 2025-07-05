use lsp_types::{CompletionItem, CompletionItemKind, Position};
use eure_parol::ParseResult;
use crate::cst_path_extractor::CstPathExtractor;
use crate::schema_validation::SchemaManager;
use eure_schema::{DocumentSchema, ObjectSchema};
use eure_value::value::PathSegment;
use eure_value::identifier::Identifier;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
enum CompletionContextType {
    AfterDot,
    AfterEquals,
    Unknown,
}

pub struct CompletionAnalyzer<'a> {
    input: String,
    parse_result: ParseResult,
    cursor_position: Position,
    schema_manager: &'a SchemaManager,
    uri: &'a str,
}

fn get_cst_ref(parse_result: &ParseResult) -> &eure_tree::Cst {
    match parse_result {
        ParseResult::Ok(cst) => cst,
        ParseResult::ErrWithCst { cst, .. } => cst,
    }
}

impl<'a> CompletionAnalyzer<'a> {
    pub fn new(
        input: String, 
        parse_result: ParseResult, 
        cursor_position: Position,
        schema_manager: &'a SchemaManager,
        uri: &'a str,
    ) -> Self {
        Self {
            input,
            parse_result,
            cursor_position,
            schema_manager,
            uri,
        }
    }
    
    pub fn analyze(&self) -> Vec<CompletionItem> {
        // Convert position to byte offset
        let byte_offset = self.position_to_byte_offset();
        
        // Check what's immediately before the cursor
        let context = self.analyze_context_before_cursor(byte_offset);
        // Check if we have a parse error at cursor position
        if let Some(error) = self.parse_result.error() {
            // Get the error string to analyze
            let _error_str = error.to_string();
            if self.is_at_error_position(byte_offset) {
                match context {
                    CompletionContextType::AfterDot => {
                        // Extract the path up to this point
                        let mut path_extractor = CstPathExtractor::new(
                            self.input.clone(),
                            byte_offset as u32,
                        );
                        let path = path_extractor.extract_path(get_cst_ref(&self.parse_result));
                        
                        // Get schema completions for this path
                        if let Some(schema_uri) = self.schema_manager.get_document_schema_uri(self.uri) {
                            if let Some(schema) = self.schema_manager.get_schema(schema_uri) {
                                return self.get_field_completions_for_path(&path, schema);
                            }
                        }
                        
                        return vec![];
                    }
                    CompletionContextType::AfterEquals => {
return vec![
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
                        ];
                    }
                    CompletionContextType::Unknown => {}
                }
            }
        }
        
        // If no error, check if we're in a partial identifier context
        if self.parse_result.error().is_none() && context == CompletionContextType::Unknown {
            // Check if we're typing a partial key after @ 
            if let Some(partial) = self.get_partial_identifier_at_cursor(byte_offset) {
                // Get root-level field completions
                if let Some(schema_uri) = self.schema_manager.get_document_schema_uri(self.uri) {
                    if let Some(schema) = self.schema_manager.get_schema(schema_uri) {
                        return self.get_field_completions_for_path(&vec![], schema)
                            .into_iter()
                            .filter(|c| c.label.starts_with(&partial))
                            .collect();
                    }
                }
            }
        }
        
vec![]
    }
    
    fn get_partial_identifier_at_cursor(&self, byte_offset: usize) -> Option<String> {
        let input_bytes = self.input.as_bytes();
        
        // Find start of identifier by going backwards
        let mut start = byte_offset;
        while start > 0 && (input_bytes[start - 1].is_ascii_alphanumeric() || input_bytes[start - 1] == b'_') {
            start -= 1;
        }
        
        if start < byte_offset {
            Some(self.input[start..byte_offset].to_string())
        } else {
            None
        }
    }
    
    fn analyze_context_before_cursor(&self, byte_offset: usize) -> CompletionContextType {
        if byte_offset == 0 {
            return CompletionContextType::Unknown;
        }
        
        // Look at the character just before the cursor
        let input_bytes = self.input.as_bytes();
        
        // Skip whitespace backwards
        let mut pos = byte_offset.saturating_sub(1);
        while pos > 0 && input_bytes[pos].is_ascii_whitespace() {
            pos = pos.saturating_sub(1);
        }
        
        // Check what non-whitespace character we found
        if input_bytes[pos] == b'.' {
            CompletionContextType::AfterDot
        } else if input_bytes[pos] == b'=' {
            CompletionContextType::AfterEquals
        } else {
            CompletionContextType::Unknown
        }
    }
    
    fn position_to_byte_offset(&self) -> usize {
        let mut offset = 0;
        let lines: Vec<&str> = self.input.lines().collect();
        
        for (i, line) in lines.iter().enumerate() {
            if i < self.cursor_position.line as usize {
                offset += line.len() + 1; // +1 for newline
            } else if i == self.cursor_position.line as usize {
                offset += self.cursor_position.character.min(line.len() as u32) as usize;
                break;
            }
        }
        
        offset
    }
    
    fn is_at_error_position(&self, byte_offset: usize) -> bool {
        // For now, just check if we're at the end of input
        // In a real implementation, we'd check the actual error location
        byte_offset == self.input.len()
    }
    
    fn get_field_completions_for_path(&self, path: &[String], schema: &DocumentSchema) -> Vec<CompletionItem> {
        // Convert string path to PathSegments
        let path_segments: Vec<PathSegment> = path.iter()
            .filter_map(|s| Identifier::from_str(s).ok().map(PathSegment::Ident))
            .collect();
        
        // Look up the schema at this path
        let object_schema = if path_segments.is_empty() {
            &schema.root
        } else {
            match self.lookup_schema_at_path(&path_segments, &schema.root) {
                Some(obj) => obj,
                None => return vec![],
            }
        };
        
        // Generate completions from the fields
        let mut completions = vec![];
        for (field_name, field_schema) in &object_schema.fields {
            let label = match field_name {
                eure_value::value::KeyCmpValue::String(s) => s.clone(),
                eure_value::value::KeyCmpValue::Extension(s) => format!("${}", s),
                eure_value::value::KeyCmpValue::MetaExtension(s) => format!("$${}", s),
                _ => continue, // Skip non-string keys for completion
            };
            completions.push(CompletionItem {
                label,
                kind: Some(CompletionItemKind::FIELD),
                documentation: field_schema.description.as_ref().map(|desc| {
                    lsp_types::Documentation::MarkupContent(lsp_types::MarkupContent {
                        kind: lsp_types::MarkupKind::Markdown,
                        value: desc.clone(),
                    })
                }),
                ..Default::default()
            });
        }
        
        completions
    }
    
    fn lookup_schema_at_path<'b>(&self, path: &[PathSegment], schema: &'b ObjectSchema) -> Option<&'b ObjectSchema> {
        if path.is_empty() {
            return Some(schema);
        }
        
        let segment = &path[0];
        let remaining = &path[1..];
        
        match segment {
            PathSegment::Ident(field_name) => {
                let key = eure_value::value::KeyCmpValue::String(field_name.to_string());
                if let Some(field_schema) = schema.fields.get(&key) {
                    match &field_schema.type_expr {
                        eure_schema::Type::Object(obj) => {
                            self.lookup_schema_at_path(remaining, &obj)
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }
            _ => None, // Handle other path segment types if needed
        }
    }
}