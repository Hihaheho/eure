# EureDocument vs Value: When to Use Each

This guide helps you understand when to use `EureDocument` versus `Value` in the EURE ecosystem, their design philosophy, and practical use cases.

## Overview

### EureDocument

`EureDocument` is a tree-based representation that preserves the full structure and metadata of EURE documents, including:
- Concrete Syntax Tree (CST) handles for precise source location tracking
- Extension namespaces (`$` and `$$` prefixed fields)
- Construction method information (how values were created in the source)
- Efficient path-based insertion and traversal
- Type-safe node content with handles

**Design Philosophy**: EureDocument is designed for scenarios where you need to maintain a connection to the source text, perform incremental updates, or provide detailed error reporting with exact source locations.

### Value

`Value` is a simplified, semantic representation of EURE data that focuses on the logical content:
- Standard data types (null, bool, numbers, strings, arrays, maps, etc.)
- Special types (Code, CodeBlock, Path, Variant, Unit, Hole)
- No source location information
- Easier to work with programmatically
- Similar to JSON/YAML value representations

**Design Philosophy**: Value is designed for scenarios where you need to work with the data itself, perform transformations, or integrate with external systems that don't need source information.

## Detailed Comparison Table

| Feature | EureDocument | Value |
|---------|--------------|-------|
| **Source Location Tracking** | ✅ Full CST handles with spans | ❌ No source information |
| **Memory Efficiency** | ✅ Node-based with shared references | ❌ Recursive enum structure |
| **Path-Based Updates** | ✅ Efficient insertion and traversal | ❌ Requires full traversal |
| **Extension Namespaces** | ✅ Separate storage for `$` fields | ❌ Merged into regular maps |
| **Construction Metadata** | ✅ Preserves how values were created | ❌ Only final values |
| **Type Safety** | ✅ Typed handles for each construct | ✅ Enum variants |
| **Serialization** | ❌ Requires conversion | ✅ Direct serialization |
| **Pattern Matching** | ❌ Requires node access | ✅ Direct enum matching |
| **Schema Validation** | ✅ With span information | ✅ Without spans |
| **Editor Support** | ✅ Completions, diagnostics | ❌ Limited support |

## Use Case Examples

### Use EureDocument When:

1. **Building Language Server Protocol (LSP) Features**
   ```rust
   // Providing completions with exact cursor position
   let node = document.get_node_at_position(cursor_pos)?;
   let completions = generate_completions_for_node(node);
   ```

2. **Schema Validation with Error Reporting**
   ```rust
   // Validation errors include exact source locations
   let validator = SchemaValidator::new(input, schema, &tree);
   let errors = validator.validate(); // Errors have InputSpan
   ```

3. **Incremental Document Updates**
   ```rust
   // Efficiently update nested values
   document.insert_node(
       vec![PathSegment::Ident("config"), PathSegment::Ident("port")],
       NodeContent::I64 { handle, value: 8080 }
   )?;
   ```

4. **Source-Preserving Transformations**
   ```rust
   // Maintain formatting and comments during updates
   let node = document.get_node_mut(node_id);
   node.content = transform_preserving_handles(node.content);
   ```

### Use Value When:

1. **Data Processing and Business Logic**
   ```rust
   // Simple data extraction
   if let Value::Map(config) = &value {
       if let Some(Value::String(db_url)) = config.get("database_url") {
           connect_to_database(db_url)?;
       }
   }
   ```

2. **Serialization/Deserialization**
   ```rust
   // Convert to/from JSON, YAML, etc.
   let json = serde_json::to_string(&value)?;
   let yaml_value: Value = serde_yaml::from_str(&yaml_content)?;
   ```

3. **Data Transformation Pipelines**
   ```rust
   // Transform values without caring about source
   let processed = value
       .as_map_mut()
       .map(|m| m.retain(|k, _| !k.starts_with("debug_")));
   ```

4. **Testing and Assertions**
   ```rust
   // Easy to construct and compare
   let expected = Value::Map(Map::from([
       ("name".into(), Value::String("test".into())),
       ("count".into(), Value::I64(42)),
   ]));
   assert_eq!(actual, expected);
   ```

## Conversion Between Types

### EureDocument to Value

The conversion from `EureDocument` to `Value` is lossy - you lose source information and construction metadata:

```rust
let document: EureDocument = parse_and_build_document(input)?;
let value: Value = document.to_value();
```

**Example**: Extensions are stripped during conversion:
```eure
# Input EURE document
@ config
debug = true
debug.$description = "Enable debug mode"
debug.$since = "v1.0"
```

Results in:
```rust
// After document.to_value()
Value::Map(map!{
    "config" => Value::Map(map!{
        "debug" => Value::Bool(true)
        // Note: $description and $since are gone
    })
})
```

**Note**: The `to_value()` method:
- Skips extension namespaces entirely (extensions are metadata, not data)
- Loses all handle information
- Converts `NodeContent` variants to corresponding `Value` variants
- Recursively processes arrays and maps

### Value to EureDocument

Converting from `Value` to `EureDocument` requires synthetic handle creation:

```rust
// This is typically done during parsing, not direct conversion
// Values are built into EureDocument during the parsing phase
let tree = parse(input)?;
let mut visitor = ValueVisitor::new(input);
tree.visit_from_root(&mut visitor)?;
let document = visitor.into_document();
```

**Note**: Direct `Value` to `EureDocument` conversion is rarely needed and not recommended.

## Performance Considerations

### EureDocument Performance

**Advantages:**
- O(1) node access by ID
- Efficient path-based insertion
- Memory sharing through node references
- Lazy evaluation of complex structures

**Disadvantages:**
- Higher memory overhead per node
- Indirection through node IDs
- More complex traversal logic

### Value Performance

**Advantages:**
- Direct memory layout
- Efficient pattern matching
- Lower memory overhead for simple data
- Cache-friendly for sequential access

**Disadvantages:**
- Deep cloning for modifications
- O(n) path-based access
- Recursive memory allocation

## Code Examples

### Example 1: Schema Validation with Diagnostics

```rust
// Using EureDocument for rich error reporting
pub fn validate_with_diagnostics(input: &str) -> Vec<Diagnostic> {
    let tree = parse(input)?;
    let mut visitor = ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor)?;
    let document = visitor.into_document();
    
    let validator = SchemaValidator::new(input, schema, &tree);
    validator.validate()
        .into_iter()
        .map(|error| Diagnostic {
            span: error.span, // Exact source location
            message: error.message,
            severity: error.severity,
        })
        .collect()
}

// Using Value for simple validation
pub fn validate_simple(value: &Value) -> Result<(), String> {
    match value {
        Value::Map(map) => {
            if !map.contains_key("required_field") {
                return Err("Missing required field".into());
            }
            Ok(())
        }
        _ => Err("Expected a map".into()),
    }
}
```

### Example 2: Configuration Processing

```rust
// Using EureDocument for config with source tracking
pub struct ConfigManager {
    document: EureDocument,
    source: String,
}

impl ConfigManager {
    pub fn update_setting(&mut self, path: &[&str], value: &str) -> Result<(), Error> {
        let segments = path.iter()
            .map(|s| PathSegment::Ident(Identifier::from_str(s)?))
            .collect::<Vec<_>>();
        
        self.document.insert_node(
            segments.into_iter(),
            NodeContent::String { 
                handle: StringConstructionHandle::Synthetic,
                value: value.to_string() 
            }
        )?;
        
        // Can provide exact location where setting was updated
        Ok(())
    }
}

// Using Value for config without source tracking  
pub fn load_config(content: &str) -> Result<Config, Error> {
    let value: Value = parse_to_value(content)?;
    
    // Simple extraction without source info
    if let Value::Map(map) = value {
        Ok(Config {
            host: extract_string(&map, "host")?,
            port: extract_i64(&map, "port")? as u16,
        })
    } else {
        Err("Invalid config format".into())
    }
}
```

### Example 3: Editor Features

```rust
// Using EureDocument for completions
pub fn get_completions(document: &EureDocument, cursor: Position) -> Vec<Completion> {
    let node = document.get_node_at_position(cursor);
    let context = analyze_node_context(node);
    
    match context {
        Context::MapKey(partial) => suggest_map_keys(partial),
        Context::Value(type_hint) => suggest_values_for_type(type_hint),
        Context::Extension => suggest_extension_fields(),
    }
}

// Value cannot provide position-based features
// ❌ Not possible with Value alone
```

## Best Practices and Recommendations

### Choose EureDocument when:
1. Building development tools (LSP, linters, formatters)
2. Providing detailed error messages with source locations  
3. Implementing incremental updates or transformations
4. Preserving source formatting and structure
5. Working with extension namespaces extensively

### Choose Value when:
1. Processing configuration or data files
2. Implementing business logic
3. Serializing/deserializing to other formats
4. Writing unit tests
5. Building APIs that consume EURE data

### Hybrid Approach

In many applications, you'll use both:

```rust
pub struct EureProcessor {
    // Keep document for error reporting
    document: EureDocument,
    // Cache value for fast access
    value: Value,
}

impl EureProcessor {
    pub fn new(input: &str) -> Result<Self, Error> {
        let tree = parse(input)?;
        let mut visitor = ValueVisitor::new(input);
        tree.visit_from_root(&mut visitor)?;
        let document = visitor.into_document();
        let value = document.to_value();
        
        Ok(Self { document, value })
    }
    
    pub fn get_with_span(&self, path: &[&str]) -> Option<(&Value, InputSpan)> {
        // Use value for data, document for spans
        let value = self.get_value_at_path(&self.value, path)?;
        let span = self.get_span_for_path(&self.document, path)?;
        Some((value, span))
    }
}
```

## Summary

- **EureDocument**: Use for tooling, error reporting, and source-aware operations
- **Value**: Use for data processing, serialization, and business logic
- **Conversion**: Prefer one-way (EureDocument → Value) when needed
- **Performance**: EureDocument for updates, Value for read-heavy operations
- **Best Practice**: Choose based on whether you need source information

The key decision factor is whether you need to maintain a connection to the source text. If yes, use EureDocument. If you only care about the data itself, use Value.