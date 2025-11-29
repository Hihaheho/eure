# Extensions

Extensions provide metadata and additional information about data without being part of the data itself. The `$` syntax indicates an extension namespace in the EURE language.

## Understanding Extensions

### Syntax vs Storage

Extensions use `$` as a syntactic element in the EURE grammar, not as a string prefix:

- `$variant` is recognized by the parser as an extension syntax
- The `$` is a grammar token, similar to `{` or `=`
- Extensions are stored separately from data in the document structure

### Internal Representation

When EURE documents are parsed:

1. **At CST Level**: `$variant` becomes `PathSegment::Extension(Identifier("variant"))`
2. **In EureDocument**: Extensions are stored in `Node.extensions: HashMap<Identifier, NodeId>`
3. **Not in Values**: Extensions are metadata and don't appear in the value representation

This separation ensures extensions don't interfere with data serialization or deserialization.

## Extension Namespaces

For third-party extensions, use proper namespacing to avoid conflicts:

```eure
# Good: Properly namespaced extensions
key1.$my-extension.is-awesome = true
key2.$my-extension.is-awesome = false

# Bad: Unnamespaced custom extensions
key1.$is-awesome = true  # Could conflict with other extensions
```

## Nested Extensions

Extensions can contain structured metadata:

```eure
$my-extension {
  schema = "https://example.com/my-extension/v1"
  version = "1.0"
}
```

Note: Extensions within extensions don't use the `$` syntax again.

## Extension Type Definitions

Extension types can be defined using the `$ext-type` namespace to provide type information and validation for extensions:

```eure
# Define that $optional extension must be a boolean
$ext-type.optional = .boolean

# Define structure for $validation extension
$ext-type.validation {
  min = .number
  max = .number
  pattern = .text
}

# Now these extensions follow the defined schemas
field {
  $optional = true
  $validation {
    min = 0
    max = 100
  }
}
```

The `$ext-type` namespace is used in schema files to define the expected types for extensions.

## Standard Extensions

### $eure

The `$eure` extension provides document-level metadata:

```eure
@ $eure
version: https://eure.dev/v1
schema: https://eure.dev/schemas/my-schema/v1
```

### $variant

Indicates variant selection in sum types:

```eure
@ user {
  $variant = "premium"
  subscription_level = "gold"
  benefits = ["priority-support", "advanced-features"]
}
```

The `$variant` extension is metadata that helps identify which variant of a sum type is being used. It doesn't affect the data structure itself.

#### Nested Variants

For nested variant types (similar to `Result<Result<T, E>, E>` in Rust), you can use dot notation to specify the variant path:

```eure
@ response {
  $variant = .ok.ok.err
  error_message = "Invalid input"
}
```

This corresponds to Rust's `Ok(Ok(Err(value)))` structure, where:
- The first `.ok` selects the outer `Ok` variant
- The second `.ok` selects the middle `Ok` variant
- The `.err` selects the inner `Err` variant

Each segment in the path represents one level of variant nesting:

```eure
# Simple variant
$variant = "success"  # or $variant = .success

# Nested variants (2 levels)
$variant = .ok.value

# Deeply nested variants (3 levels)
$variant = .ok.ok.err

# Any depth is supported
$variant = .a.b.c.d.e
```

### $local

Provides document-local metadata storage:

```eure
@ config {
  $local.last-modified = "2024-01-15"
  $local.author = "system"

  # Actual configuration data
  timeout = 30
  retries = 3
}
```

### $license

Specifies license information using SPDX License IDs for root or sub-document objects:

```eure
# Root-level license
@ $license = "MIT"

# Object-level license
@ library {
  $license = "Apache-2.0"
  name = "my-library"
  version = "1.0.0"
}

# Sub-document with different license
@ component {
  $license = "GPL-3.0"
  name = "gpl-component"
  source = "https://example.com/component"
}
```

The `$license` extension uses SPDX License Identifiers (e.g., "MIT", "Apache-2.0", "GPL-3.0") to specify the license for a document or object.

## Important Concepts

### Extensions Are Not Data

Extensions are metadata stored separately from data:
- They don't appear in serialized output (JSON, YAML, etc.) unless explicitly handled
- They don't affect data validation
- They provide additional context without modifying the data structure

### No String Manipulation

The `$` symbol is part of the EURE grammar, not a string prefix:
- Never use string operations like `starts_with("$")` or `strip_prefix("$")`
- The parser recognizes extension syntax at the grammatical level
- Extension names are stored as identifiers, not strings with prefixes

### Quoted Dollar Signs

To use a literal `$` in a key name (not as an extension), quote it:

```eure
# This is an extension (metadata)
$variant = "type-a"

# This is a data field with a literal $ in the name
"$price" = 99.99
```


## Design Rationale

The separation of extensions from data ensures:
1. **Clean data models**: Data remains pure without metadata pollution
2. **Forward compatibility**: New extensions can be added without breaking existing data
3. **Tool flexibility**: Different tools can use different extensions without conflicts
4. **Type safety**: Extensions can have type definitions via the `$ext-type` namespace
