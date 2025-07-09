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

## Meta-Extensions

Meta-extensions (using `$$` syntax) define schemas and constraints for regular extensions:

```eure
# Define that $optional extension must be a boolean
$$optional = .boolean

# Define structure for $validation extension
$$validation {
    min = .number
    max = .number
    pattern = .string
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

Meta-extensions use `DocumentKey::MetaExtension` in the document model and help validate extension usage.

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

## Important Concepts

### Extensions Are Not Data

Extensions are metadata stored separately from data:
- They don't appear in serialized output (JSON, YAML, etc.) unless explicitly handled
- They don't affect data validation
- They provide additional context without modifying the data structure

### No String Manipulation

The `$` and `$$` symbols are part of the EURE grammar, not string prefixes:
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
4. **Type safety**: Extensions have their own type system via meta-extensions