# EURE Completion System Architecture

## Overview

This document describes a complete redesign of the EURE LSP completion system to use proper CST (Concrete Syntax Tree) traversal instead of string-based parsing.

## Current Problems

1. **String-based parsing** - Re-parsing already parsed content
2. **Two-phase extraction** - First extracting values, then context
3. **Incomplete context** - Missing array indices, variant contexts
4. **Fragile logic** - Many edge cases with line-by-line parsing
5. **Poor error recovery** - Doesn't handle incomplete syntax well

## Design Goals

1. **Single-pass CST traversal** - One visitor extracts all context
2. **Position-aware** - Accurately map LSP position to CST nodes
3. **Error-tolerant** - Handle incomplete/invalid syntax gracefully
4. **Type-aware** - Track expected types for value completions
5. **Efficient** - Minimal allocations and traversals

## Architecture

### Core Components

#### 1. Position Mapping

```rust
/// Converts LSP Position (line/column) to byte offset in text
struct PositionMapper {
    line_starts: Vec<usize>, // Byte offset of each line start
}

impl PositionMapper {
    fn new(text: &str) -> Self {
        let mut line_starts = vec![0];
        for (i, ch) in text.char_indices() {
            if ch == '\n' {
                line_starts.push(i + 1);
            }
        }
        Self { line_starts }
    }
    
    fn position_to_offset(&self, pos: Position) -> Option<usize> {
        let line_start = *self.line_starts.get(pos.line as usize)?;
        // Handle UTF-16 column offset (LSP uses UTF-16)
        Some(line_start + pos.character as usize)
    }
}
```

#### 2. Completion Context

```rust
#[derive(Debug)]
struct CompletionContext {
    /// Path to current location (e.g., ["script", "dependencies"])
    path: Vec<PathSegment>,
    
    /// What kind of position we're in
    position: CompletionPosition,
    
    /// Partial text being typed (e.g., "scr" in "@ scr|")
    partial: Option<String>,
    
    /// Expected type from schema (if known)
    expected_type: Option<Type>,
    
    /// Current variant context (if in variant)
    variant: Option<String>,
    
    /// Whether we're in an array element
    in_array: bool,
}

#[derive(Debug)]
enum CompletionPosition {
    /// After @ at start of line
    SectionStart,
    
    /// In section key position (e.g., "@ script.|")
    SectionKey { prefix: Vec<String> },
    
    /// In binding key position
    BindingKey,
    
    /// After = (any value)
    ValueAny,
    
    /// After : (string value only)
    ValueString,
    
    /// After . in value (e.g., "type = .|")
    ValueDot,
    
    /// After $variant: or $variant=
    VariantName,
    
    /// Unknown/invalid position
    Unknown,
}
```

#### 3. Completion Visitor

```rust
// CompletionVisitor is specialized for finding completion contexts
// For full value extraction, use the new ValueVisitor API:
//   let mut visitor = ValueVisitor::new(input);
//   visitor.visit_eure(handle, view, tree)?;
//   let document = visitor.into_document();

struct CompletionVisitor<'a> {
    input: &'a str,
    cursor_offset: usize,
    
    // Current state as we traverse
    path_stack: Vec<PathSegment>,
    variant_stack: Vec<Option<String>>,
    
    // Result
    context: Option<CompletionContext>,
}

// Note: This implements CstVisitor for completion-specific traversal
// The new ValueVisitor uses similar path building with build_path_segments()
impl<'a> CstVisitor for CompletionVisitor<'a> {
    fn visit_section(&mut self, handle: SectionHandle, view: SectionView, tree: &Cst) -> Result<(), Self::Error> {
        let span = self.get_span(handle, tree);
        
        // Check if cursor is in section header
        if self.cursor_in_span(span) {
            // Analyze section header for completion position
            if let Some(keys_span) = self.get_span(view.keys, tree) {
                if self.cursor_in_span(keys_span) {
                    // Cursor is in section keys
                    self.analyze_section_keys(view.keys, tree);
                    return Ok(()); // Don't traverse children
                }
            }
        }
        
        // Push section path
        let section_path = self.extract_section_path(view.keys, tree);
        self.path_stack.extend(section_path);
        
        // Check for variant
        let variant = self.extract_variant_from_body(view.section_body, tree);
        self.variant_stack.push(variant);
        
        // Continue traversal
        self.visit_section_super(handle, view, tree)?;
        
        // Pop context
        self.path_stack.truncate(original_len);
        self.variant_stack.pop();
        
        Ok(())
    }
    
    fn visit_binding(&mut self, handle: BindingHandle, view: BindingView, tree: &Cst) -> Result<(), Self::Error> {
        let span = self.get_span(handle, tree);
        
        if self.cursor_in_span(span) {
            // Analyze binding for completion position
            self.analyze_binding(view, tree);
        }
        
        Ok(())
    }
}
```

### Key Algorithms

#### 1. Finding Cursor Position

The visitor needs to efficiently determine if the cursor is within a node:

```rust
fn cursor_in_span(&self, span: Span) -> bool {
    self.cursor_offset >= span.start && self.cursor_offset <= span.end
}
```

For incomplete syntax (e.g., "@ script."), we need to be more flexible:

```rust
fn cursor_at_span_end(&self, span: Span) -> bool {
    // Allow cursor to be just after span for incomplete syntax
    self.cursor_offset >= span.end && self.cursor_offset <= span.end + 1
}
```

#### 2. Analyzing Section Keys

When cursor is in section keys like "@ script.dep|":

```rust
fn analyze_section_keys(&mut self, keys: KeysHandle, tree: &Cst) {
    // Extract the key segments before cursor
    let keys_text = self.extract_text_before_cursor(keys, tree);
    
    // Split by dots to get path segments
    let segments: Vec<_> = keys_text.split('.').collect();
    
    // Last segment might be partial
    let (complete_segments, partial) = if keys_text.ends_with('.') {
        (segments, None)
    } else {
        let partial = segments.last().map(|s| s.to_string());
        (&segments[..segments.len()-1], partial)
    };
    
    self.context = Some(CompletionContext {
        path: self.path_stack.clone(),
        position: CompletionPosition::SectionKey {
            prefix: complete_segments.to_vec(),
        },
        partial,
        expected_type: None,
        variant: self.variant_stack.last().cloned().flatten(),
        in_array: self.in_array_context(),
    });
}
```

#### 3. Analyzing Bindings

When cursor is in a binding like "host = |":

```rust
fn analyze_binding(&mut self, view: BindingView, tree: &Cst) {
    // Check if cursor is in keys
    if let Some(keys_span) = self.get_span(view.keys, tree) {
        if self.cursor_in_span(keys_span) {
            // In binding key position
            let partial = self.extract_partial_key(view.keys, tree);
            self.context = Some(CompletionContext {
                path: self.path_stack.clone(),
                position: CompletionPosition::BindingKey,
                partial,
                // ... other fields
            });
            return;
        }
    }
    
    // Check if cursor is in value
    if let Some(rhs) = view.binding_rhs {
        match rhs {
            BindingRhs::ValueBinding(vb) => {
                // After =
                self.analyze_value_binding(vb, tree);
            }
            BindingRhs::TextBinding(tb) => {
                // After :
                self.context = Some(CompletionContext {
                    position: CompletionPosition::ValueString,
                    // ...
                });
            }
        }
    }
}
```

### Challenges and Solutions

#### 1. Incomplete Syntax

**Challenge**: CST might have error nodes when syntax is incomplete.

**Solution**: 
- Handle error nodes gracefully
- Use span boundaries flexibly
- Extract text from raw input when needed

#### 2. Whitespace Positions

**Challenge**: Cursor might be in whitespace between nodes.

**Solution**:
- Check if cursor is just after a node that typically has completions
- Use "fuzzy" span matching for ends of nodes

#### 3. UTF-16 vs UTF-8

**Challenge**: LSP uses UTF-16 offsets but Rust strings are UTF-8.

**Solution**:
- Proper position mapping that handles UTF-16
- Test with unicode characters

#### 4. Performance

**Challenge**: Need fast response times for IDE experience.

**Solution**:
- Stop traversal once context is found
- Reuse CST from parsing phase
- Minimal allocations

### Completion Generation

Once we have context, generating completions is straightforward:

```rust
fn generate_completions(ctx: &CompletionContext, schema: &DocumentSchema) -> Vec<CompletionItem> {
    match &ctx.position {
        CompletionPosition::SectionStart => {
            // Suggest root-level fields from schema
            generate_field_completions(&schema.root, ctx.partial.as_deref())
        }
        
        CompletionPosition::SectionKey { prefix } => {
            // Navigate schema to prefix path, then suggest fields
            let schema_node = navigate_schema_path(&prefix, schema);
            generate_field_completions(schema_node, ctx.partial.as_deref())
        }
        
        CompletionPosition::BindingKey => {
            // Suggest fields at current path
            let schema_node = navigate_schema_path(&ctx.path, schema);
            generate_field_completions(schema_node, ctx.partial.as_deref())
        }
        
        CompletionPosition::ValueAny => {
            // Generate based on expected type
            generate_value_completions(ctx.expected_type.as_ref())
        }
        
        CompletionPosition::VariantName => {
            // Find variant type and suggest variant names
            let variant_type = find_variant_type(&ctx.path, schema);
            generate_variant_completions(variant_type)
        }
        
        // ... other cases
    }
}
```

### Testing Strategy

1. **Unit tests for position mapping**
   - UTF-8/UTF-16 edge cases
   - Multi-line documents
   - Unicode characters

2. **Visitor tests**
   - Various cursor positions
   - Incomplete syntax
   - Nested sections
   - Array contexts

3. **Integration tests**
   - Real-world schemas
   - Complex completions
   - Performance benchmarks

### Migration Plan

1. **Phase 1**: Implement new visitor alongside old code
2. **Phase 2**: Add feature flag to switch implementations
3. **Phase 3**: Migrate tests to new implementation
4. **Phase 4**: Remove old string-based code

## Conclusion

This architecture provides a solid foundation for accurate, efficient completions:

- **Single-pass** traversal minimizes overhead
- **Position-aware** visitor accurately finds cursor context
- **Type-safe** path tracking prevents errors
- **Extensible** for future completion types

The key insight is that the CST already contains all structural information - we just need to traverse it properly instead of re-parsing text.