use ahash::AHashMap;
use eure_value::{identifier::Identifier, value::PathSegment, value::Value, value::Code, value::TypedString, value::Array, value::Map, value::KeyCmpValue};
use std::str::FromStr;
use thiserror::Error;

use crate::{prelude::*, tree::CstFacade};

pub struct Values {
    ident_handles: AHashMap<IdentHandle, Identifier>,
    key_handles: AHashMap<KeyHandle, PathSegment>,
    keys_handles: AHashMap<KeysHandle, Vec<KeyHandle>>,
    value_handles: AHashMap<ValueHandle, Value>,
}

impl Default for Values {
    fn default() -> Self {
        Self {
            ident_handles: AHashMap::new(),
            key_handles: AHashMap::new(),
            keys_handles: AHashMap::new(),
            value_handles: AHashMap::new(),
        }
    }
}

impl Values {
    pub fn get_value(&self, handle: &ValueHandle) -> Option<&Value> {
        self.value_handles.get(handle)
    }
    
    pub fn get_identifier(&self, handle: &IdentHandle) -> Option<&Identifier> {
        self.ident_handles.get(handle)
    }
    
    pub fn get_path_segment(&self, handle: &KeyHandle) -> Option<&PathSegment> {
        self.key_handles.get(handle)
    }
    
    pub fn get_keys(&self, handle: &KeysHandle) -> Option<&Vec<KeyHandle>> {
        self.keys_handles.get(handle)
    }
    
    #[cfg(test)]
    pub(crate) fn test_value_handles(&self) -> &AHashMap<ValueHandle, Value> {
        &self.value_handles
    }
    
    #[cfg(test)]
    pub(crate) fn test_ident_handles(&self) -> &AHashMap<IdentHandle, Identifier> {
        &self.ident_handles
    }
    
    #[cfg(test)]
    pub(crate) fn test_key_handles(&self) -> &AHashMap<KeyHandle, PathSegment> {
        &self.key_handles
    }
    
    #[cfg(test)]
    pub(crate) fn test_keys_handles(&self) -> &AHashMap<KeysHandle, Vec<KeyHandle>> {
        &self.keys_handles
    }
}

pub struct ValueVisitor<'a> {
    input: &'a str,
    values: &'a mut Values,
    current_keys: Vec<KeyHandle>,
}

impl<'a> ValueVisitor<'a> {
    pub fn new(input: &'a str, values: &'a mut Values) -> Self {
        Self {
            input,
            values,
            current_keys: Vec::new(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ValueVisitorError {
    #[error(transparent)]
    CstError(#[from] CstConstructError),
    #[error("Invalid identifier: {0}")]
    InvalidIdentifier(String),
    #[error("Failed to parse integer: {0}")]
    InvalidInteger(String),
    #[error("Failed to parse string: {0}")]
    InvalidString(String),
}

impl<F: CstFacade> CstVisitor<F> for ValueVisitor<'_> {
    type Error = ValueVisitorError;

    fn visit_null(
        &mut self,
        _handle: NullHandle,
        _view: NullView,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        // For null, we don't need to process anything special
        // Just return Ok since the actual Value::Null will be created in visit_value
        Ok(())
    }

    fn visit_hole(
        &mut self,
        _handle: HoleHandle,
        _view: HoleView,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        // For hole, we don't need to process anything special
        // Just return Ok since the actual Value::Unit will be created in visit_value
        Ok(())
    }

    fn visit_keys(
        &mut self,
        handle: KeysHandle,
        view: KeysView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        assert_eq!(self.current_keys.len(), 0);
        self.visit_keys_super(handle, view, tree)?;
        self.values
            .keys_handles
            .insert(handle, std::mem::take(&mut self.current_keys));
        Ok(())
    }

    fn visit_key(&mut self, handle: KeyHandle, view: KeyView, tree: &F) -> Result<(), Self::Error> {
        // Visit the key components
        self.visit_key_super(handle, view, tree)?;
        
        // Create PathSegment based on the key type
        let path_segment = match view.key_base.get_view(tree)? {
            KeyBaseView::Ident(ident_handle) => {
                if let Some(identifier) = self.values.ident_handles.get(&ident_handle) {
                    PathSegment::Extension(identifier.clone())
                } else {
                    return Ok(());
                }
            }
            KeyBaseView::Str(str_handle) => {
                let str_view = str_handle.get_view(tree)?;
                let data = str_view.str.get_data(tree)?;
                let text = tree.get_str(data, self.input).unwrap();
                let string_value = self.parse_string_literal(text);
                PathSegment::Value(Value::String(string_value))
            }
            KeyBaseView::Integer(integer_handle) => {
                let integer_view = integer_handle.get_view(tree)?;
                let data = integer_view.integer.get_data(tree)?;
                let text = tree.get_str(data, self.input).unwrap();
                
                let value = if let Ok(i) = text.parse::<i64>() {
                    Value::I64(i)
                } else if let Ok(u) = text.parse::<u64>() {
                    Value::U64(u)
                } else {
                    return Err(ValueVisitorError::InvalidInteger(text.to_string()));
                };
                PathSegment::Value(value)
            }
            KeyBaseView::ExtensionNameSpace(ext_ns_handle) => {
                let ext_ns_view = ext_ns_handle.get_view(tree)?;
                // Extension namespace uses $ prefix, e.g., $eure, $variant
                if let Some(identifier) = self.values.ident_handles.get(&ext_ns_view.ident) {
                    PathSegment::Extension(identifier.clone())
                } else {
                    // Need to visit the identifier first
                    let ident_view = ext_ns_view.ident.get_view(tree)?;
                    self.visit_ident(ext_ns_view.ident, ident_view, tree)?;
                    if let Some(identifier) = self.values.ident_handles.get(&ext_ns_view.ident) {
                        PathSegment::Extension(identifier.clone())
                    } else {
                        return Ok(());
                    }
                }
            }
        };
        
        // Handle array indexing if present
        let final_path_segment = if let Some(array_marker) = view.key_opt.get_view(tree)? {
            // Parse array index from ArrayMarkerView
            let array_marker_view = array_marker.get_view(tree)?;
            
            // Check if there's an index specified
            let index = if let Some(integer_handle) = array_marker_view.array_marker_opt.get_view(tree)? {
                // Get the integer value
                let integer_view = integer_handle.get_view(tree)?;
                let data = integer_view.integer.get_data(tree)?;
                let text = tree.get_str(data, self.input).unwrap();
                
                // Parse the index value
                let index_value = if let Ok(i) = text.parse::<i64>() {
                    Value::I64(i)
                } else if let Ok(u) = text.parse::<u64>() {
                    Value::U64(u)
                } else {
                    return Err(ValueVisitorError::InvalidInteger(text.to_string()));
                };
                Some(index_value)
            } else {
                None
            };
            
            // Convert the base path segment to a value for the key
            let key_value = match &path_segment {
                PathSegment::Extension(ident) => Value::String(ident.to_string()),
                PathSegment::Value(val) => val.clone(),
                PathSegment::Array { .. } => {
                    // Nested array syntax not expected here
                    return Ok(());
                }
            };
            
            PathSegment::Array { key: key_value, index }
        } else {
            path_segment
        };
        
        // Store the PathSegment
        self.values.key_handles.insert(handle, final_path_segment);
        self.current_keys.push(handle);
        
        Ok(())
    }

    fn visit_key_base(
        &mut self,
        handle: KeyBaseHandle,
        view: KeyBaseView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_key_base_super(handle, view, tree)?;
        Ok(())
    }

    fn visit_ident(
        &mut self,
        handle: IdentHandle,
        view: IdentView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        let data = view.ident.get_data(tree)?;
        let text = tree.get_str(data, self.input).unwrap();
        let identifier = Identifier::from_str(text)
            .map_err(|_| ValueVisitorError::InvalidIdentifier(text.to_string()))?;
        self.values.ident_handles.insert(handle, identifier);
        Ok(())
    }

    fn visit_key_opt(
        &mut self,
        handle: KeyOptHandle,
        view: ArrayMarkerHandle,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_key_opt_super(handle, view, tree)?;
        Ok(())
    }

    fn visit_value(
        &mut self,
        handle: ValueHandle,
        view: ValueView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Construct the appropriate value based on the view type
        let value = match view {
            ValueView::Null(_null_handle) => {
                // For null, we don't need to visit children
                // We've overridden visit_null to do nothing, so we can skip the handle call
                Value::Null
            }
            ValueView::Boolean(boolean_handle) => {
                // Get the boolean view to determine true/false
                let boolean_view = boolean_handle.get_view(tree)?;
                self.visit_boolean(boolean_handle, boolean_view, tree)?;
                match boolean_view {
                    BooleanView::True(_) => Value::Bool(true),
                    BooleanView::False(_) => Value::Bool(false),
                }
            }
            ValueView::Integer(integer_handle) => {
                // Get the integer view to access the terminal
                let integer_view = integer_handle.get_view(tree)?;
                let data = integer_view.integer.get_data(tree)?;
                let text = tree.get_str(data, self.input).unwrap();
                
                // Parse as i64 first, then u64 if it fails
                if let Ok(i) = text.parse::<i64>() {
                    Value::I64(i)
                } else if let Ok(u) = text.parse::<u64>() {
                    Value::U64(u)
                } else {
                    return Err(ValueVisitorError::InvalidInteger(text.to_string()));
                }
            }
            ValueView::Strings(strings_handle) => {
                // Get the strings view to access the string parts
                let strings_view = strings_handle.get_view(tree)?;
                
                // Parse the first string
                let first_str_view = strings_view.str.get_view(tree)?;
                let first_str_data = first_str_view.str.get_data(tree)?;
                let first_str_text = tree.get_str(first_str_data, self.input).unwrap();
                
                // Start with the first string (removing quotes)
                let mut result = self.parse_string_literal(first_str_text);
                
                // Check if there are additional string parts
                if let Some(strings_list_view) = strings_view.strings_list.get_view(tree)? {
                    // Recursively collect additional string parts
                    self.collect_string_list_parts(&mut result, strings_list_view, tree)?;
                }
                
                Value::String(result)
            }
            ValueView::Object(object_handle) => {
                // Get the object view
                let object_view = object_handle.get_view(tree)?;
                
                // Collect all key-value pairs
                let mut map = AHashMap::new();
                if let Some(object_list_view) = object_view.object_list.get_view(tree)? {
                    self.collect_object_items(&mut map, object_list_view, tree)?;
                }
                
                Value::Map(Map(map))
            }
            ValueView::Array(array_handle) => {
                // Get the array view
                let array_view = array_handle.get_view(tree)?;
                
                // Collect all array elements
                let mut elements = Vec::new();
                if let Some(array_opt_view) = array_view.array_opt.get_view(tree)? {
                    // Get the first element
                    let first_value_handle = array_opt_view.value;
                    self.visit_value_handle(first_value_handle, tree)?;
                    if let Some(value) = self.values.value_handles.get(&first_value_handle) {
                        elements.push(value.clone());
                    }
                    
                    // Collect remaining elements
                    let more_items_view = array_opt_view.more_items.get_view(tree)?;
                    self.collect_array_elements(&mut elements, more_items_view, tree)?;
                }
                
                Value::Array(Array(elements))
            }
            ValueView::Code(code_handle) => {
                // Get the code view to access the terminal
                let code_view = code_handle.get_view(tree)?;
                let data = code_view.code.get_data(tree)?;
                let text = tree.get_str(data, self.input).unwrap();
                
                // Remove backticks and create Code with empty language
                let content = text[1..text.len()-1].to_string();
                Value::Code(Code {
                    language: String::new(),
                    content,
                })
            }
            ValueView::CodeBlock(code_block_handle) => {
                // Get the code block view to access the terminal
                let code_block_view = code_block_handle.get_view(tree)?;
                let data = code_block_view.code_block.get_data(tree)?;
                let text = tree.get_str(data, self.input).unwrap();
                
                // Parse code block: ```lang\ncontent\n```
                let without_fences = &text[3..text.len()-3];
                let newline_pos = without_fences.find('\n').unwrap_or(0);
                
                let language = without_fences[..newline_pos].to_string();
                let content = if newline_pos < without_fences.len() {
                    without_fences[newline_pos+1..].to_string()
                } else {
                    String::new()
                };
                
                Value::Code(Code { language, content })
            }
            ValueView::NamedCode(named_code_handle) => {
                // Get the named code view to access the terminal
                let named_code_view = named_code_handle.get_view(tree)?;
                let data = named_code_view.named_code.get_data(tree)?;
                let text = tree.get_str(data, self.input).unwrap();
                
                // Parse named code: type`content`
                let backtick_pos = text.find('`').unwrap();
                let type_name = text[..backtick_pos].to_string();
                let value = text[backtick_pos+1..text.len()-1].to_string();
                
                Value::TypedString(TypedString { type_name, value })
            }
            ValueView::Hole(_hole_handle) => {
                // Hole represents a placeholder value "!"
                // We've overridden visit_hole to do nothing, so we can skip the handle call
                // For now, we'll represent it as Unit
                Value::Unit
            }
        };
        
        // Store the constructed value with its handle
        self.values.value_handles.insert(handle, value);
        Ok(())
    }
}

impl<'a> ValueVisitor<'a> {
    /// Parse a string literal, removing quotes and handling escape sequences
    #[cfg(not(test))]
    fn parse_string_literal(&self, text: &str) -> String {
        self.parse_string_literal_impl(text)
    }
    
    /// Parse a string literal, removing quotes and handling escape sequences
    #[cfg(test)]
    pub(crate) fn parse_string_literal(&self, text: &str) -> String {
        self.parse_string_literal_impl(text)
    }
    
    fn parse_string_literal_impl(&self, text: &str) -> String {
        // Remove surrounding quotes
        let without_quotes = &text[1..text.len()-1];
        
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
    
    /// Collect additional string parts from a StringsListView
    fn collect_string_list_parts<F: CstFacade>(
        &mut self,
        result: &mut String,
        strings_list_view: StringsListView,
        tree: &F,
    ) -> Result<(), ValueVisitorError> {
        // Get the string from this list item
        let str_view = strings_list_view.str.get_view(tree)?;
        let str_data = str_view.str.get_data(tree)?;
        let str_text = tree.get_str(str_data, self.input).unwrap();
        result.push_str(&self.parse_string_literal(str_text));
        
        // Check if there are more string parts
        if let Some(next_list_view) = strings_list_view.strings_list.get_view(tree)? {
            self.collect_string_list_parts(result, next_list_view, tree)?;
        }
        
        Ok(())
    }
    
    /// Collect array elements from a MoreItemsView
    fn collect_array_elements<F: CstFacade>(
        &mut self,
        elements: &mut Vec<Value>,
        more_items_view: MoreItemsView,
        tree: &F,
    ) -> Result<(), ValueVisitorError> {
        // Check if there are more items
        if let Some(rest_tail_handle) = more_items_view.more_items_opt.get_view(tree)? {
            let rest_tail_view = rest_tail_handle.get_view(tree)?;
            
            // Visit and collect the next value
            self.visit_value_handle(rest_tail_view.value, tree)?;
            if let Some(value) = self.values.value_handles.get(&rest_tail_view.value) {
                elements.push(value.clone());
            }
            
            // Continue collecting if there are more items
            let next_more_items = rest_tail_view.more_items.get_view(tree)?;
            self.collect_array_elements(elements, next_more_items, tree)?;
        }
        
        Ok(())
    }
    
    /// Collect object key-value pairs from an ObjectListView
    fn collect_object_items<F: CstFacade>(
        &mut self,
        map: &mut AHashMap<KeyCmpValue, Value>,
        object_list_view: ObjectListView,
        tree: &F,
    ) -> Result<(), ValueVisitorError> {
        // Visit the key and value
        self.visit_key_handle(object_list_view.key, tree)?;
        self.visit_value_handle(object_list_view.value, tree)?;
        
        // Convert the key to KeyCmpValue
        if let Some(key_cmp_value) = self.key_handle_to_key_cmp_value(object_list_view.key, tree)? {
            if let Some(value) = self.values.value_handles.get(&object_list_view.value) {
                map.insert(key_cmp_value, value.clone());
            }
        }
        
        // Continue with the rest of the object if present
        if let Some(next_object_list) = object_list_view.object_list.get_view(tree)? {
            self.collect_object_items(map, next_object_list, tree)?;
        }
        
        Ok(())
    }
    
    /// Convert a KeyHandle to a KeyCmpValue for use as a map key
    fn key_handle_to_key_cmp_value<F: CstFacade>(
        &self,
        key_handle: KeyHandle,
        tree: &F,
    ) -> Result<Option<KeyCmpValue>, ValueVisitorError> {
        let key_view = key_handle.get_view(tree)?;
        
        match key_view.key_base.get_view(tree)? {
            KeyBaseView::Ident(ident_handle) => {
                if let Some(identifier) = self.values.ident_handles.get(&ident_handle) {
                    Ok(Some(KeyCmpValue::String(identifier.to_string())))
                } else {
                    Ok(None)
                }
            }
            KeyBaseView::Str(str_handle) => {
                let str_view = str_handle.get_view(tree)?;
                let data = str_view.str.get_data(tree)?;
                let text = tree.get_str(data, self.input).unwrap();
                let string_value = self.parse_string_literal(text);
                Ok(Some(KeyCmpValue::String(string_value)))
            }
            KeyBaseView::Integer(integer_handle) => {
                let integer_view = integer_handle.get_view(tree)?;
                let data = integer_view.integer.get_data(tree)?;
                let text = tree.get_str(data, self.input).unwrap();
                
                if let Ok(i) = text.parse::<i64>() {
                    Ok(Some(KeyCmpValue::I64(i)))
                } else if let Ok(u) = text.parse::<u64>() {
                    Ok(Some(KeyCmpValue::U64(u)))
                } else {
                    Err(ValueVisitorError::InvalidInteger(text.to_string()))
                }
            }
            KeyBaseView::ExtensionNameSpace(ext_ns_handle) => {
                let ext_ns_view = ext_ns_handle.get_view(tree)?;
                // Extension namespace uses $ prefix, e.g., $eure, $variant
                if let Some(identifier) = self.values.ident_handles.get(&ext_ns_view.ident) {
                    Ok(Some(KeyCmpValue::String(identifier.to_string())))
                } else {
                    Ok(None)
                }
            }
        }
    }
}
