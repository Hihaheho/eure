//! Path context extraction for completion support
//!
//! This module provides utilities to extract the current path context
//! from a document at a given cursor position.

use eure_tree::{
    Cst,
    nodes::{BindingRhsView, BindingView, SectionBodyView, SectionView},
    prelude::*,
    tree::{
        CstFacade, CstNodeData, CstNodeId, InputSpan, NonTerminalData, RecursiveView, TerminalData,
    },
};
use eure_value::value::PathSegment;
use lsp_types::Position;

#[derive(Debug, Clone)]
pub struct ArrayContext {
    pub path_index: usize,
    pub has_index: bool,
}

#[derive(Debug, Clone)]
pub struct PathContext {
    pub path_segments: Vec<PathSegment>,
    pub variant_context: Option<String>,
    pub array_contexts: Vec<ArrayContext>,
    pub is_in_value_position: bool,
    pub is_in_key_position: bool,
    pub is_variant_position: bool,
    pub is_string_only: bool,
    pub parent_path: Option<String>,
    pub partial_field: Option<String>,
}

pub struct PathContextExtractor<'a> {
    input: &'a str,
    values: &'a Values,
    position: Position,
    current_path: Vec<PathSegment>,
    variant_stack: Vec<Option<String>>,
    array_contexts: Vec<ArrayContext>,
    found_context: Option<PathContext>,
    current_line: u32,
    current_column: u32,
}

impl<'a> PathContextExtractor<'a> {
    pub fn new(input: &'a str, values: &'a Values, position: Position) -> Self {
        Self {
            input,
            values,
            position,
            current_path: Vec::new(),
            variant_stack: vec![None],
            array_contexts: Vec::new(),
            found_context: None,
            current_line: 0,
            current_column: 0,
        }
    }

    pub fn extract_context(mut self, tree: &Cst) -> Option<PathContext> {
        let _ = tree.visit_from_root(&mut self);
        self.found_context
    }

    fn get_span_from_node<F: CstFacade>(&self, node_id: CstNodeId, tree: &F) -> Option<InputSpan> {
        if let Some(node_data) = tree.node_data(node_id) {
            match node_data {
                CstNodeData::Terminal {
                    data: TerminalData::Input(span),
                    ..
                } => Some(span),
                CstNodeData::NonTerminal {
                    data: NonTerminalData::Input(span),
                    ..
                } => Some(span),
                _ => None,
            }
        } else {
            None
        }
    }

    fn update_position(&mut self, span: InputSpan) {
        // Count lines and columns up to span start
        let mut line = 0;
        let mut column = 0;
        for (i, ch) in self.input.char_indices() {
            if i >= span.start as usize {
                break;
            }
            if ch == '\n' {
                line += 1;
                column = 0;
            } else {
                column += ch.len_utf16() as u32;
            }
        }
        self.current_line = line;
        self.current_column = column;
    }

    fn is_at_cursor_position(&self, span: InputSpan) -> bool {
        // Check if span contains the cursor position
        let mut line = 0;
        let mut column = 0;
        let mut start_line = 0;
        let mut start_column = 0;
        let mut found_start = false;

        for (i, ch) in self.input.char_indices() {
            if i == span.start as usize {
                start_line = line;
                start_column = column;
                found_start = true;
            }

            if found_start && i >= span.end as usize {
                // Check if cursor is within this span
                return (self.position.line >= start_line && self.position.line <= line)
                    && (self.position.line > start_line
                        || self.position.character >= start_column)
                    && (self.position.line < line || self.position.character <= column);
            }

            if ch == '\n' {
                line += 1;
                column = 0;
            } else {
                column += ch.len_utf16() as u32;
            }
        }

        false
    }

    fn extract_variant_from_section<F: CstFacade>(
        &self,
        section_view: &SectionView,
        tree: &F,
    ) -> Option<String> {
        // Look for $variant field in section body
        if let Ok(section_body_view) = section_view.section_body.get_view(tree) {
            match section_body_view {
                SectionBodyView::SectionBinding(binding_handle) => {
                    // Extensions are not in the value map - need to check the bindings directly
                    if let Ok(binding_view) = binding_handle.get_view(tree)
                        && let Ok(eure_view) = binding_view.eure.get_view(tree)
                        && let Ok(Some(bindings)) = eure_view.eure_bindings.get_view(tree)
                    {
                        // Check bindings for $variant
                        if let Ok(bindings_vec) = bindings.get_all(tree) {
                            for binding in bindings_vec {
                                if let Ok(binding_view) = binding.get_view(tree)
                                    && let Some(key_handles) =
                                        self.values.get_keys(&binding_view.keys)
                                    && key_handles.len() == 1
                                    && let Some((segment, _)) =
                                        self.values.get_key_with_span(&key_handles[0])
                                    && let PathSegment::Extension(ext) = segment
                                    && ext.as_ref() == "variant"
                                {
                                    // Found $variant field
                                    match binding_view.binding_rhs.get_view(tree) {
                                        Ok(BindingRhsView::ValueBinding(value_binding)) => {
                                            if let Ok(value_binding_view) =
                                                value_binding.get_view(tree)
                                                && let Some(value) =
                                                    self.values.get_value(&value_binding_view.value)
                                                && let eure_value::value::Value::String(
                                                    variant_name,
                                                ) = value
                                            {
                                                return Some(variant_name.clone());
                                            }
                                        }
                                        Ok(BindingRhsView::TextBinding(text_binding)) => {
                                            if let Ok(text_binding_view) =
                                                text_binding.get_view(tree)
                                            {
                                                if let Ok(text_view) =
                                                    text_binding_view.text.get_view(tree)
                                                {
                                                    if let Ok(data) = text_view.text.get_data(tree)
                                                        && let Some(text) =
                                                            tree.get_str(data, self.input)
                                                    {
                                                        let variant_name = text.trim();
                                                        return Some(variant_name.to_string());
                                                    }
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
                SectionBodyView::SectionBodyList(list_handle) => {
                    // Iterate through bindings to find $variant field
                    if let Ok(Some(list_view)) = list_handle.get_view(tree)
                        && let Ok(bindings) = list_view.get_all(tree)
                    {
                        for binding_handle in bindings.iter() {
                            if let Ok(binding_view) = binding_handle.get_view(tree)
                                && let Some(key_handles) = self.values.get_keys(&binding_view.keys)
                                && key_handles.len() == 1
                                && let Some((segment, _)) =
                                    self.values.get_key_with_span(&key_handles[0])
                                && let PathSegment::Extension(ext) = segment
                                && ext.as_ref() == "variant"
                            {
                                // Found $variant field
                                match binding_view.binding_rhs.get_view(tree) {
                                    Ok(BindingRhsView::ValueBinding(value_binding)) => {
                                        if let Ok(value_binding_view) = value_binding.get_view(tree)
                                            && let Some(value) =
                                                self.values.get_value(&value_binding_view.value)
                                            && let eure_value::value::Value::String(variant_name) =
                                                value
                                        {
                                            return Some(variant_name.clone());
                                        }
                                    }
                                    Ok(BindingRhsView::TextBinding(text_binding)) => {
                                        if let Ok(text_binding_view) = text_binding.get_view(tree) {
                                            // text_binding_view.text is a TextHandle
                                            if let Ok(text_view) =
                                                text_binding_view.text.get_view(tree)
                                            {
                                                // Get the text directly from the tree
                                                if let Ok(data) = text_view.text.get_data(tree)
                                                    && let Some(text) =
                                                        tree.get_str(data, self.input)
                                                {
                                                    let variant_name = text.trim();
                                                    return Some(variant_name.to_string());
                                                }
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn check_cursor_in_line(&mut self, text: &str, line_offset: u32) {
        if self.found_context.is_some() {
            return;
        }

        // Check if cursor is on this line
        if self.position.line != self.current_line + line_offset {
            return;
        }

        let char_pos = self.position.character as usize;
        if char_pos > text.len() {
            return;
        }

        let line_before_cursor = &text[..char_pos];

        // Analyze the line to determine context
        let mut is_in_value_position = false;
        let mut is_in_key_position = false;
        let mut is_variant_position = false;
        let mut is_string_only = false;
        let mut parent_path = None;
        let mut partial_field = None;

        // Check for various patterns
        if line_before_cursor.ends_with(':') {
            is_in_value_position = true;
            is_string_only = true;
            if line_before_cursor
                .trim_end_matches(':')
                .trim_end()
                .ends_with("$variant")
            {
                is_variant_position = true;
            }
        } else if line_before_cursor.ends_with('=') {
            is_in_value_position = true;
            if line_before_cursor
                .trim_end_matches('=')
                .trim_end()
                .ends_with("$variant")
            {
                is_variant_position = true;
            }
        } else if line_before_cursor.ends_with('.') {
            is_in_key_position = true;
            // Extract parent path
            if let Some(dot_pos) = line_before_cursor.rfind('.') {
                let before_dot = &line_before_cursor[..dot_pos];
                parent_path = before_dot.split_whitespace().last().map(|s| s.to_string());
            }
        } else if line_before_cursor.ends_with('@') || line_before_cursor.trim().is_empty() {
            is_in_key_position = true;
        } else {
            // Check if we're in the middle of typing a field name
            let trimmed = line_before_cursor.trim();
            if !trimmed.contains('=') && !trimmed.contains(':') {
                is_in_key_position = true;
                // Extract partial field name
                if let Some(last_word) = trimmed.split_whitespace().last()
                    && !last_word.starts_with('@')
                {
                    partial_field = Some(last_word.to_string());
                }
            }
        }

        self.found_context = Some(PathContext {
            path_segments: self.current_path.clone(),
            variant_context: self.variant_stack.last().cloned().flatten(),
            array_contexts: self.array_contexts.clone(),
            is_in_value_position,
            is_in_key_position,
            is_variant_position,
            is_string_only,
            parent_path,
            partial_field,
        });
    }
}

impl<'a, F: CstFacade> CstVisitor<F> for PathContextExtractor<'a> {
    type Error = std::convert::Infallible;

    fn visit_section(
        &mut self,
        _handle: SectionHandle,
        view: SectionView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        if self.found_context.is_some() {
            return Ok(());
        }

        // Extract section path using the values helper
        if let Some(key_handles) = self.values.get_keys(&view.keys) {
            // Build path from keys
            let mut section_path = Vec::new();

            // For nested sections within blocks, preserve parent path
            if !self.current_path.is_empty() {
                section_path.extend(self.current_path.clone());
            }

            for key_handle in key_handles {
                if let Some((segment, _)) = self.values.get_key_with_span(key_handle) {
                    section_path.push(segment.clone());
                }
            }

            // Save current state
            let saved_path = self.current_path.clone();
            let saved_variant = self.variant_stack.len();

            // Update current path
            self.current_path = section_path;

            // Check if this section has a variant
            let variant = self.extract_variant_from_section(&view, tree);
            self.variant_stack.push(variant);

            // Visit the body
            let _ = self.visit_section_super(_handle, view, tree);

            // Restore state
            self.current_path = saved_path;
            self.variant_stack.truncate(saved_variant);
        } else {
            // No keys, just continue visiting
            let _ = self.visit_section_super(_handle, view, tree);
        }

        Ok(())
    }

    fn visit_binding(
        &mut self,
        handle: BindingHandle,
        view: BindingView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        if self.found_context.is_some() {
            return Ok(());
        }

        // Get the span of this binding to check cursor position
        if let Some(span) = self.get_span_from_node(handle.node_id(), tree)
            && self.is_at_cursor_position(span)
        {
            // Extract the line containing the binding
            let line_start = self.input[..span.start as usize]
                .rfind('\n')
                .map(|pos| pos + 1)
                .unwrap_or(0);

            let line_end = self.input[span.start as usize..]
                .find('\n')
                .map(|pos| span.start as usize + pos)
                .unwrap_or(self.input.len());

            let line_text = &self.input[line_start..line_end];

            // Calculate line offset
            let line_offset = self.input[..line_start]
                .chars()
                .filter(|&c| c == '\n')
                .count() as u32;

            self.check_cursor_in_line(line_text, line_offset);
        }

        // Continue visiting children
        let _ = self.visit_binding_super(handle, view, tree);

        Ok(())
    }

    fn visit_eure(
        &mut self,
        handle: EureHandle,
        view: EureView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Visit all items in the document using the super method
        self.visit_eure_super(handle, view, tree)
    }
}
