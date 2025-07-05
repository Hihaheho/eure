# Completion with Error Recovery Approach

## Problem Statement
We need to support completion for:
- Trailing dots: `@ script.` (complete after dot)
- Empty values: `key = ` (complete value)
- Partial keys: `@ scr` (complete key)

While maintaining LL(1) property of the grammar.

## Solution: Error Recovery Based Completion

Instead of modifying the grammar to accept incomplete constructs (which breaks LL(1)), we use error-tolerant parsing with a two-phase analysis approach.

### Two-Phase Approach

The key insight is that we need BOTH error information AND the partial CST:

1. **Phase 1: Error Analysis** (Syntactic Context)
   - What tokens were expected at the error location
   - What production rule was being parsed
   - Exact error position

2. **Phase 2: CST Traversal** (Semantic Context)
   - Current path in the document (e.g., `["script", "commands"]`)
   - Whether we're in a section, binding, or value context
   - Actual identifiers and values parsed so far

### Why String-Based Parsing is Wrong

**NEVER use string-based parsing**. It is fundamentally flawed because:
- Cannot handle nested structures correctly
- Breaks with whitespace and formatting variations
- Cannot distinguish between similar constructs
- Loses all semantic information from the parse

**ALWAYS use CST traversal** because:
- Preserves exact parse structure
- Handles all valid syntax variations
- Provides semantic context
- Works with partial/invalid syntax

### Detailed Implementation

```rust
use parol_runtime::{ParolError, SyntaxError, Location};
use crate::cst::{Cst, CstVisitor};

pub struct ErrorBasedCompletion {
    parse_result: ParseResult,
    cursor_position: Position,
}

impl ErrorBasedCompletion {
    pub fn analyze_completion_context(&self) -> CompletionContext {
        let cst = self.parse_result.cst();
        
        if let Some(error) = self.parse_result.error() {
            // Check if error is at cursor position
            if self.is_at_cursor(&error.error_location) {
                // Phase 1: Analyze error for syntactic context
                let error_context = self.analyze_error(&error);
                
                // Phase 2: Traverse CST for semantic context
                let path_context = self.extract_path_from_cst(cst, error.error_location);
                
                // Combine both contexts for rich completion
                return self.combine_contexts(error_context, path_context);
            }
        }
        
        // No error at cursor - use pure CST analysis
        self.analyze_cst_only(cst)
    }
    
    fn analyze_error(&self, error: &SyntaxError) -> ErrorContext {
        // Extract what parser expected
        let expected_tokens = &error.expected_tokens;
        let production = &error.cause; // Contains production info
        
        // Determine syntactic expectation
        if production.contains("ValueBinding") && expected_tokens.contains("Value") {
            ErrorContext::ExpectingValue {
                allowed_types: Self::tokens_to_value_types(expected_tokens),
            }
        } else if (production.contains("Keys") || production.contains("KeysList")) 
                  && expected_tokens.contains("Ident") {
            ErrorContext::ExpectingKey
        } else {
            ErrorContext::Unknown
        }
    }
    
    fn extract_path_from_cst(&self, cst: &Cst, error_loc: Location) -> PathContext {
        // CST visitor that builds path up to error location
        let mut visitor = PathExtractorVisitor {
            target_location: error_loc,
            current_path: vec![],
            found_path: None,
            in_section: false,
            in_binding: false,
        };
        
        visitor.visit_cst(cst);
        
        PathContext {
            path: visitor.found_path.unwrap_or_default(),
            is_in_section: visitor.in_section,
            is_in_binding: visitor.in_binding,
            last_complete_node: visitor.last_complete_node,
        }
    }
    
    fn combine_contexts(&self, error: ErrorContext, path: PathContext) -> CompletionContext {
        match error {
            ErrorContext::ExpectingValue { allowed_types } => {
                CompletionContext::Value {
                    binding_path: path.path,
                    allowed_types,
                    is_section_value: path.is_in_section && !path.is_in_binding,
                }
            }
            ErrorContext::ExpectingKey => {
                CompletionContext::Key {
                    parent_path: path.path,
                    is_section_key: !path.is_in_binding,
                }
            }
            ErrorContext::Unknown => {
                // Fallback to CST-only analysis
                self.analyze_cst_only_with_path(path)
            }
        }
    }
}

struct PathExtractorVisitor {
    target_location: Location,
    current_path: Vec<String>,
    found_path: Option<Vec<String>>,
    in_section: bool,
    in_binding: bool,
    last_complete_node: Option<NodeInfo>,
}

impl CstVisitor for PathExtractorVisitor {
    fn visit_section(&mut self, section: &Section) {
        self.in_section = true;
        // Extract path from section keys
        let keys_path = self.extract_keys_path(&section.keys);
        self.current_path = keys_path;
        
        // Check if we've reached the error location
        if self.is_at_or_past_error(&section.span) {
            self.found_path = Some(self.current_path.clone());
        }
        
        // Continue traversal
        self.visit_section_body(&section.body);
    }
    
    fn visit_binding(&mut self, binding: &Binding) {
        self.in_binding = true;
        let binding_key = self.extract_keys_path(&binding.keys);
        
        // Check if error is in this binding
        if self.is_at_or_past_error(&binding.span) {
            self.found_path = Some(self.current_path.clone());
            self.last_complete_node = Some(NodeInfo::Binding(binding_key));
        }
    }
    
    // ... other visitor methods
}
```

### Concrete Examples

#### Example 1: Trailing Dot
Input: `@ script.`

Parser Error:
```
LA(1): $ (EndOfInput) at test.eure:1:9-1:10
at non-terminal "Keys"
Expected tokens: ["Ident", "Str", "Integer", "True", "False", "Null", "Ext", "MetaExt"]
```

CST Analysis:
- Path extracted: `["script"]`
- Context: In section keys after dot

Combined Result:
- CompletionContext::Key { parent_path: ["script"], is_section_key: true }
- Suggest: Keys that can follow `script.`

#### Example 2: Empty Value
Input: `key = `

Parser Error:
```
LA(1): $ (EndOfInput) at test.eure:1:6-1:7
at non-terminal "Value"
Current production: ValueBinding: Bind Value;
Expected tokens: ["Integer", "True", "False", "Null", "Hole", "Str", "CodeBlock", "NamedCode", "Code", "Dot", "Begin", "ArrayBegin", "LParen"]
```

CST Analysis:
- Binding key: `["key"]`
- Context: In binding after `=`

Combined Result:
- CompletionContext::Value { binding_path: ["key"], allowed_types: [String, Integer, Boolean, ...] }
- Suggest: Value completions for the binding

#### Example 3: Nested Context
Input: `@ a.b.c.`

Parser Error:
```
LA(1): $ (EndOfInput) at test.eure:1:8-1:9
at non-terminal "KeysList"
Expected tokens: ["Ident", ...]
```

CST Analysis:
- Path extracted: `["a", "b", "c"]`
- Context: In nested section keys

Combined Result:
- CompletionContext::Key { parent_path: ["a", "b", "c"], is_section_key: true }
- Suggest: Keys under `a.b.c`

### Key Implementation Points

1. **Never parse strings** - Always traverse the CST
2. **Use error location precisely** - Match byte offsets, not line/column
3. **Build context incrementally** - Track path as visitor traverses
4. **Handle partial trees** - CST may be incomplete but still useful
5. **Combine both phases** - Neither error nor CST alone is sufficient

### Advantages

1. **Grammar remains LL(1)** - No ambiguity introduced
2. **Robust completion** - Works with syntax errors
3. **Rich context** - Both syntactic and semantic information
4. **Type-aware** - Can suggest appropriate value types
5. **Path-aware** - Knows exact location in document hierarchy

### Next Steps

1. Implement PathExtractorVisitor with proper CST traversal
2. Create error-to-completion-context mapping
3. Add tests for all error scenarios
4. Integrate with LSP completion handler