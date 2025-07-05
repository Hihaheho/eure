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
# Schema for a configuration file
@ $types.log-level
@ $union[]
$literal = "debug"
@ $union[]
$literal = "info"
@ $union[]
$literal = "warn"
@ $union[]
$literal = "error"

@ $types.port
$type = .number
$range = [1, 65535]

@ $types.host
$union = [.string, .code.url]  # String or URL

# Feature type using inline style
$types.feature {
  id.$type = .string
  id.$pattern = regex"^[a-z-]+$"

  enabled.$type = .boolean

  config.$type = .object
  config.$optional = true
}

# Document schema with serde hints
@ app
$serde.rename-all = "camelCase"

name.$type = .string
version.$type = .code.semver

@ server
$prefer.section = true
host.$type = .$types.host
port.$type = .$types.port
port.$serde.rename = "serverPort"  # Override rename-all

@ logging
level.$type = .$types.log-level

features.$array = .$types.feature
```

## Common Mistakes and Best Practices

### Schema Definition vs Data Binding

The most common mistake is confusing schema definitions with data bindings:

```eure
# WRONG - This creates data bindings, not schema definitions
@ person
name = .string          # Assigns the path value `.string` to field `name`
age = .number          # Assigns the path value `.number` to field `age`
email = .code.email

# CORRECT - Use $type extension for schema definitions
@ person
name.$type = .string
age.$type = .number
email.$type = .code.email
```

When you write `name = .string`, you're creating a data entry where the field `name` contains the path value `.string`. This is valid EURE but not what you want for schema definitions.

### Arrays

```eure
# WRONG
items.$type = .array    # Invalid - there's no .array type

# CORRECT
items.$array = .string  # Array of strings
```

### Mixed Schema and Data

Be careful not to mix schema definitions with data in the same document:

```eure
# PROBLEMATIC - Mixing schema and data
@ config
debug = true                    # Data
debug.$type = .boolean         # Schema - but debug already has data!

# BETTER - Separate schema from data
# schema.eure
@ config
debug.$type = .boolean

# config.eure
$schema = "schema.eure"
@ config
debug = true
```

## Meta-Schema Insights

The meta-schema (eure-schema.schema.eure) reveals the extension hierarchy:

```eure
# Global extension definitions
$$optional = .boolean
$$optional.$optional = true  # optional is itself optional

$$prefer {
  section = .boolean
  section.$optional = true
  $optional = true  # The entire prefer block is optional
}

# Serde extensions
$$serde.rename = .string
$$serde.rename-all.$union[].$literal = "camelCase"
# ... other naming conventions

# Type definitions with meta-extensions
@ $union[].$$union.$array = .$types.type  # Union as array of types
@ $union[].$$cascade-type = .$types.type
```

This self-hosting design ensures all extensions are formally defined and validated.
