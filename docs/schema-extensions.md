# Schema Extensions

EURE Schema uses extensions to define types and constraints directly within EURE documents. This document describes all schema-related extensions.

## Overview

EURE Schema is embedded within EURE documents using extension namespaces (prefixed with `$`). The schema system is self-hosted, meaning the schema language is defined using itself.

### Extension Levels

- `$` - Regular extensions used in documents and schemas
- `$$` - Meta extensions used only in the meta-schema (eure-schema.schema.eure) to define what extensions are available

## Core Type Extensions

### $type

Assigns a type to a specific field.

```eure
@ user
name.$type = .string
age.$type = .number
age.$optional = true  # Fields are required by default
```

**Important:** Do not confuse schema definitions with data bindings:
- `name.$type = .string` - Correct: defines the type of the field
- `name = .string` - Wrong: assigns the path value `.string` as data

For more about extensions as metadata, see [extensions.md](./extensions.md#extensions-are-not-data).

### $types

Namespace for defining custom types.

```eure
@ $types.username
$type = .string
$length = [3, 20]  # Min and max length
$pattern = regex"^[a-z0-9_]+$"

# Using the custom type
@ user
username.$type = .$types.username
```

### $union

Defines a union type (untagged) that accepts multiple types.

```eure
@ $types.id
$union = [.string, .number]  # Accepts either string or number

@ $types.config-value
$union = [.string, .number, .boolean, .array]
```

### $variants

Defines algebraic data types (tagged unions). The `$variant` field is always used as the discriminator.

```eure
$types.action {
  @ $variants.set-text {
    text.$type = .string
    speaker.$type = .string
  }

  @ $variants.set-choices {
    prompt.$type = .string
    choices.$array = .$types.choice  # Array of choice type
  }
}

# Optionally specify variant representation
$types.response {
  $variant-repr = "untagged"  # No discriminator field
  @ $variants.success {
    data.$type = .any
  }
  @ $variants.error {
    message.$type = .string
  }
}
```

### $array

Indicates that a field is an array type. Note: Use `$array`, not `$type = .array`.

```eure
@ config
features.$array = .$types.feature  # Array of feature type

@ user
tags.$array = .string  # Array of strings
tags.$unique = true    # No duplicate values
tags.$min-items = 1
tags.$max-items = 10
```

**Important:**
- `items.$array = .string` - Correct: defines an array of strings
- `items.$type = .array` - Wrong: invalid syntax for arrays

### $cascade-type

Applies a type to all descendant fields (excluding extensions). Can be combined with other modifiers.

```eure
@ config
$cascade-type = .string  # All nested fields will be strings
$optional = true         # The cascade-type itself is optional

@ server.host    # Automatically .string
@ server.port    # Automatically .string
```

### $optional

Marks a field as optional. Fields are required by default.

```eure
@ user
email.$type = .code.email

bio.$type = .string
bio.$optional = true  # This field can be omitted
```

## Type Constraints

### String Constraints

```eure
$types.username {
  $type = .string
  $length = [3, 20]         # Array with [min, max]
  $pattern = regex"^[a-z]+" # Regex pattern
  $format = "email"         # Format validation
}
```

### Number Constraints

```eure
$types.age {
  $type = .number
  $range = [0, 150]    # Inclusive range
  $minimum = 0         # Alternative: just minimum
  $maximum = 150       # Alternative: just maximum
  $exclusive-min = 0   # Exclusive minimum
  $exclusive-max = 150 # Exclusive maximum
}
```

### Array Constraints

```eure
@ config
tags.$array = .string
tags.$unique = true       # No duplicates
tags.$min-items = 1
tags.$max-items = 10
tags.$contains = "required-tag"  # Must contain this value
```

## Primitive Types

All primitive types are accessed via path syntax:

- `.string` - String values
- `.number` - Numeric values (integer or float)
- `.boolean` - true/false
- `.null` - Null value
- `.any` - Any valid EURE value
- `.path` - EURE path syntax
- `.array` - Array type (usually with item type)
- `.object` - Object type

### Typed Strings

```eure
.code.email     # Email addresses
.code.url       # URLs
.code.uuid      # UUIDs
.code.date      # Date strings
.code.datetime  # DateTime strings
.code.regex     # Regular expressions
.code.semver    # Semantic versions
```

### Code Types

```eure
.code.javascript  # JavaScript code blocks
.code.rust        # Rust code blocks
.code.sql         # SQL queries
# etc.
```

## Structural Preferences

### $prefer.section

Hints that a field should be expressed as a @ section.

```eure
@ user.profile
$prefer.section = true  # Suggest: @ user.profile section
$type = .object
```

### $variant-repr

Specifies how variants should be represented (defined on the variant type itself).

```eure
@ $types.action
@ $variants { ... }
$variant-repr = "untagged"  # No $variant field

# Or with custom tag
@ $types.message
@ $variants { ... }
$variant-repr = { tag = "type" }  # Use "type" instead of "$variant"

# Or with separate content
@ $types.event
@ $variants { ... }
$variant-repr = { tag = "kind", content = "data" }
```

Representation styles:
- `"untagged"` - No discriminator field
- `{ tag = "..." }` - Custom tag name (internally tagged)
- `{ tag = "...", content = "..." }` - Separate tag and content (adjacently tagged)

## Type Expressions

Types can be expressed as literals or references:

```eure
@ $types.status
# Literal type - only these exact values allowed
@ $union[]
$literal = "active"

@ $union[]
$literal = "inactive"

# Usage
@ user.status
$type = .$types.status  # Only "active" or "inactive"
```

## Serialization Extensions (Serde)

### $serde.rename

Rename a field when serializing/deserializing.

```eure
@ user_name
$type = .string
$serde.rename = "userName"  # Different name in serialized form
```

### $serde.rename-all

Apply naming convention to all fields.

```eure
@ config
$serde.rename-all = "camelCase"  # Convert all to camelCase

# Available conventions:
# - "camelCase"    user_name → userName
# - "snake_case"   userName → user_name
# - "kebab-case"   user_name → user-name
# - "PascalCase"   user_name → UserName
# - "lowercase"    UserName → username
# - "UPPERCASE"    userName → USERNAME
```

## JSON Schema Interoperability

### $json-schema

Embed or reference a JSON Schema for compatibility. Can be provided as a plain object, or as JSON/YAML code blocks.

````eure
@ user
$json-schema = {
  "type": "object",
  "properties": {
    "name": { "type": "string" },
    "age": { "type": "number" }
  }
}

# Or as a code block
@ product
$json-schema = json```
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "id": { "type": "string", "format": "uuid" },
    "price": { "type": "number", "minimum": 0 }
  },
  "required": ["id", "price"]
}
```
````

## Complete Example

```eure
# Define a custom type
$types.log-level {
  $union = ["debug", "info", "warn", "error"]
}

# Define the main schema
@ config
name.$type = .string
port.$type = .number
port.$range = [1, 65535]

logging.level.$type = .$types.log-level

features.$array = {
  id.$type = .string
  enabled.$type = .boolean
  enabled.$optional = true
}
```

This example shows:
- Custom type definition with union
- Basic field types and constraints
- Nested structure
- Array with inline type definition

## Common Mistakes

### The #1 Mistake: Confusing Schema with Data

```eure
# WRONG - Creates data, not schema
name = .string          # Sets field 'name' to the path value '.string'

# CORRECT - Defines schema  
name.$type = .string    # Defines that 'name' must be a string
```

Remember:
- Use `.$type` for type definitions
- Use `.$array` for arrays (not `.$type = .array`)
- Keep schemas separate from data files

## Meta-Schema

The EURE Schema system is self-hosted using meta-extensions (`$$` prefix) to define what extensions are available. For details about meta-extensions and how they work, see [extensions.md](./extensions.md#meta-extensions).

The complete meta-schema can be found in `eure-schema.schema.eure`.
