//! Proper context tracking for completion support
//! 
//! This module provides a robust context tracker that maintains the complete
//! path context as it traverses the document, properly handling nested sections,
//! arrays, and variants.

use eure_tree::{
    prelude::*,
    tree::{CstFacade, CstNodeId, InputSpan, CstNodeData, TerminalData, NonTerminalData, RecursiveView},
    nodes::{BindingView, SectionView, SectionBodyView, BindingRhsView, ValueBindingView, ObjectView},
    value_visitor::Values,
    Cst,
};
use eure_value::value::{PathSegment, Value as EureValue};
use lsp_types::Position;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CompletionContext {
    /// The complete path from root to current position
    pub path_segments: Vec<PathSegment>,
    /// Current variant context if inside a variant
    pub variant_context: Option<String>,
    /// Whether we're in a position to enter a value
    pub is_in_value_position: bool,
    /// Whether we're in a position to enter a key
    pub is_in_key_position: bool,
    /// Whether this is specifically a variant value position
    pub is_variant_position: bool,
    /// Whether only strings are allowed (after :)
    pub is_string_only: bool,
    /// Partial field name being typed
    pub partial_field: Option<String>,
    /// Map of path to variant context for nested variants
    pub variant_contexts: HashMap<String, String>,
    /// Fields already used at the current level
    pub used_fields: Vec<String>,
}

/// Stack frame for tracking context during traversal
#[derive(Debug, Clone, Default)]
struct ContextFrame {
    /// Path segments for this frame
    path: Vec<PathSegment>,
    /// Variant context if this frame represents a variant
    variant: Option<String>,
    /// Whether this frame is inside an array
    in_array: bool,
    /// Array index if applicable
    array_index: Option<usize>,
}

pub struct CompletionContextTracker<'a> {
    input: &'a str,
    values: &'a Values,
    position: Position,
    /// Stack of context frames
    context_stack: Vec<ContextFrame>,
    /// The found context at cursor position
    found_context: Option<CompletionContext>,
    /// Byte offset of the cursor
    cursor_byte_offset: usize,
    /// Map tracking variants at different paths
    variant_map: HashMap<String, String>,
    /// Map tracking used fields at each path
    used_fields_map: HashMap<String, Vec<String>>,
    /// Track the current section path as we traverse
    /// This is different from context_stack - it persists across array boundaries
    current_section_path: Vec<PathSegment>,
    /// Stack to save/restore section paths when entering/exiting arrays
    section_path_stack: Vec<Vec<PathSegment>>,
}

impl<'a> CompletionContextTracker<'a> {
    pub fn new(input: &'a str, values: &'a Values, position: Position) -> Self {
        let cursor_byte_offset = position_to_byte_offset(input, position);
        eprintln!("CompletionContextTracker::new: position = {position:?}, cursor_byte_offset = {cursor_byte_offset}");
        eprintln!("Input text:\n{input}");
        
        Self {
            input,
            values,
            position,
            context_stack: vec![ContextFrame {
                path: Vec::new(),
                variant: None,
                in_array: false,
                array_index: None,
            }],
            found_context: None,
            cursor_byte_offset,
            variant_map: HashMap::new(),
            used_fields_map: HashMap::new(),
            current_section_path: Vec::new(),
            section_path_stack: Vec::new(),
        }
    }
    
    pub fn track_context(mut self, tree: &Cst) -> Option<CompletionContext> {
        let _ = tree.visit_from_root(&mut self);
        
        // If we haven't found a context during tree traversal, check if we're on an empty line
        if self.found_context.is_none() {
            self.check_cursor_position_directly();
        }
        
        eprintln!("track_context: found_context = {:?}", self.found_context);
        self.found_context
    }
    
    fn current_path(&self) -> Vec<PathSegment> {
        self.context_stack
            .last()
            .map(|frame| frame.path.clone())
            .unwrap_or_default()
    }
    
    
    fn current_variant(&self) -> Option<String> {
        // Look up the stack for the nearest variant context
        for frame in self.context_stack.iter().rev() {
            if let Some(variant) = &frame.variant {
                return Some(variant.clone());
            }
        }
        None
    }
    
    fn push_frame(&mut self, mut path_extension: Vec<PathSegment>, variant: Option<String>) {
        let mut new_path = self.current_path();
        new_path.append(&mut path_extension);
        
        self.context_stack.push(ContextFrame {
            path: new_path,
            variant,
            in_array: false,
            array_index: None,
        });
    }
    
    fn push_array_frame(&mut self, index: Option<usize>) {
        let mut frame = self.context_stack.last().cloned().unwrap_or_default();
        frame.in_array = true;
        frame.array_index = index;
        // Add array segment to path
        frame.path.push(PathSegment::Array { 
            key: EureValue::Null, // No key for array access
            index: index.map(|idx| EureValue::I64(idx as i64))
        });
        self.context_stack.push(frame);
    }
    
    fn pop_frame(&mut self) {
        if self.context_stack.len() > 1 {
            self.context_stack.pop();
        }
    }
    
    fn is_cursor_in_span(&self, span: InputSpan) -> bool {
        self.cursor_byte_offset >= span.start as usize && 
        self.cursor_byte_offset <= span.end as usize
    }
    
    fn is_cursor_after_span(&self, span: InputSpan) -> bool {
        self.cursor_byte_offset > span.end as usize
    }
    
    fn extract_variant_from_bindings<F: CstFacade>(&self, bindings: &[BindingHandle], tree: &F) -> Option<String> {
        for binding_handle in bindings {
            if let Ok(binding_view) = binding_handle.get_view(tree)
                && let Some(key_handles) = self.values.get_keys(&binding_view.keys)
                    && key_handles.len() == 1
                        && let Some((segment, _)) = self.values.get_key_with_span(&key_handles[0])
                            && let PathSegment::Extension(ext) = segment
                                && ext.as_ref() == "variant" {
                                    // Found $variant field, extract its value
                                    match binding_view.binding_rhs.get_view(tree) {
                                        Ok(BindingRhsView::ValueBinding(value_binding)) => {
                                            if let Ok(vb_view) = value_binding.get_view(tree)
                                                && let Some(value) = self.values.get_value(&vb_view.value)
                                                    && let EureValue::String(variant_name) = value {
                                                        return Some(variant_name.clone());
                                                    }
                                        }
                                        Ok(BindingRhsView::TextBinding(text_binding)) => {
                                            if let Ok(tb_view) = text_binding.get_view(tree)
                                                && let Ok(text_view) = tb_view.text.get_view(tree)
                                                    && let Ok(data) = text_view.text.get_data(tree)
                                                        && let Some(text) = tree.get_str(data, self.input) {
                                                            return Some(text.trim().to_string());
                                                        }
                                        }
                                        _ => {}
                                    }
                                }
        }
        None
    }
    
    fn check_completion_position(&mut self, span: InputSpan) {
        if !self.is_cursor_in_span(span) || self.found_context.is_some() {
            eprintln!("check_completion_position: skipping - in_span={}, already_found={}", 
                     self.is_cursor_in_span(span), self.found_context.is_some());
            return;
        }
        
        // Find the line containing the cursor
        let line_start = self.input[..self.cursor_byte_offset]
            .rfind('\n')
            .map(|pos| pos + 1)
            .unwrap_or(0);
            
        let line_end = self.input[self.cursor_byte_offset..]
            .find('\n')
            .map(|pos| self.cursor_byte_offset + pos)
            .unwrap_or(self.input.len());
            
        let line_text = &self.input[line_start..line_end];
        let cursor_pos_in_line = self.cursor_byte_offset - line_start;
        let line_before_cursor = &line_text[..cursor_pos_in_line];
        
        let mut context = CompletionContext {
            path_segments: self.current_path(),
            variant_context: self.current_variant(),
            is_in_value_position: false,
            is_in_key_position: false,
            is_variant_position: false,
            is_string_only: false,
            partial_field: None,
            variant_contexts: self.variant_map.clone(),
            used_fields: vec![],
        };
        
        // Analyze the line to determine position type
        let trimmed = line_before_cursor.trim_end();
        
        if trimmed.ends_with(':') {
            context.is_in_value_position = true;
            context.is_string_only = true;
            if trimmed.trim_end_matches(':').trim_end().ends_with("$variant") {
                context.is_variant_position = true;
            }
        } else if trimmed.ends_with('=') {
            context.is_in_value_position = true;
            if trimmed.trim_end_matches('=').trim_end().ends_with("$variant") {
                context.is_variant_position = true;
            }
        } else if trimmed.ends_with('@') || trimmed.is_empty() || line_before_cursor.ends_with(' ') {
            context.is_in_key_position = true;
        } else {
            // Check if we're typing a partial field name
            if !trimmed.contains('=') && !trimmed.contains(':') {
                context.is_in_key_position = true;
                // Extract partial field after @ if present
                if let Some(at_pos) = trimmed.rfind('@') {
                    let after_at = &trimmed[at_pos + 1..].trim();
                    if !after_at.is_empty() {
                        context.partial_field = Some(after_at.to_string());
                    }
                }
            }
        }
        
        self.found_context = Some(context);
    }
    
    fn check_cursor_position_in_section(&mut self) {
        // Get the line at cursor
        let lines: Vec<&str> = self.input.lines().collect();
        if self.position.line as usize >= lines.len() {
            return;
        }
        
        let current_line = lines[self.position.line as usize];
        let char_pos = self.position.character.min(current_line.len() as u32) as usize;
        let line_before_cursor = &current_line[..char_pos];
        
        eprintln!("check_cursor_position_in_section: line = {:?}, path = {:?}", 
                 line_before_cursor, self.current_path());
        
        // Check if we're in a key position
        if line_before_cursor.trim().ends_with('@') || line_before_cursor.trim().is_empty() {
            // Collect used fields at current level
            let used_fields = self.collect_used_fields_at_current_level();
            
            let context = CompletionContext {
                path_segments: self.current_path(),
                variant_context: self.current_variant(),
                is_in_value_position: false,
                is_in_key_position: true,
                is_variant_position: false,
                is_string_only: false,
                partial_field: None,
                variant_contexts: self.variant_map.clone(),
                used_fields,
            };
            
            eprintln!("Found completion context in section: path = {:?}, used_fields = {:?}", 
                     context.path_segments, context.used_fields);
            self.found_context = Some(context);
        }
    }
    
    fn collect_used_fields_at_current_level(&self) -> Vec<String> {
        let path_key = path_to_string(&self.current_path());
        eprintln!("collect_used_fields_at_current_level: path_key = {}, map = {:?}", 
                 path_key, self.used_fields_map);
        self.used_fields_map
            .get(&path_key)
            .cloned()
            .unwrap_or_default()
    }
    
    fn section_has_array_value<F: CstFacade>(&self, view: &SectionView, tree: &F) -> bool {
        // For sections like @ users = [...], we need a heuristic approach
        // The CST doesn't directly link the section to its value
        
        // Simple heuristic: if the section has a Bind body (=), it might have an array value
        // We'll assume sections with certain names typically contain arrays
        match view.section_body.get_view(tree) {
            Ok(SectionBodyView::Bind(_)) => {
                eprintln!("Section has Bind body - assuming it might be array");
                true // Optimistically assume it's an array
            }
            Ok(SectionBodyView::SectionBinding(sb)) => {
                // Section binding contains the value directly
                if let Ok(sb_view) = sb.get_view(tree) {
                    self.values.get_eure_with_span(&sb_view.eure)
                        .map(|(value, _)| matches!(value, EureValue::Array(_)))
                        .unwrap_or(false)
                } else {
                    false
                }
            }
            _ => {
                // Check if this looks like an array section based on naming
                // Common patterns: users, items, roles, elements, etc.
                if let Some(key_handles) = self.values.get_keys(&view.keys) {
                    for key_handle in key_handles {
                        if let Some((PathSegment::Ident(id), _)) = self.values.get_key_with_span(key_handle) {
                            let name = id.as_ref();
                            // Common plural names that often contain arrays
                            if name.ends_with("s") || name.ends_with("es") || 
                               name == "data" || name == "children" {
                                eprintln!("Section '{name}' looks like it might contain array");
                                return true;
                            }
                        }
                    }
                }
                false
            }
        }
    }
    
    fn check_cursor_position_directly(&mut self) {
        // Get the line at cursor
        let lines: Vec<&str> = self.input.lines().collect();
        if self.position.line as usize >= lines.len() {
            return;
        }
        
        let current_line = lines[self.position.line as usize];
        let char_pos = self.position.character.min(current_line.len() as u32) as usize;
        let line_before_cursor = &current_line[..char_pos];
        
        eprintln!("check_cursor_position_directly: line_before_cursor = {line_before_cursor:?}");
        eprintln!("current context stack: {:?}", self.context_stack);
        
        // Check if we're in a key position (after @ or on empty line)
        if line_before_cursor.trim().ends_with('@') || 
           (line_before_cursor.trim().is_empty() && current_line.trim_start().starts_with('@')) {
            let context = CompletionContext {
                path_segments: self.current_path(),
                variant_context: self.current_variant(),
                is_in_value_position: false,
                is_in_key_position: true,
                is_variant_position: false,
                is_string_only: false,
                partial_field: None,
                variant_contexts: self.variant_map.clone(),
                used_fields: vec![],
            };
            
            eprintln!("Found context on empty line: path = {:?}", context.path_segments);
            self.found_context = Some(context);
        }
    }
}

impl<'a, F: CstFacade> CstVisitor<F> for CompletionContextTracker<'a> {
    type Error = std::convert::Infallible;
    
    fn visit_section_body(
        &mut self,
        handle: SectionBodyHandle,
        view: SectionBodyView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Continue visiting first to go deeper
        let _ = self.visit_section_body_super(handle, view, tree);
        
        // Then check if cursor is within this section body and we haven't found context yet
        if self.found_context.is_none()
            && let Some(span) = get_span_from_node(handle.node_id(), tree) {
                eprintln!("visit_section_body (after children): span = {:?}, cursor = {}, current_path = {:?}", 
                         span, self.cursor_byte_offset, self.current_path());
                if self.is_cursor_in_span(span) {
                    // We're inside a section body, check for empty line
                    self.check_cursor_position_in_section();
                }
            }
        
        Ok(())
    }
    
    fn visit_section(
        &mut self,
        handle: SectionHandle,
        view: SectionView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Extract section keys
        if let Some(key_handles) = self.values.get_keys(&view.keys) {
            let mut section_keys = Vec::new();
            
            for key_handle in key_handles {
                if let Some((segment, _)) = self.values.get_key_with_span(key_handle) {
                    // Skip value segments (they're data in arrays, not schema paths)
                    if !matches!(segment, PathSegment::Value(_)) {
                        section_keys.push(segment.clone());
                    }
                }
            }
            
            // Only process if we have meaningful keys
            if !section_keys.is_empty() {
                eprintln!("visit_section (before extend): keys = {:?}, current_section_path = {:?}", 
                         section_keys, self.current_section_path);
                
                // Update current section path
                self.current_section_path.extend(section_keys.clone());
                
                eprintln!("visit_section (after extend): current_section_path = {:?}", 
                         self.current_section_path);
                
                // Track used fields
                if let Some(PathSegment::Ident(id)) = section_keys.first() {
                    let field_name = id.as_ref().to_string();
                    // Use parent path for tracking (without current section)
                    let parent_path = self.current_section_path[..self.current_section_path.len() - section_keys.len()].to_vec();
                    let path_key = path_to_string(&parent_path);
                    
                    self.used_fields_map
                        .entry(path_key)
                        .or_default()
                        .push(field_name);
                }
                
                // Check for variant
                let variant = match view.section_body.get_view(tree) {
                    Ok(SectionBodyView::SectionBodyList(list_handle)) => {
                        if let Ok(Some(list_view)) = list_handle.get_view(tree) {
                            if let Ok(bindings) = list_view.get_all(tree) {
                                self.extract_variant_from_bindings(&bindings, tree)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                
                // Store variant context
                if let Some(ref variant_name) = variant {
                    let path_key = path_to_string(&self.current_section_path);
                    self.variant_map.insert(path_key, variant_name.clone());
                }
                
                // Update context stack for completion checks
                self.context_stack.clear();
                self.context_stack.push(ContextFrame {
                    path: self.current_section_path.clone(),
                    variant,
                    in_array: false,
                    array_index: None,
                });
                
                // Check if this section has an array value
                let has_array_value = self.section_has_array_value(&view, tree);
                eprintln!("Section {section_keys:?} has_array_value: {has_array_value}");
                
                if has_array_value {
                    // Save current path for array elements
                    self.section_path_stack.push(self.current_section_path.clone());
                    eprintln!("Saving array section path: {:?}, stack depth now: {}", 
                             self.current_section_path, self.section_path_stack.len());
                }
                
                // Visit children
                let _ = self.visit_section_super(handle, view, tree);
                
                // Don't pop the array section path here - let the object visitor handle it
                // This ensures the path persists for all objects in the array
                
                // Restore section path
                for _ in 0..section_keys.len() {
                    self.current_section_path.pop();
                }
            } else {
                // No keys - we're likely in an array element
                // Check if we can inherit from array context
                let inherited_path = if self.current_section_path.is_empty() && !self.section_path_stack.is_empty() {
                    self.section_path_stack.last().cloned().unwrap_or_default()
                } else {
                    self.current_section_path.clone()
                };
                
                eprintln!("visit_section: no keys, inheriting path: {inherited_path:?}");
                
                // Set the inherited path temporarily
                let saved_path = self.current_section_path.clone();
                self.current_section_path = inherited_path.clone();
                
                self.context_stack.clear();
                self.context_stack.push(ContextFrame {
                    path: inherited_path,
                    variant: None,
                    in_array: false,
                    array_index: None,
                });
                
                // Visit children
                let _ = self.visit_section_super(handle, view, tree);
                
                // Restore original path
                self.current_section_path = saved_path;
            }
        } else {
            // No keys at all
            let _ = self.visit_section_super(handle, view, tree);
        }
        
        Ok(())
    }
    
    fn visit_binding(
        &mut self,
        handle: BindingHandle,
        view: BindingView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        eprintln!("visit_binding: entering, current_path = {:?}", self.current_path());
        
        // Extract the binding key to track used fields AND to extend path for array bindings
        let mut binding_key_segments = Vec::new();
        
        if let Some(key_handles) = self.values.get_keys(&view.keys) {
            for key_handle in key_handles {
                if let Some((segment, _)) = self.values.get_key_with_span(key_handle) {
                    binding_key_segments.push(segment.clone());
                    
                    if let PathSegment::Ident(id) = &segment {
                        // Only track as used field if we're not going to push this as a path segment
                        // (i.e., if it's not an array binding)
                        let field_name = id.as_ref().to_string();
                        let path_key = path_to_string(&self.current_path());
                        
                        // We'll decide whether to track after checking if it's an array
                        eprintln!("Found binding key: {} at path {:?}", id.as_ref(), self.current_path());
                    }
                }
            }
        }
        
        // Check if this binding has an array value
        let has_array_value = match &view.binding_rhs.get_view(tree) {
            Ok(BindingRhsView::ValueBinding(vb)) => {
                if let Ok(vb_view) = vb.get_view(tree) {
                    let is_array = self.values.get_value(&vb_view.value)
                        .map(|v| matches!(v, EureValue::Array(_)))
                        .unwrap_or(false);
                    if is_array {
                        eprintln!("Detected array value in binding");
                    }
                    is_array
                } else {
                    false
                }
            }
            _ => false,
        };
        
        // Track used fields only if this is not an array binding
        if !has_array_value {
            for segment in &binding_key_segments {
                if let PathSegment::Ident(id) = segment {
                    let field_name = id.as_ref().to_string();
                    let path_key = path_to_string(&self.current_path());
                    
                    self.used_fields_map
                        .entry(path_key)
                        .or_default()
                        .push(field_name);
                        
                    eprintln!("Tracking used field: {} at path {:?}", id.as_ref(), self.current_path());
                }
            }
        }
        
        // If this binding has an array value, we need to push the binding key to the path
        // so that nested sections inside array elements have the correct context
        let pushed_array_frame = has_array_value && !binding_key_segments.is_empty();
        if pushed_array_frame {
            eprintln!("Found array binding, pushing path segments: {binding_key_segments:?}");
            self.push_frame(binding_key_segments, None);
        }
        
        // Check if cursor is within this binding
        if let Some(span) = get_span_from_node(handle.node_id(), tree) {
            eprintln!("visit_binding: span = {:?}, cursor_byte_offset = {}", span, self.cursor_byte_offset);
            self.check_completion_position(span);
        }
        
        // Continue visiting
        let _ = self.visit_binding_super(handle, view, tree);
        
        // Pop the frame if we pushed one for an array binding
        if pushed_array_frame {
            self.pop_frame();
            eprintln!("Popped array binding frame, path now: {:?}", self.current_path());
        }
        
        Ok(())
    }
    
    fn visit_value_binding(
        &mut self,
        handle: ValueBindingHandle,
        view: ValueBindingView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Check if this value binding contains an array
        if let Some(value) = self.values.get_value(&view.value)
            && matches!(value, EureValue::Array(_)) {
                eprintln!("Found array value binding at path: {:?}", self.current_path());
                
                // When we have a binding like `@ field = [...]`, 
                // the path includes 'field' from the binding key
                // We need to track that we're now inside an array for that field
                
                // Continue visiting to handle array elements
                let _ = self.visit_value_binding_super(handle, view, tree);
                return Ok(());
            }
        
        // For non-array values, continue normally
        let _ = self.visit_value_binding_super(handle, view, tree);
        Ok(())
    }
    
    fn visit_object(
        &mut self,
        handle: ObjectHandle,
        view: ObjectView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        eprintln!("visit_object: entering with current_section_path = {:?}, stack depth = {}", 
                 self.current_section_path, self.section_path_stack.len());
        
        // Determine if we're in an array context
        let in_array = self.current_section_path.is_empty() && !self.section_path_stack.is_empty();
        
        if in_array {
            // We're inside an array - use the full stack to build our path
            // This handles nested arrays properly
            let mut array_path = Vec::new();
            for path in &self.section_path_stack {
                // Each entry in the stack represents a nested array level
                // We want the last segment from each level
                if let Some(last_segment) = path.last() {
                    array_path.push(last_segment.clone());
                }
            }
            self.current_section_path = array_path;
            eprintln!("visit_object: built array path from stack: {:?}", self.current_section_path);
        }
        
        // Save state for restoration
        let saved_path = self.current_section_path.clone();
        
        // Continue visiting
        let _ = self.visit_object_super(handle, view, tree);
        
        // Restore the original path state
        self.current_section_path = saved_path;
        
        Ok(())
    }
}

fn get_span_from_node<F: CstFacade>(node_id: CstNodeId, tree: &F) -> Option<InputSpan> {
    if let Some(node_data) = tree.node_data(node_id) {
        match node_data {
            CstNodeData::Terminal { data: TerminalData::Input(span), .. } => Some(span),
            CstNodeData::NonTerminal { data: NonTerminalData::Input(span), .. } => Some(span),
            _ => None,
        }
    } else {
        None
    }
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

/// Convert path segments to a string key
fn path_to_string(path: &[PathSegment]) -> String {
    path.iter()
        .map(|seg| match seg {
            PathSegment::Ident(id) => id.as_ref().to_string(),
            PathSegment::Extension(ext) => format!("${}", ext.as_ref()),
            PathSegment::MetaExt(meta) => format!("$${}", meta.as_ref()),
            PathSegment::Array { index, .. } => {
                if let Some(EureValue::I64(idx)) = index {
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