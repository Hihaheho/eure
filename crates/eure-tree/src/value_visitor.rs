use ahash::AHashMap;
use eure_value::{
    identifier::Identifier, value::Array, value::Code, value::KeyCmpValue, value::Map,
    value::PathSegment, value::Tuple, value::TypedString, value::Value,
};
use std::str::FromStr;
use thiserror::Error;

use crate::{prelude::*, tree::CstFacade};

pub struct Values {
    ident_handles: AHashMap<IdentHandle, Identifier>,
    key_handles: AHashMap<KeyHandle, PathSegment>,
    keys_handles: AHashMap<KeysHandle, Vec<KeyHandle>>,
    value_handles: AHashMap<ValueHandle, Value>,
    eure_handles: AHashMap<EureHandle, Value>,
}

impl Default for Values {
    fn default() -> Self {
        Self {
            ident_handles: AHashMap::new(),
            key_handles: AHashMap::new(),
            keys_handles: AHashMap::new(),
            value_handles: AHashMap::new(),
            eure_handles: AHashMap::new(),
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

    pub fn get_eure(&self, handle: &EureHandle) -> Option<&Value> {
        self.eure_handles.get(handle)
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
    document_map: AHashMap<KeyCmpValue, Value>,
    current_section_stack: Vec<AHashMap<KeyCmpValue, Value>>,
}

impl<'a> ValueVisitor<'a> {
    pub fn new(input: &'a str, values: &'a mut Values) -> Self {
        Self {
            input,
            values,
            current_keys: Vec::new(),
            document_map: AHashMap::new(),
            current_section_stack: Vec::new(),
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

    fn visit_keys(
        &mut self,
        handle: KeysHandle,
        view: KeysView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.current_keys.clear();
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
                    PathSegment::Ident(identifier.clone())
                } else {
                    return Ok(());
                }
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
            let index =
                if let Some(integer_handle) = array_marker_view.array_marker_opt.get_view(tree)? {
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
                PathSegment::Ident(ident) => Value::String(ident.to_string()),
                PathSegment::Extension(ident) => Value::String(ident.to_string()),
                PathSegment::Value(val) => match val {
                    KeyCmpValue::Null => Value::Null,
                    KeyCmpValue::Bool(b) => Value::Bool(*b),
                    KeyCmpValue::I64(i) => Value::I64(*i),
                    KeyCmpValue::U64(u) => Value::U64(*u),
                    KeyCmpValue::String(s) => Value::String(s.clone()),
                    KeyCmpValue::Tuple(t) => Value::Tuple(Tuple(t.0.iter().map(|v| match v {
                        KeyCmpValue::Null => Value::Null,
                        KeyCmpValue::Bool(b) => Value::Bool(*b),
                        KeyCmpValue::I64(i) => Value::I64(*i),
                        KeyCmpValue::U64(u) => Value::U64(*u),
                        KeyCmpValue::String(s) => Value::String(s.clone()),
                        KeyCmpValue::Tuple(_) => Value::Null, // Nested tuples not supported
                        KeyCmpValue::Unit => Value::Unit,
                    }).collect())),
                    KeyCmpValue::Unit => Value::Unit,
                },
                PathSegment::Array { .. } => {
                    // Nested array syntax not expected here
                    return Ok(());
                }
            };

            PathSegment::Array {
                key: key_value,
                index,
            }
        } else {
            path_segment
        };

        // Store the PathSegment
        self.values.key_handles.insert(handle, final_path_segment);
        self.current_keys.push(handle);

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
                if let Some(array_elements_handle) = array_view.array_opt.get_view(tree)? {
                    // Get the ArrayElementsView
                    let array_elements_view = array_elements_handle.get_view(tree)?;

                    // Visit and collect the first element
                    self.visit_value_handle(array_elements_view.value, tree)?;
                    if let Some(value) = self.values.value_handles.get(&array_elements_view.value) {
                        elements.push(value.clone());
                    }

                    // Collect remaining elements if they exist
                    if let Some(array_elements_tail_handle) =
                        array_elements_view.array_elements_opt.get_view(tree)?
                    {
                        self.collect_array_elements_tail(
                            &mut elements,
                            array_elements_tail_handle,
                            tree,
                        )?;
                    }
                }

                Value::Array(Array(elements))
            }
            ValueView::Code(code_handle) => {
                // Get the code view to access the terminal
                let code_view = code_handle.get_view(tree)?;
                let data = code_view.code.get_data(tree)?;
                let text = tree.get_str(data, self.input).unwrap();

                // Remove backticks and create Code with empty language
                let content = text[1..text.len() - 1].to_string();
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
                let without_fences = &text[3..text.len() - 3];
                let newline_pos = without_fences.find('\n').unwrap_or(0);

                let language = without_fences[..newline_pos].to_string();
                let content = if newline_pos < without_fences.len() {
                    without_fences[newline_pos + 1..].to_string()
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
                let value = text[backtick_pos + 1..text.len() - 1].to_string();

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

    fn visit_eure(
        &mut self,
        handle: EureHandle,
        view: EureView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Clear the document map for this eure
        self.document_map.clear();

        // Visit the eure structure
        self.visit_eure_super(handle, view, tree)?;

        // Transform variants and store the final document value
        let document_value = transform_variants(Value::Map(Map(self.document_map.clone())));
        self.values.eure_handles.insert(handle, document_value);

        Ok(())
    }

    fn visit_eure_bindings(
        &mut self,
        _handle: EureBindingsHandle,
        view: EureBindingsView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Visit the binding
        if let Ok(binding_view) = view.binding.get_view(tree) {
            self.visit_binding(view.binding, binding_view, tree)?;
        }

        // Visit remaining bindings recursively
        if let Ok(Some(more_bindings)) = view.eure_bindings.get_view(tree) {
            self.visit_eure_bindings(_handle, more_bindings, tree)?;
        }

        Ok(())
    }

    fn visit_binding(
        &mut self,
        handle: BindingHandle,
        view: BindingView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // First visit the binding using the default implementation to process keys
        self.visit_binding_super(handle, view, tree)?;

        // Get the key path
        if let Some(key_handles) = self.values.get_keys(&view.keys).cloned()
            && !key_handles.is_empty()
        {
            // Get the value to bind
            let binding_value = match view.binding_rhs.get_view(tree) {
                Ok(BindingRhsView::ValueBinding(value_binding_handle)) => {
                    if let Ok(value_binding_view) = value_binding_handle.get_view(tree) {
                        self.values.get_value(&value_binding_view.value).cloned()
                    } else {
                        None
                    }
                }
                Ok(BindingRhsView::TextBinding(text_binding_handle)) => {
                    if let Ok(text_binding_view) = text_binding_handle.get_view(tree) {
                        if let Ok(text_view) = text_binding_view.text.get_view(tree) {
                            if let Ok(data) = text_view.text.get_data(tree) {
                                let text = tree.get_str(data, self.input).unwrap_or("").trim();
                                Some(Value::String(text.to_string()))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                Ok(BindingRhsView::SectionBinding(section_binding_handle)) => {
                    // Create a new section map
                    let section_map = AHashMap::new();
                    self.current_section_stack.push(section_map);

                    // Process section binding
                    if let Ok(section_binding_view) = section_binding_handle.get_view(tree)
                        && let Ok(eure_view) = section_binding_view.eure.get_view(tree)
                    {
                        // Visit the eure within the section
                        self.visit_eure(section_binding_view.eure, eure_view, tree)?;
                    }

                    // Pop the section map and use it as the value
                    self.current_section_stack
                        .pop()
                        .map(|section_map| Value::Map(Map(section_map)))
                }
                _ => None,
            };

            // Process the path and bind the value
            if let Some(value) = binding_value {
                let target_map = self
                    .current_section_stack
                    .last_mut()
                    .unwrap_or(&mut self.document_map);
                process_path_recursive(target_map, &key_handles, value, self.values);
            }
        }

        Ok(())
    }

    fn visit_eure_sections(
        &mut self,
        _handle: EureSectionsHandle,
        view: EureSectionsView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Visit the section
        if let Ok(section_view) = view.section.get_view(tree) {
            self.visit_section(view.section, section_view, tree)?;
        }

        // Visit remaining sections recursively
        if let Ok(Some(more_sections)) = view.eure_sections.get_view(tree) {
            self.visit_eure_sections(_handle, more_sections, tree)?;
        }

        Ok(())
    }

    fn visit_section(
        &mut self,
        handle: SectionHandle,
        view: SectionView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Get keys from the section before visiting
        let key_handles = self.values.get_keys(&view.keys).cloned();

        // Visit the section using the default implementation
        self.visit_section_super(handle, view, tree)?;

        // Process the section if we have valid keys
        if let Some(key_handles) = key_handles
            && !key_handles.is_empty()
        {
            // The section content will be processed via visit_section_body_list or visit_section_binding
            // For now, we'll handle this in the visit_eure method when transforming the document
        }

        Ok(())
    }

    fn visit_section_body_list(
        &mut self,
        handle: SectionBodyListHandle,
        view: SectionBodyListView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Create a new map for this section body
        let section_map = AHashMap::new();
        self.current_section_stack.push(section_map);

        // Visit the section body list
        self.visit_section_body_list_super(handle, view, tree)?;

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

    /// Collect array elements from the tail (after the first element)
    fn collect_array_elements_tail<F: CstFacade>(
        &mut self,
        elements: &mut Vec<Value>,
        array_elements_tail_handle: ArrayElementsTailHandle,
        tree: &F,
    ) -> Result<(), ValueVisitorError> {
        // Get the tail view which contains comma and optional next elements
        let array_elements_tail_view = array_elements_tail_handle.get_view(tree)?;

        // Check if there are more elements after the comma
        if let Some(array_elements_handle) = array_elements_tail_view
            .array_elements_tail_opt
            .get_view(tree)?
        {
            // Get the ArrayElementsView for the next element
            let array_elements_view = array_elements_handle.get_view(tree)?;

            // Visit and collect the value
            self.visit_value_handle(array_elements_view.value, tree)?;
            if let Some(value) = self.values.value_handles.get(&array_elements_view.value) {
                elements.push(value.clone());
            }

            // Recursively collect more elements if they exist
            if let Some(next_tail_handle) = array_elements_view.array_elements_opt.get_view(tree)? {
                self.collect_array_elements_tail(elements, next_tail_handle, tree)?;
            }
        }
        // If array_elements_tail_opt is None, we just have a trailing comma

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
        if let Some(key_cmp_value) = self.key_handle_to_key_cmp_value(object_list_view.key, tree)?
            && let Some(value) = self.values.value_handles.get(&object_list_view.value)
        {
            map.insert(key_cmp_value, value.clone());
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
                    // Preserve the $ prefix when converting to a string key
                    Ok(Some(KeyCmpValue::String(format!("${identifier}"))))
                } else {
                    Ok(None)
                }
            }
        }
    }
}

// Helper functions for document-level processing

fn transform_variants(value: Value) -> Value {
    match value {
        Value::Map(Map(mut map)) => {
            // Check if this map has a $variant field
            let variant_name = map
                .get(&KeyCmpValue::String("$variant".to_string()))
                .and_then(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    _ => None,
                });

            if let Some(name) = variant_name {
                // Remove $variant field
                map.remove(&KeyCmpValue::String("$variant".to_string()));

                // Remove the representation field if present
                map.remove(&KeyCmpValue::String("$variant.repr".to_string()));

                // Transform remaining values in the map
                let mut transformed_map = AHashMap::new();
                for (k, v) in map {
                    transformed_map.insert(k, transform_variants(v));
                }

                // Convert variants to a map with the variant name as key
                // This matches the External representation
                let mut variant_map = AHashMap::new();
                variant_map.insert(KeyCmpValue::String(name), Value::Map(Map(transformed_map)));
                Value::Map(Map(variant_map))
            } else {
                // Not a variant, but transform nested values
                let mut transformed_map = AHashMap::new();
                for (k, v) in map {
                    transformed_map.insert(k, transform_variants(v));
                }
                Value::Map(Map(transformed_map))
            }
        }
        Value::Array(Array(elements)) => {
            // Transform each element in the array
            Value::Array(Array(
                elements.into_iter().map(transform_variants).collect(),
            ))
        }
        // Other values pass through unchanged
        other => other,
    }
}

fn process_path_recursive(
    map: &mut AHashMap<KeyCmpValue, Value>,
    key_handles: &[KeyHandle],
    value: Value,
    values: &Values,
) {
    if key_handles.is_empty() {
        return;
    }

    // Convert the first key handle to a key
    let first_segment = values.get_path_segment(&key_handles[0]);
    let (key, is_array_access) = match first_segment {
        Some(PathSegment::Ident(ident)) => (KeyCmpValue::String(ident.to_string()), false),
        Some(PathSegment::Value(val)) => (val.clone(), false),
        Some(PathSegment::Extension(ident)) => {
            // For extension namespace, preserve the $ prefix
            (KeyCmpValue::String(format!("${ident}")), false)
        }
        Some(PathSegment::Array { key, index: _ }) => {
            // Handle array access
            let base_key = match key {
                Value::String(s) => KeyCmpValue::String(s.clone()),
                Value::I64(i) => KeyCmpValue::I64(*i),
                Value::U64(u) => KeyCmpValue::U64(*u),
                _ => return,
            };
            (base_key, true)
        }
        _ => return,
    };

    if key_handles.len() == 1 && !is_array_access {
        // Simple assignment
        map.insert(key, value);
    } else if is_array_access {
        // Handle array access
        if let Some(PathSegment::Array { index, .. }) = first_segment {
            let array = map
                .entry(key)
                .or_insert_with(|| Value::Array(Array(Vec::new())));

            if let Value::Array(Array(elements)) = array {
                // Ensure the array is large enough
                let idx = match index {
                    Some(Value::I64(i)) => *i as usize,
                    Some(Value::U64(u)) => *u as usize,
                    _ => elements.len(), // Append to end
                };

                if idx >= elements.len() {
                    elements.resize(idx + 1, Value::Null);
                }

                if key_handles.len() == 1 {
                    // Direct array element assignment
                    elements[idx] = value;
                } else {
                    // Nested path within array element
                    if let Value::Map(Map(nested_map)) = &mut elements[idx] {
                        process_path_recursive(nested_map, &key_handles[1..], value, values);
                    } else {
                        // Create a new map for nested assignment
                        let mut nested_map = AHashMap::new();
                        process_path_recursive(&mut nested_map, &key_handles[1..], value, values);
                        elements[idx] = Value::Map(Map(nested_map));
                    }
                }
            }
        }
    } else {
        // Nested object path
        let nested = map
            .entry(key)
            .or_insert_with(|| Value::Map(Map(AHashMap::new())));

        if let Value::Map(Map(nested_map)) = nested {
            process_path_recursive(nested_map, &key_handles[1..], value, values);
        }
    }
}
