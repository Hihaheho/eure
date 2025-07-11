use crate::cst_path_extractor::CstPathExtractor;
use crate::schema_validation::SchemaManager;
use eure_parol::ParseResult;
use eure_schema::{DocumentSchema, ObjectSchema};
use eure_value::identifier::Identifier;
use eure_value::value::PathSegment;
use lsp_types::{CompletionItem, CompletionItemKind, Position};
use std::str::FromStr;

#[derive(Debug, PartialEq)]
enum CompletionContextType {
    AfterDot,
    AfterEquals,
    AfterAt,
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
        eprintln!("DEBUG: Context type determined: {:?}", context);
        eprintln!("DEBUG: Byte offset: {}, Input length: {}", byte_offset, self.input.len());
        eprintln!("DEBUG: Input at cursor: {:?}", &self.input[..byte_offset]);
        
        // Check if we have a parse error at cursor position
        if let Some(error) = self.parse_result.error() {
            // Get the error string to analyze
            let _error_str = error.to_string();
            eprintln!("DEBUG: Parse error detected: {}", _error_str);
            let at_error_pos = self.is_at_error_position(byte_offset);
            eprintln!("DEBUG: Is at error position: {}", at_error_pos);
            
            if at_error_pos {
                match context {
                    CompletionContextType::AfterDot => {
                        eprintln!("DEBUG: Handling completion after dot");
                        // Extract the path up to this point
                        let mut path_extractor =
                            CstPathExtractor::new(self.input.clone(), byte_offset as u32);
                        let path = path_extractor.extract_path(get_cst_ref(&self.parse_result));
                        eprintln!("DEBUG: Extracted path: {:?}", path);

                        // Get schema completions for this path
                        if let Some(schema_uri) =
                            self.schema_manager.get_document_schema_uri(self.uri)
                            && let Some(schema) = self.schema_manager.get_schema(schema_uri)
                        {
                            eprintln!("DEBUG: Found schema, getting field completions");
                            return self.get_field_completions_for_path(&path, schema);
                        } else {
                            eprintln!("DEBUG: No schema found for URI: {}", self.uri);
                        }

                        return vec![];
                    }
                    CompletionContextType::AfterEquals => {
                        eprintln!("DEBUG: Handling completion after equals");
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
                    CompletionContextType::AfterAt => {
                        eprintln!("DEBUG: Handling completion after @");
                        // Get root-level field completions
                        if let Some(schema_uri) = self.schema_manager.get_document_schema_uri(self.uri)
                            && let Some(schema) = self.schema_manager.get_schema(schema_uri)
                        {
                            eprintln!("DEBUG: Found schema, getting root field completions");
                            return self.get_field_completions_for_path(&[], schema);
                        } else {
                            eprintln!("DEBUG: No schema found for URI: {}", self.uri);
                        }
                        return vec![];
                    }
                    CompletionContextType::Unknown => {
                        eprintln!("DEBUG: Unknown context type in error handling");
                    }
                }
            }
        } else {
            eprintln!("DEBUG: No parse error detected");
        }

        // If no error, check if we're in a partial identifier context
        if self.parse_result.error().is_none() && context == CompletionContextType::Unknown {
            // Check if we're typing a partial key after @
            if let Some(partial) = self.get_partial_identifier_at_cursor(byte_offset) {
                // Get root-level field completions
                if let Some(schema_uri) = self.schema_manager.get_document_schema_uri(self.uri)
                    && let Some(schema) = self.schema_manager.get_schema(schema_uri)
                {
                    return self
                        .get_field_completions_for_path(&[], schema)
                        .into_iter()
                        .filter(|c| c.label.starts_with(&partial))
                        .collect();
                }
            }
        }

        vec![]
    }

    fn get_partial_identifier_at_cursor(&self, byte_offset: usize) -> Option<String> {
        let input_bytes = self.input.as_bytes();

        // Find start of identifier by going backwards
        let mut start = byte_offset;
        while start > 0
            && (input_bytes[start - 1].is_ascii_alphanumeric() || input_bytes[start - 1] == b'_')
        {
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
            eprintln!("DEBUG analyze_context: byte_offset is 0, returning Unknown");
            return CompletionContextType::Unknown;
        }

        // Look at the character just before the cursor
        let input_bytes = self.input.as_bytes();

        // Skip whitespace backwards
        let mut pos = byte_offset.saturating_sub(1);
        eprintln!("DEBUG analyze_context: Starting pos: {}", pos);
        while pos > 0 && input_bytes[pos].is_ascii_whitespace() {
            eprintln!("DEBUG analyze_context: Skipping whitespace at pos {}", pos);
            pos = pos.saturating_sub(1);
        }

        // Check what non-whitespace character we found
        eprintln!("DEBUG analyze_context: Final pos: {}, char: {:?} ({})", 
                 pos, input_bytes[pos] as char, input_bytes[pos]);
        
        if input_bytes[pos] == b'.' {
            eprintln!("DEBUG analyze_context: Found dot, returning AfterDot");
            CompletionContextType::AfterDot
        } else if input_bytes[pos] == b'=' {
            eprintln!("DEBUG analyze_context: Found equals, returning AfterEquals");
            CompletionContextType::AfterEquals
        } else if input_bytes[pos] == b'@' {
            eprintln!("DEBUG analyze_context: Found at symbol, returning AfterAt");
            CompletionContextType::AfterAt
        } else {
            eprintln!("DEBUG analyze_context: Found other char, returning Unknown");
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
        let result = byte_offset == self.input.len();
        eprintln!("DEBUG is_at_error_position: byte_offset={}, input.len()={}, result={}", 
                 byte_offset, self.input.len(), result);
        result
    }

    fn get_field_completions_for_path(
        &self,
        path: &[String],
        schema: &DocumentSchema,
    ) -> Vec<CompletionItem> {
        eprintln!("DEBUG get_field_completions: path = {:?}", path);
        
        // Convert string path to PathSegments
        let path_segments: Vec<PathSegment> = path
            .iter()
            .filter_map(|s| Identifier::from_str(s).ok().map(PathSegment::Ident))
            .collect();
        eprintln!("DEBUG get_field_completions: path_segments = {:?}", path_segments);

        // Look up the schema at this path
        let object_schema = if path_segments.is_empty() {
            eprintln!("DEBUG get_field_completions: Using root schema");
            &schema.root
        } else {
            match self.lookup_schema_at_path(&path_segments, &schema.root) {
                Some(obj) => {
                    eprintln!("DEBUG get_field_completions: Found schema at path");
                    obj
                },
                None => {
                    eprintln!("DEBUG get_field_completions: No schema found at path");
                    return vec![];
                },
            }
        };

        // Generate completions from the fields
        let mut completions = vec![];
        eprintln!("DEBUG get_field_completions: Number of fields: {}", object_schema.fields.len());
        for (field_name, field_schema) in &object_schema.fields {
            let label = match field_name {
                eure_value::value::KeyCmpValue::String(s) => s.clone(),
                eure_value::value::KeyCmpValue::MetaExtension(s) => format!("${s}"),
                _ => continue, // Skip non-string keys for completion
            };
            eprintln!("DEBUG get_field_completions: Adding field: {}", label);
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

        eprintln!("DEBUG get_field_completions: Returning {} completions", completions.len());
        completions
    }

    fn lookup_schema_at_path<'b>(
        &self,
        path: &[PathSegment],
        schema: &'b ObjectSchema,
    ) -> Option<&'b ObjectSchema> {
        eprintln!("DEBUG lookup_schema_at_path: path = {:?}", path);
        eprintln!("DEBUG lookup_schema_at_path: schema has {} fields", schema.fields.len());
        for (k, _) in &schema.fields {
            eprintln!("  - Field: {:?}", k);
        }
        
        if path.is_empty() {
            return Some(schema);
        }

        let segment = &path[0];
        let remaining = &path[1..];

        match segment {
            PathSegment::Ident(field_name) => {
                let key = eure_value::value::KeyCmpValue::String(field_name.to_string());
                eprintln!("DEBUG lookup_schema_at_path: Looking for field {:?}", key);
                if let Some(field_schema) = schema.fields.get(&key) {
                    eprintln!("DEBUG lookup_schema_at_path: Found field, type = {:?}", field_schema.type_expr);
                    match &field_schema.type_expr {
                        eure_schema::Type::Object(obj) => {
                            self.lookup_schema_at_path(remaining, obj)
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
