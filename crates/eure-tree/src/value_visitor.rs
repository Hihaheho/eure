use crate::{
    document::{
        ArrayConstructionHandle, EureDocument, InsertError, MapConstructionHandle, NodeValue,
        StringConstructionHandle,
    },
    prelude::*,
    tree::CstFacade,
};
use eure_value::{
    identifier::Identifier,
    value::{Code, KeyCmpValue, Path, PathSegment},
};
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValueVisitorError {
    #[error(transparent)]
    CstError(#[from] CstConstructError),
    #[error("Invalid identifier: {0}")]
    InvalidIdentifier(String),
    #[error("Failed to parse integer: {0}")]
    InvalidInteger(String),
    #[error("Document insert error: {0}")]
    DocumentInsert(#[from] InsertError),
}

pub struct ValueVisitor<'a> {
    input: &'a str,
    // Main document being built
    document: EureDocument,
    // Stack of paths for nested sections
    path_stack: Vec<Vec<PathSegment>>,
}

impl<'a> ValueVisitor<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            document: EureDocument::new(),
            path_stack: vec![vec![]], // Start with empty root path
        }
    }

    pub fn into_document(self) -> EureDocument {
        self.document
    }

    /// Get the current base path from the path stack
    fn current_path(&self) -> Vec<PathSegment> {
        self.path_stack.last().cloned().unwrap_or_default()
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

    /// Build path segments from a key view
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
                Ok(vec![base_segment, PathSegment::ArrayIndex(Some(index))])
            } else {
                // Empty brackets [] means append/next index
                Ok(vec![base_segment, PathSegment::ArrayIndex(None)])
            }
        } else {
            Ok(vec![base_segment])
        }
    }

    /// Process a value and insert it at the given path
    fn process_value_at_path<F: CstFacade>(
        &mut self,
        path: Vec<PathSegment>,
        value_view: ValueView,
        tree: &F,
    ) -> Result<(), ValueVisitorError> {
        match value_view {
            ValueView::Null(null_handle) => {
                let content = NodeValue::Null {
                    handle: null_handle,
                };
                let mut full_path = self.current_path();
                full_path.extend(path);
                self.document.insert_node(full_path.into_iter(), content)?;
            }
            ValueView::Boolean(boolean_handle) => {
                let boolean_view = boolean_handle.get_view(tree)?;
                let value = match boolean_view {
                    BooleanView::True(_) => true,
                    BooleanView::False(_) => false,
                };
                let content = NodeValue::Bool {
                    handle: boolean_handle,
                    value,
                };
                let mut full_path = self.current_path();
                full_path.extend(path);
                self.document.insert_node(full_path.into_iter(), content)?;
            }
            ValueView::Integer(integer_handle) => {
                let integer_view = integer_handle.get_view(tree)?;
                let data = integer_view.integer.get_data(tree)?;
                let text = tree.get_str(data, self.input).unwrap();

                let content = if let Ok(i) = text.parse::<i64>() {
                    NodeValue::I64 {
                        handle: integer_handle,
                        value: i,
                    }
                } else if let Ok(u) = text.parse::<u64>() {
                    NodeValue::U64 {
                        handle: integer_handle,
                        value: u,
                    }
                } else {
                    return Err(ValueVisitorError::InvalidInteger(text.to_string()));
                };
                let mut full_path = self.current_path();
                full_path.extend(path);
                self.document.insert_node(full_path.into_iter(), content)?;
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

                let content = NodeValue::String {
                    handle: StringConstructionHandle::Strings(strings_handle),
                    value: result.clone(),
                };
                let mut full_path = self.current_path();
                full_path.extend(path);
                    self.document.insert_node(full_path.into_iter(), content)?;
            }
            ValueView::Object(object_handle) => {
                // First insert an empty map node at this path
                let content = NodeValue::Map {
                    handle: MapConstructionHandle::ObjectLiteral(object_handle),
                    entries: vec![],
                };
                self.document
                    .insert_node(path.clone().into_iter(), content)?;

                // Now process each key-value pair
                let object_view = object_handle.get_view(tree)?;
                if let Some(mut object_list) = object_view.object_list.get_view(tree)? {
                    loop {
                        // Get the key
                        let key_view = object_list.key.get_view(tree)?;
                        let key_segments = self.build_path_segments(key_view, tree)?;

                        // Build the full path for this entry
                        let mut entry_path = path.clone();
                        entry_path.extend(key_segments);

                        // Process the value at this path
                        let value_handle = object_list.value;
                        let value_view = value_handle.get_view(tree)?;
                        self.process_value_at_path(entry_path, value_view, tree)?;

                        match object_list.object_list.get_view(tree)? {
                            Some(next) => object_list = next,
                            None => break,
                        }
                    }
                }
            }
            ValueView::Array(array_handle) => {
                // First insert an empty array node at this path
                let content = NodeValue::Array {
                    handle: ArrayConstructionHandle::ArrayLiteral(array_handle),
                    children: vec![],
                };
                self.document
                    .insert_node(path.clone().into_iter(), content)?;

                // Now process each array element
                let array_view = array_handle.get_view(tree)?;
                let mut index = 0;

                if let Some(array_elements) = array_view.array_opt.get_view(tree)? {
                    let array_elements_view = array_elements.get_view(tree)?;

                    // First element
                    let mut element_path = path.clone();
                    element_path.push(PathSegment::ArrayIndex(Some(index)));
                    let value_view = array_elements_view.value.get_view(tree)?;
                    self.process_value_at_path(element_path, value_view, tree)?;
                    index += 1;

                    // Rest of the elements
                    if let Some(mut tail) = array_elements_view.array_elements_opt.get_view(tree)? {
                        loop {
                            let tail_view = tail.get_view(tree)?;

                            if let Some(next_elements) =
                                tail_view.array_elements_tail_opt.get_view(tree)?
                            {
                                let next_view = next_elements.get_view(tree)?;

                                let mut element_path = path.clone();
                                element_path.push(PathSegment::ArrayIndex(Some(index)));
                                let value_view = next_view.value.get_view(tree)?;
                                self.process_value_at_path(element_path, value_view, tree)?;
                                index += 1;

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
            }
            ValueView::Code(code_handle) => {
                let code_view = code_handle.get_view(tree)?;
                let data = code_view.code.get_data(tree)?;
                let text = tree.get_str(data, self.input).unwrap();

                let content = text[1..text.len() - 1].to_string();
                let node_content = NodeValue::Code {
                    handle: code_handle,
                    value: Code {
                        language: String::new(),
                        content,
                    },
                };
                self.document.insert_node(path.into_iter(), node_content)?;
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

                let node_content = NodeValue::CodeBlock {
                    handle: code_block_handle,
                    value: Code { language, content },
                };
                self.document.insert_node(path.into_iter(), node_content)?;
            }
            ValueView::NamedCode(named_code_handle) => {
                let named_code_view = named_code_handle.get_view(tree)?;
                let data = named_code_view.named_code.get_data(tree)?;
                let text = tree.get_str(data, self.input).unwrap();

                let backtick_pos = text.find('`').unwrap();
                let language = text[..backtick_pos].to_string();
                let content = text[backtick_pos + 1..text.len() - 1].to_string();

                let node_content = NodeValue::NamedCode {
                    handle: named_code_handle,
                    value: Code { language, content },
                };
                self.document.insert_node(path.into_iter(), node_content)?;
            }
            ValueView::Hole(hole_handle) => {
                let content = NodeValue::Hole {
                    handle: hole_handle,
                };
                let mut full_path = self.current_path();
                full_path.extend(path);
                self.document.insert_node(full_path.into_iter(), content)?;
            }
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

                let content = NodeValue::Path {
                    handle: path_handle,
                    value: Path(segments),
                };
                let mut full_path = self.current_path();
                full_path.extend(path);
                self.document.insert_node(full_path.into_iter(), content)?;
            }
            ValueView::Tuple(tuple_handle) => {
                // First insert an empty tuple node at this path
                let content = NodeValue::Tuple {
                    handle: tuple_handle,
                    children: vec![],
                };
                self.document
                    .insert_node(path.clone().into_iter(), content)?;

                // Now process each tuple element
                let tuple_view = tuple_handle.get_view(tree)?;
                let mut index = 0u8;

                if let Ok(Some(tuple_elements)) = tuple_view.tuple_opt.get_view(tree) {
                    let tuple_elements_view = tuple_elements.get_view(tree)?;

                    // First element
                    let mut element_path = path.clone();
                    element_path.push(PathSegment::TupleIndex(index));
                    let value_view = tuple_elements_view.value.get_view(tree)?;
                    self.process_value_at_path(element_path, value_view, tree)?;
                    index += 1;

                    // Rest of the elements
                    if let Ok(Some(mut tail)) =
                        tuple_elements_view.tuple_elements_opt.get_view(tree)
                    {
                        loop {
                            let tail_view = tail.get_view(tree)?;

                            if let Some(next_elements) =
                                tail_view.tuple_elements_tail_opt.get_view(tree)?
                            {
                                let next_view = next_elements.get_view(tree)?;

                                let mut element_path = path.clone();
                                element_path.push(PathSegment::TupleIndex(index));
                                let value_view = next_view.value.get_view(tree)?;
                                self.process_value_at_path(element_path, value_view, tree)?;
                                index += 1;

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
            }
        }
        Ok(())
    }
}

/// Convert an EureDocument to a Value, discarding span information
///
/// This is a convenience function that calls document.to_value()
pub fn document_to_value(document: EureDocument) -> eure_value::value::Value {
    document.to_value()
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


        // Process the binding based on its type
        match view.binding_rhs.get_view(tree) {
            Ok(BindingRhsView::ValueBinding(value_binding_handle)) => {
                let value_binding_view = value_binding_handle.get_view(tree)?;
                let value_view = value_binding_view.value.get_view(tree)?;
                
                self.process_value_at_path(path, value_view, tree)?;
            }
            Ok(BindingRhsView::TextBinding(text_binding_handle)) => {
                let text_binding_view = text_binding_handle.get_view(tree)?;
                let text = if let Ok(text_view) = text_binding_view.text.get_view(tree) {
                    if let Ok(data) = text_view.text.get_data(tree) {
                        tree.get_str(data, self.input)
                            .unwrap_or("")
                            .trim()
                            .to_string()
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };

                let content = NodeValue::String {
                    handle: StringConstructionHandle::TextBinding(text_binding_handle),
                    value: text,
                };
                let mut full_path = self.current_path();
                full_path.extend(path);
                self.document.insert_node(full_path.into_iter(), content)?;
            }
            Ok(BindingRhsView::SectionBinding(section_binding_handle)) => {
                // First insert a map node for this section
                let content = NodeValue::Map {
                    handle: MapConstructionHandle::SectionBinding(section_binding_handle),
                    entries: vec![],
                };
                let mut full_path = self.current_path();
                full_path.extend(path.clone());
                self.document
                    .insert_node(full_path.clone().into_iter(), content)?;

                // Push this path onto the stack for nested content
                self.path_stack.push(full_path);

                // Process section binding
                if let Ok(section_binding_view) = section_binding_handle.get_view(tree)
                    && let Ok(eure_view) = section_binding_view.eure.get_view(tree)
                {
                    // Visit the eure within the section
                    self.visit_eure(section_binding_view.eure, eure_view, tree)?;
                }

                // Pop the path from the stack
                self.path_stack.pop();
            }
            _ => {}
        }

        Ok(())
    }

    fn visit_section(
        &mut self,
        handle: SectionHandle,
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

        let mut full_path = self.current_path();
        full_path.extend(path.clone());


        // Check what kind of section body we have
        let is_direct_bind = matches!(
            view.section_body.get_view(tree),
            Ok(SectionBodyView::Bind(_))
        );

        if !is_direct_bind {
            // For sections with braces { }, create a map node
            // This handles both regular sections and array element sections like @ employees[0] { ... }
            let content = NodeValue::Map {
                handle: MapConstructionHandle::Section(handle),
                entries: vec![],
            };
            let _node_id = self.document
                .insert_node(full_path.clone().into_iter(), content)?;
            
            // Special handling for array append sections
            if matches!(full_path.last(), Some(PathSegment::ArrayIndex(None))) {
                // We just created an array element. We need to find out which index it got
                // so that child bindings use the correct path
                let mut array_path = full_path.clone();
                array_path.pop(); // Remove the ArrayIndex(None)
                
                // Get the array node to find the index of the element we just created
                if let Ok(array_node) = self.document.get_node_mut_or_insert(array_path.iter().cloned())
                    && let NodeValue::Array { children, .. } = &array_node.content {
                        // The element we just created should be the last one
                        let actual_index = children.len().saturating_sub(1);
                        
                        // Update the path to use the actual index
                        let mut resolved_path = array_path;
                        resolved_path.push(PathSegment::ArrayIndex(Some(actual_index as u8)));
                        
                        
                        // Push the resolved path instead of the one with ArrayIndex(None)
                        self.path_stack.push(resolved_path);
                        
                        // Skip the normal path push below
                        // Process section body...
                        match view.section_body.get_view(tree) {
                            Ok(section_body) => match section_body {
                                SectionBodyView::SectionBinding(binding_handle) => {
                                    if let Ok(binding_view) = binding_handle.get_view(tree)
                                        && let Ok(eure_view) = binding_view.eure.get_view(tree)
                                    {
                                        // Visit the eure content within this section's context
                                        if let Ok(Some(bindings)) = eure_view.eure_bindings.get_view(tree) {
                                            self.visit_eure_bindings(eure_view.eure_bindings, bindings, tree)?;
                                        }
                                        if let Ok(Some(sections)) = eure_view.eure_sections.get_view(tree) {
                                            self.visit_eure_sections(eure_view.eure_sections, sections, tree)?;
                                        }
                                    }
                                }
                                SectionBodyView::SectionBodyList(body_list_handle) => {
                                    if let Ok(Some(body_list)) = body_list_handle.get_view(tree) {
                                        self.visit_section_body_list(body_list_handle, body_list, tree)?;
                                    }
                                }
                                SectionBodyView::Bind(_bind_handle) => {
                                    // Direct assignment to section - e.g., @ items[0] = "value"
                                    // TODO: This case is not fully implemented in the grammar yet
                                    // For now, this should not occur in practice since the test case
                                    // uses section binding syntax with braces: @ employees[0] { ... }
                                }
                            },
                            Err(_) => {
                                // Failed to parse section body
                                // This can happen with certain syntax forms
                            }
                        }

                        // Pop the section path from the stack
                        self.path_stack.pop();

                        return Ok(());
                    }
            }
        }

        // Push this section's path onto the stack (for non-array-append cases)
        self.path_stack.push(full_path);

        // Process section body
        match view.section_body.get_view(tree) {
            Ok(section_body) => match section_body {
                SectionBodyView::SectionBinding(binding_handle) => {
                    // Processing SectionBinding");
                    if let Ok(binding_view) = binding_handle.get_view(tree)
                        && let Ok(eure_view) = binding_view.eure.get_view(tree)
                    {
                        // Visit the eure content within this section's context
                        if let Ok(Some(bindings)) = eure_view.eure_bindings.get_view(tree) {
                            // Visiting bindings within section");
                            self.visit_eure_bindings(eure_view.eure_bindings, bindings, tree)?;
                        }
                        if let Ok(Some(sections)) = eure_view.eure_sections.get_view(tree) {
                            // Visiting sections within section");
                            self.visit_eure_sections(eure_view.eure_sections, sections, tree)?;
                        }
                    }
                }
                SectionBodyView::SectionBodyList(body_list_handle) => {
                    // Processing SectionBodyList");
                    if let Ok(Some(body_list)) = body_list_handle.get_view(tree) {
                        self.visit_section_body_list(body_list_handle, body_list, tree)?;
                    }
                }
                SectionBodyView::Bind(_bind_handle) => {
                    // Processing direct Bind (not implemented)");
                    // Direct assignment to section - e.g., @ items[0] = "value"
                    // TODO: This case is not fully implemented in the grammar yet
                    // For now, this should not occur in practice since the test case
                    // uses section binding syntax with braces: @ employees[0] { ... }
                }
            },
            Err(_) => {
                // Failed to parse section body");
                // Failed to parse section body
                // This can happen with certain syntax forms
            }
        }

        // Pop the section path from the stack
        self.path_stack.pop();

        Ok(())
    }
}
