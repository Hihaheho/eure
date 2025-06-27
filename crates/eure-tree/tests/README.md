# Eure Tree Constructor API Tests

This directory contains integration tests for the Eure CST constructor API.

## Test Files

### constructor_integration.rs

Comprehensive tests demonstrating the constructor API for building Concrete Syntax Trees programmatically.

#### Tests:

1. **test_constructor_complete_tree**
   - Demonstrates building a simple CST for "answer = 42"
   - Shows terminal and non-terminal constructor usage
   - Compares with parsed tree to verify correctness

2. **test_constructor_patterns**
   - Shows various patterns for using the constructor API
   - Demonstrates CommandNodeId usage
   - Explains handle creation requirements
   - Shows builder patterns for optional/repetition nodes

3. **test_constructor_complex_object**
   - Demonstrates constructor patterns without full tree building
   - Shows how complex structures would be built

4. **test_constructor_array**
   - Demonstrates array construction patterns
   - Shows list builder usage

5. **test_constructor_section**
   - Demonstrates section construction with dotted keys
   - Shows nested structure building

6. **test_constructor_all_value_types**
   - Tests all value type constructors (null, boolean, string, integer, code, hole)
   - Demonstrates terminal constructor usage for each type

## Key Insights

1. **Terminal Constructors**: Located in the `terminals` module, create CommandNodeIds
2. **Non-Terminal Constructors**: Use builder pattern, accept CommandNodeIds or Handles
3. **Alternative Constructors**: Use From trait, only work with Handle types
4. **Command Batching**: Apply commands to convert CommandNodeIds to CstNodeIds
5. **Handle Creation**: Requires proper tree structure with parent-child relationships

## Usage Patterns

### Building New Nodes
```rust
let mut commands = CstCommands::default();
let token = terminals::ident(&mut commands, "example");
let node = IdentConstructor::builder()
    .child_0(token)
    .build()
    .build_with_commands(&mut commands);
commands.apply_to(&mut tree).unwrap();
```

### Using Handles (for existing parsed trees)
```rust
let handle = SomeHandle::new(node_id, &tree)?;
let new_node = AlternativeConstructor::from(handle)
    .build_with_commands(&mut commands);
```

### Optional Nodes
```rust
let empty = OptionalConstructor::builder().build();
let with_value = OptionalConstructor::builder()
    .value(some_handle)
    .build();
```

### Repetition Nodes
```rust
let list = ListConstructor::builder()
    .add_item(item1)
    .add_item(item2)
    .build();
```

## Important Notes

- The constructor API is designed for programmatic tree manipulation
- For parsing text, use `eure_parol::parse()` instead
- Handle creation validates tree structure and may fail if nodes don't match expected patterns
- Mixing CommandNodeIds and Handles requires careful design and command batching