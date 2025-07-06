use ahash::AHashMap;
use eure_value::{
    document::{EureDocument, InsertError},
    identifier::Identifier,
    value::{Array, Code, KeyCmpValue, Map, Path, PathSegment, Tuple, Value},
};
use std::str::FromStr;
use thiserror::Error;

use crate::{
    prelude::*,
    tree::CstFacade,
};

#[derive(Debug, Error)]
pub enum ValueVisitorError {
    #[error(transparent)]
    CstError(#[from] CstConstructError),
    #[error("Invalid identifier: {0}")]
    InvalidIdentifier(String),
    #[error("Failed to parse integer: {0}")]
    InvalidInteger(String),
    #[error("Document insert error: {0}")]
    DocumentInsert(#[from] InsertError<Value>),
}

pub struct ValueVisitor<'a> {
    input: &'a str,
    // Main document being built
    document: EureDocument<Value>,
    // Stack for nested sections
    section_stack: Vec<EureDocument<Value>>,
    // Temporary value cache for references
    value_cache: AHashMap<ValueHandle, Value>,
}

impl<'a> ValueVisitor<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            document: EureDocument::new(),
            section_stack: Vec::new(),
            value_cache: AHashMap::new(),
        }
    }

    pub fn into_document(self) -> EureDocument<Value> {
        self.document
    }

    /// Get the current document to insert into (either main or section)
    fn current_document(&mut self) -> &mut EureDocument<Value> {
        self.section_stack.last_mut().unwrap_or(&mut self.document)
    }

    /// Parse a string literal, removing quotes and handling escape sequences
    fn parse_string_literal(&self, text: &str) -> String {
        // Remove surrounding quotes
        let without_quotes = &text[1..text.len() - 1];

        // Handle escape sequences
        let mut result = String::new();
        let mut chars = without_quotes.chars();

        while let Some(ch) = chars.next() {
            if ch == '\\' {
                if let Some(escaped) = chars.next() {
                    match escaped {
                        'n' => result.push('\n'),
                        't' => result.push('\t'),
                        'r' => result.push('\r'),
                        '\\' => result.push('\\'),
                        '"' => result.push('"'),
                        '\'' => result.push('\''),
                        _ => {
                            // Unknown escape sequence, keep as is
                            result.push('\\');
                            result.push(escaped);
                        }
                    }
                } else {
                    // Trailing backslash
                    result.push('\\');
                }
            } else {
                result.push(ch);
            }
        }

        result
    }

    /// Build path segments from a key view (may return multiple segments for array syntax)
    fn build_path_segments<F: CstFacade>(
        &mut self,
        key_view: KeyView,
        tree: &F,
    ) -> Result<Vec<PathSegment>, ValueVisitorError> {
        let base_segment = match key_view.key_base.get_view(tree)? {
            KeyBaseView::Ident(ident_handle) => {
                let ident_view = ident_handle.get_view(tree)?;
                let data = ident_view.ident.get_data(tree)?;
                let text = tree.get_str(data, self.input).unwrap();
                let identifier = Identifier::from_str(text)
                    .map_err(|_| ValueVisitorError::InvalidIdentifier(text.to_string()))?;
                PathSegment::Ident(identifier)
            }
            KeyBaseView::Str(str_handle) => {
                let str_view = str_handle.get_view(tree)?;
                let data = str_view.str.get_data(tree)?;
                let text = tree.get_str(data, self.input).unwrap();
                let string_value = self.parse_string_literal(text);
                PathSegment::Value(KeyCmpValue::String(string_value))
            }
            KeyBaseView::Integer(integer_handle) => {
                let integer_view = integer_handle.get_view(tree)?;
                let data = integer_view.integer.get_data(tree)?;
                let text = tree.get_str(data, self.input).unwrap();

                let value = if let Ok(i) = text.parse::<i64>() {
                    KeyCmpValue::I64(i)
                } else if let Ok(u) = text.parse::<u64>() {
                    KeyCmpValue::U64(u)
                } else {
                    return Err(ValueVisitorError::InvalidInteger(text.to_string()));
                };
                PathSegment::Value(value)
            }
            KeyBaseView::ExtensionNameSpace(ext_ns_handle) => {
                let ext_ns_view = ext_ns_handle.get_view(tree)?;
                let ident_view = ext_ns_view.ident.get_view(tree)?;
                let data = ident_view.ident.get_data(tree)?;
                let text = tree.get_str(data, self.input).unwrap();
                let identifier = Identifier::from_str(text)
                    .map_err(|_| ValueVisitorError::InvalidIdentifier(text.to_string()))?;
                PathSegment::Extension(identifier)
            }
            KeyBaseView::MetaExtKey(meta_ext_key_handle) => {
                let meta_ext_key_view = meta_ext_key_handle.get_view(tree)?;
                let ident_view = meta_ext_key_view.ident.get_view(tree)?;
                let data = ident_view.ident.get_data(tree)?;
                let text = tree.get_str(data, self.input).unwrap();
                let identifier = Identifier::from_str(text)
                    .map_err(|_| ValueVisitorError::InvalidIdentifier(text.to_string()))?;
                PathSegment::MetaExt(identifier)
            }
            KeyBaseView::Null(_) => PathSegment::Ident(Identifier::from_str("null").unwrap()),
            KeyBaseView::True(_) => PathSegment::Ident(Identifier::from_str("true").unwrap()),
            KeyBaseView::False(_) => PathSegment::Ident(Identifier::from_str("false").unwrap()),
        };

        // Handle array indexing if present
        if let Some(array_marker) = key_view.key_opt.get_view(tree)? {
            let array_marker_view = array_marker.get_view(tree)?;

            // Check if there's an index specified
            if let Some(integer_handle) = array_marker_view.array_marker_opt.get_view(tree)? {
                let integer_view = integer_handle.get_view(tree)?;
                let data = integer_view.integer.get_data(tree)?;
                let text = tree.get_str(data, self.input).unwrap();

                let index = if let Ok(i) = text.parse::<u8>() {
                    i
                } else {
                    return Err(ValueVisitorError::InvalidInteger(text.to_string()));
                };
                
                // Return both the base segment and the array index
                Ok(vec![base_segment, PathSegment::ArrayIndex(index)])
            } else {
                // Empty brackets [] means append/next index
                // For now, use 0 as placeholder
                Ok(vec![base_segment, PathSegment::ArrayIndex(0)])
            }
        } else {
            Ok(vec![base_segment])
        }
    }

    /// Build a value from a value view
    fn build_value<F: CstFacade>(
        &mut self,
        value_view: ValueView,
        tree: &F,
    ) -> Result<Value, ValueVisitorError> {
        match value_view {
            ValueView::Null(_) => Ok(Value::Null),
            ValueView::Boolean(boolean_handle) => {
                let boolean_view = boolean_handle.get_view(tree)?;
                match boolean_view {
                    BooleanView::True(_) => Ok(Value::Bool(true)),
                    BooleanView::False(_) => Ok(Value::Bool(false)),
                }
            }
            ValueView::Integer(integer_handle) => {
                let integer_view = integer_handle.get_view(tree)?;
                let data = integer_view.integer.get_data(tree)?;
                let text = tree.get_str(data, self.input).unwrap();

                if let Ok(i) = text.parse::<i64>() {
                    Ok(Value::I64(i))
                } else if let Ok(u) = text.parse::<u64>() {
                    Ok(Value::U64(u))
                } else {
                    Err(ValueVisitorError::InvalidInteger(text.to_string()))
                }
            }
            ValueView::Strings(strings_handle) => {
                let strings_view = strings_handle.get_view(tree)?;

                let first_str_view = strings_view.str.get_view(tree)?;
                let first_str_data = first_str_view.str.get_data(tree)?;
                let first_str_text = tree.get_str(first_str_data, self.input).unwrap();

                let mut result = self.parse_string_literal(first_str_text);

                // Collect additional string parts
                if let Some(mut strings_list) = strings_view.strings_list.get_view(tree)? {
                    loop {
                        let str_view = strings_list.str.get_view(tree)?;
                        let str_data = str_view.str.get_data(tree)?;
                        let str_text = tree.get_str(str_data, self.input).unwrap();
                        result.push_str(&self.parse_string_literal(str_text));

                        match strings_list.strings_list.get_view(tree)? {
                            Some(next) => strings_list = next,
                            None => break,
                        }
                    }
                }

                Ok(Value::String(result))
            }
            ValueView::Object(object_handle) => {
                let object_view = object_handle.get_view(tree)?;
                let mut map = AHashMap::new();

                if let Some(mut object_list) = object_view.object_list.get_view(tree)? {
                    loop {
                        // Get the key
                        let key_view = object_list.key.get_view(tree)?;
                        let key_segments = self.build_path_segments(key_view, tree)?;
                        
                        // For object literals, we only support single segment keys
                        if key_segments.len() != 1 {
                            continue; // Skip array syntax in object literals
                        }
                        
                        // Convert to KeyCmpValue
                        let key_cmp = match &key_segments[0] {
                            PathSegment::Ident(ident) => KeyCmpValue::String(ident.to_string()),
                            PathSegment::Value(val) => val.clone(),
                            _ => continue, // Skip extension keys in object literals
                        };

                        // Get the value
                        let value_handle = object_list.value;
                        let value_view = value_handle.get_view(tree)?;
                        let value = self.build_value(value_view, tree)?;
                        
                        map.insert(key_cmp, value);

                        match object_list.object_list.get_view(tree)? {
                            Some(next) => object_list = next,
                            None => break,
                        }
                    }
                }

                Ok(Value::Map(Map(map)))
            }
            ValueView::Array(array_handle) => {
                let array_view = array_handle.get_view(tree)?;
                let mut elements = Vec::new();

                if let Some(array_elements) = array_view.array_opt.get_view(tree)? {
                    let array_elements_view = array_elements.get_view(tree)?;
                    
                    // First element
                    let value_view = array_elements_view.value.get_view(tree)?;
                    elements.push(self.build_value(value_view, tree)?);

                    // Rest of the elements
                    if let Some(mut tail) = array_elements_view.array_elements_opt.get_view(tree)? {
                        loop {
                            let tail_view = tail.get_view(tree)?;
                            
                            if let Some(next_elements) = tail_view.array_elements_tail_opt.get_view(tree)? {
                                let next_view = next_elements.get_view(tree)?;
                                let value_view = next_view.value.get_view(tree)?;
                                elements.push(self.build_value(value_view, tree)?);
                                
                                match next_view.array_elements_opt.get_view(tree)? {
                                    Some(next_tail) => tail = next_tail,
                                    None => break,
                                }
                            } else {
                                break;
                            }
                        }
                    }
                }

                Ok(Value::Array(Array(elements)))
            }
            ValueView::Code(code_handle) => {
                let code_view = code_handle.get_view(tree)?;
                let data = code_view.code.get_data(tree)?;
                let text = tree.get_str(data, self.input).unwrap();

                let content = text[1..text.len() - 1].to_string();
                Ok(Value::CodeBlock(Code {
                    language: String::new(),
                    content,
                }))
            }
            ValueView::CodeBlock(code_block_handle) => {
                let code_block_view = code_block_handle.get_view(tree)?;
                let data = code_block_view.code_block.get_data(tree)?;
                let text = tree.get_str(data, self.input).unwrap();

                // Skip the opening ```
                let after_opening = &text[3..];

                // Find the matching closing ```
                let closing_pos = after_opening.find("\n```").unwrap_or(after_opening.len());
                let code_content = &after_opening[..closing_pos];

                // Find the first newline to separate language from content
                let newline_pos = code_content.find('\n').unwrap_or(code_content.len());

                let language = code_content[..newline_pos].trim().to_string();
                let content = if newline_pos < code_content.len() {
                    code_content[newline_pos + 1..].to_string()
                } else {
                    String::new()
                };

                Ok(Value::CodeBlock(Code { language, content }))
            }
            ValueView::NamedCode(named_code_handle) => {
                let named_code_view = named_code_handle.get_view(tree)?;
                let data = named_code_view.named_code.get_data(tree)?;
                let text = tree.get_str(data, self.input).unwrap();

                let backtick_pos = text.find('`').unwrap();
                let language = text[..backtick_pos].to_string();
                let content = text[backtick_pos + 1..text.len() - 1].to_string();

                Ok(Value::Code(Code { language, content }))
            }
            ValueView::Hole(_) => Ok(Value::Hole),
            ValueView::Path(path_handle) => {
                let path_view = path_handle.get_view(tree)?;
                let mut segments = Vec::new();

                if let Ok(keys_view) = path_view.keys.get_view(tree) {
                    // First key
                    let key_view = keys_view.key.get_view(tree)?;
                    segments.extend(self.build_path_segments(key_view, tree)?);

                    // Additional keys
                    if let Ok(Some(mut keys_list)) = keys_view.keys_list.get_view(tree) {
                        loop {
                            let key_view = keys_list.key.get_view(tree)?;
                            segments.extend(self.build_path_segments(key_view, tree)?);

                            match keys_list.keys_list.get_view(tree) {
                                Ok(Some(next)) => keys_list = next,
                                _ => break,
                            }
                        }
                    }
                }

                Ok(Value::Path(Path(segments)))
            }
            ValueView::Tuple(tuple_handle) => {
                let tuple_view = tuple_handle.get_view(tree)?;
                let mut elements = Vec::new();

                if let Ok(Some(tuple_elements)) = tuple_view.tuple_opt.get_view(tree) {
                    let tuple_elements_view = tuple_elements.get_view(tree)?;
                    
                    // First element
                    let value_view = tuple_elements_view.value.get_view(tree)?;
                    elements.push(self.build_value(value_view, tree)?);

                    // Rest of the elements
                    if let Ok(Some(mut tail)) = tuple_elements_view.tuple_elements_opt.get_view(tree) {
                        loop {
                            let tail_view = tail.get_view(tree)?;
                            
                            if let Some(next_elements) = tail_view.tuple_elements_tail_opt.get_view(tree)? {
                                let next_view = next_elements.get_view(tree)?;
                                let value_view = next_view.value.get_view(tree)?;
                                elements.push(self.build_value(value_view, tree)?);
                                
                                match next_view.tuple_elements_opt.get_view(tree)? {
                                    Some(next_tail) => tail = next_tail,
                                    None => break,
                                }
                            } else {
                                break;
                            }
                        }
                    }
                }

                Ok(Value::Tuple(Tuple(elements)))
            }
        }
    }
}

impl<F: CstFacade> CstVisitor<F> for ValueVisitor<'_> {
    type Error = ValueVisitorError;

    fn visit_eure(
        &mut self,
        handle: EureHandle,
        view: EureView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Visit the eure structure
        self.visit_eure_super(handle, view, tree)?;
        Ok(())
    }

    fn visit_value(
        &mut self,
        handle: ValueHandle,
        view: ValueView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Build and cache the value
        let value = self.build_value(view, tree)?;
        self.value_cache.insert(handle, value);
        Ok(())
    }

    fn visit_binding(
        &mut self,
        _handle: BindingHandle,
        view: BindingView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Build path from keys
        let mut path = Vec::new();
        
        if let Ok(keys_view) = view.keys.get_view(tree) {
            // First key
            let key_view = keys_view.key.get_view(tree)?;
            path.extend(self.build_path_segments(key_view, tree)?);

            // Additional keys
            if let Ok(Some(mut keys_list)) = keys_view.keys_list.get_view(tree) {
                loop {
                    let key_view = keys_list.key.get_view(tree)?;
                    path.extend(self.build_path_segments(key_view, tree)?);

                    match keys_list.keys_list.get_view(tree) {
                        Ok(Some(next)) => keys_list = next,
                        _ => break,
                    }
                }
            }
        }

        // Get the value to bind
        let value = match view.binding_rhs.get_view(tree) {
            Ok(BindingRhsView::ValueBinding(value_binding_handle)) => {
                let value_binding_view = value_binding_handle.get_view(tree)?;
                let value_view = value_binding_view.value.get_view(tree)?;
                self.build_value(value_view, tree)?
            }
            Ok(BindingRhsView::TextBinding(text_binding_handle)) => {
                let text_binding_view = text_binding_handle.get_view(tree)?;
                if let Ok(text_view) = text_binding_view.text.get_view(tree) {
                    if let Ok(data) = text_view.text.get_data(tree) {
                        let text = tree.get_str(data, self.input).unwrap_or("").trim();
                        Value::String(text.to_string())
                    } else {
                        Value::String(String::new())
                    }
                } else {
                    Value::String(String::new())
                }
            }
            Ok(BindingRhsView::SectionBinding(section_binding_handle)) => {
                // Create new document for the section
                let section_document = EureDocument::new();
                self.section_stack.push(section_document);

                // Process section binding
                if let Ok(section_binding_view) = section_binding_handle.get_view(tree)
                    && let Ok(eure_view) = section_binding_view.eure.get_view(tree)
                {
                    // Visit the eure within the section
                    self.visit_eure(section_binding_view.eure, eure_view, tree)?;
                }

                // Pop the section document and convert to value
                if let Some(section_doc) = self.section_stack.pop() {
                    document_to_value(section_doc)
                } else {
                    Value::Null
                }
            }
            _ => Value::Null,
        };

        // Insert the value into the document at the given path
        self.current_document().insert_node(path.into_iter(), value)?;
        
        Ok(())
    }

    fn visit_section(
        &mut self,
        _handle: SectionHandle,
        view: SectionView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Build path from keys
        let mut path = Vec::new();
        
        if let Ok(keys_view) = view.keys.get_view(tree) {
            // First key
            let key_view = keys_view.key.get_view(tree)?;
            path.extend(self.build_path_segments(key_view, tree)?);

            // Additional keys
            if let Ok(Some(mut keys_list)) = keys_view.keys_list.get_view(tree) {
                loop {
                    let key_view = keys_list.key.get_view(tree)?;
                    path.extend(self.build_path_segments(key_view, tree)?);

                    match keys_list.keys_list.get_view(tree) {
                        Ok(Some(next)) => keys_list = next,
                        _ => break,
                    }
                }
            }
        }

        // Create new document for this section
        let mut section_document = EureDocument::new();

        // Process section body
        if let Ok(section_body) = view.section_body.get_view(tree) {
            match section_body {
                SectionBodyView::SectionBinding(binding_handle) => {
                    if let Ok(binding_view) = binding_handle.get_view(tree)
                        && let Ok(eure_view) = binding_view.eure.get_view(tree)
                    {
                        self.section_stack.push(section_document);

                        // Visit the eure content
                        if let Ok(Some(bindings)) = eure_view.eure_bindings.get_view(tree) {
                            self.visit_eure_bindings(eure_view.eure_bindings, bindings, tree)?;
                        }
                        if let Ok(Some(sections)) = eure_view.eure_sections.get_view(tree) {
                            self.visit_eure_sections(eure_view.eure_sections, sections, tree)?;
                        }

                        section_document = self.section_stack.pop().unwrap();
                    }
                }
                SectionBodyView::SectionBodyList(body_list_handle) => {
                    self.section_stack.push(section_document);

                    if let Ok(Some(body_list)) = body_list_handle.get_view(tree) {
                        self.visit_section_body_list(body_list_handle, body_list, tree)?;
                    }

                    section_document = self.section_stack.pop().unwrap();
                }
                SectionBodyView::Bind(_) => {
                    // Direct assignment not fully supported
                }
            }
        }

        // Convert section document to value and insert into parent document
        let section_value = document_to_value(section_document);
        self.current_document().insert_node(path.into_iter(), section_value)?;

        Ok(())
    }
}


// Convert EureDocument to Value, handling variants
fn document_to_value(doc: EureDocument<Value>) -> Value {
    let root = doc.get_root();
    let value = node_to_value(&doc, root);
    
    // Check if this is a variant (has $variant field)
    if let Value::Map(Map(mut map)) = value {
        if let Some(Value::String(variant_name)) = map.get(&KeyCmpValue::String("$variant".to_string())) {
            let variant_name = variant_name.clone();
            
            // Remove $variant and $variant.repr fields
            map.remove(&KeyCmpValue::String("$variant".to_string()));
            map.remove(&KeyCmpValue::String("$variant.repr".to_string()));
            
            // Create variant structure
            let mut variant_map = AHashMap::new();
            variant_map.insert(KeyCmpValue::String(variant_name), Value::Map(Map(map)));
            Value::Map(Map(variant_map))
        } else {
            Value::Map(Map(map))
        }
    } else {
        value
    }
}

fn node_to_value(doc: &EureDocument<Value>, node: &eure_value::document::Node<Value>) -> Value {
    use eure_value::document::{DocumentKey, NodeContent};
    
    match &node.content {
        NodeContent::Value(v) => {
            // If this node has extensions, create a map with extensions
            if !node.extensions.is_empty() {
                let mut map = AHashMap::new();
                
                // Store the actual value under _value key
                map.insert(KeyCmpValue::String("_value".to_string()), v.clone());
                
                // Add extensions
                for (ext_name, ext_node_id) in &node.extensions {
                    let ext_node = doc.get_node(*ext_node_id);
                    let ext_value = node_to_value(doc, ext_node);
                    map.insert(
                        KeyCmpValue::String(format!("${}", ext_name)),
                        ext_value,
                    );
                }
                
                Value::Map(Map(map))
            } else {
                v.clone()
            }
        }
        NodeContent::Map(entries) => {
            let mut map = AHashMap::new();
            
            // Check if this is a tuple (all keys are TupleIndex)
            let is_tuple = entries.iter().all(|(k, _)| matches!(k, DocumentKey::TupleIndex(_)));
            
            if is_tuple {
                // Convert to tuple
                let mut tuple_elements: Vec<(u8, Value)> = entries
                    .iter()
                    .filter_map(|(key, node_id)| {
                        if let DocumentKey::TupleIndex(idx) = key {
                            let child_node = doc.get_node(*node_id);
                            let child_value = node_to_value(doc, child_node);
                            Some((*idx, child_value))
                        } else {
                            None
                        }
                    })
                    .collect();
                
                // Sort by index
                tuple_elements.sort_by_key(|(idx, _)| *idx);
                
                // Extract values in order
                let values: Vec<Value> = tuple_elements.into_iter().map(|(_, v)| v).collect();
                Value::Tuple(Tuple(values))
            } else {
                // Regular map
                for (key, node_id) in entries {
                    let child_node = doc.get_node(*node_id);
                    let child_value = node_to_value(doc, child_node);
                    
                    let key_cmp = match key {
                        DocumentKey::Ident(id) => KeyCmpValue::String(id.to_string()),
                        DocumentKey::Extension(id) => KeyCmpValue::String(format!("${}", id)),
                        DocumentKey::Value(v) => v.clone(),
                        DocumentKey::TupleIndex(idx) => KeyCmpValue::U64(*idx as u64),
                    };
                    
                    map.insert(key_cmp, child_value);
                }
                
                // Add extensions
                for (ext_name, ext_node_id) in &node.extensions {
                    let ext_node = doc.get_node(*ext_node_id);
                    let ext_value = node_to_value(doc, ext_node);
                    map.insert(
                        KeyCmpValue::String(format!("${}", ext_name)),
                        ext_value,
                    );
                }
                
                Value::Map(Map(map))
            }
        }
        NodeContent::Array(node_ids) => {
            let elements: Vec<Value> = node_ids
                .iter()
                .map(|node_id| {
                    let child_node = doc.get_node(*node_id);
                    node_to_value(doc, child_node)
                })
                .collect();
            Value::Array(Array(elements))
        }
    }
}

