# EURE Schema Format Documentation

This document describes the EURE schema format for defining and validating EURE documents.

## Table of Contents
- [Overview](#overview)
- [Basic Schema Definitions](#basic-schema-definitions)
- [Type System](#type-system)
- [Constraints](#constraints)
- [Meta-Extensions](#meta-extensions)
- [Advanced Features](#advanced-features)
- [Examples](#examples)

## Overview

EURE schemas define the structure, types, and constraints for EURE documents. Schemas can be:
- **External**: Stored in separate `.schema.eure` files
- **Inline**: Embedded directly in documents using schema extensions
- **Mixed**: Combination of external schemas with inline refinements

### Schema Reference

Documents can reference external schemas using the `$schema` key:

```eure
$schema = "./my-schema.eure"

# Document content follows...
```

## Basic Schema Definitions

### Field Type Definitions

Define field types using the `$type` extension:

```eure
# Simple field type
name.$type = .string

# Nested object field
person.email.$type = .code.email
```

### Type Definitions Section

Define reusable types in the `$types` section:

```eure
@ $types.Address
$type = .object
@ $types.Address.street
$type = .string
@ $types.Address.city
$type = .string
@ $types.Address.zipCode
$type = .string
$pattern = "^[0-9]{5}$"
```

### Using Type References

Reference defined types with the `.$types.TypeName` syntax:

```eure
@ person
$type = .object
@ person.homeAddress
$type = .$types.Address
@ person.workAddress
$type = .$types.Address
```

## Type System

### Basic Types

- `.string` - Text values
- `.number` - Numeric values (integers and floats)
- `.boolean` - True/false values
- `.null` - Null value
- `.path` - Path expressions (e.g., `.some.path`)
- `.object` - Object/section containers
- `.array` - Array containers
- `.any` - Any value type

### Typed Strings

Typed strings provide semantic meaning:

```eure
email.$type = .code.email
url.$type = .code.url
uuid.$type = .code.uuid
date.$type = .code.date
time.$type = .code.time
datetime.$type = .code.datetime
duration.$type = .code.duration
semver.$type = .code.semver
regex.$type = .code.regex
```

### Code Types

For code blocks with language specification:

```eure
script.$type = .code.javascript
config.$type = .code.yaml
query.$type = .code.sql
```

### Union Types

Define fields that accept multiple types:

```eure
@ $types.StringOrNumber
$union = [.string, .number]

value.$type = .$types.StringOrNumber
```

### Array Types

Arrays with element type specification:

```eure
# Using $array meta-extension
tags.$array = .string

# Array of objects
items.$array = .$types.Item
```

## Constraints

### String Constraints

```eure
username.$type = .string
username.$length = [3, 20]      # Min and max length
username.$pattern = "^[a-zA-Z0-9_]+$"  # Regex pattern
```

### Number Constraints

```eure
age.$type = .number
age.$range = [0, 150]           # Min and max values
age.$exclusive-min = true       # Exclude minimum
age.$exclusive-max = false      # Include maximum
```

### Array Constraints

```eure
tags.$type = .array
tags.$min-items = 1             # Minimum items
tags.$max-items = 10            # Maximum items
tags.$unique = true             # All items must be unique
tags.$contains = .string        # Array must contain this type
```

## Meta-Extensions

Meta-extensions use the `$$` prefix and define schema metadata:

### Optional Fields

```eure
@ $types.User
name = .string                  # Required by default
email = .code.email
email.$$optional = true         # Make field optional
```

### Preferences

```eure
# Prefer section vs binding syntax
config.$$prefer.section = true  # Prefer @ config sections
cache.$$prefer.section = false  # Prefer cache.key = value

# Array syntax preference
items.$$prefer.array = true     # Prefer explicit array syntax
```

### Serialization Options

```eure
# Field renaming
statusCode.$$serde.rename = "status_code"

# Type-level rename rules
@ $types.ApiResponse
$$serde.rename-all = "camelCase"
```

### Cascade Type

Set default type for all undefined fields:

```eure
# Global cascade type
$$cascade-type = .string

# All undefined fields accept strings
anyField = "value"
```

## Advanced Features

### Variant Types

Define tagged union types with variants:

```eure
@ $types.Action
$$variant-repr = { tag = "type" }   # Internally tagged

@ $$variants.create
name = .string
timestamp = .number

@ $$variants.update
id = .string
changes = .object

@ $$variants.delete
id = .string
reason = .string
reason.$$optional = true
```

### Variant Representations

```eure
# Untagged variants (inferred from structure)
$$variant-repr = "untagged"

# Internally tagged (tag is a field)
$$variant-repr = { tag = "type" }

# Adjacently tagged (separate tag and content)
$$variant-repr = { tag = "t", content = "c" }
```

### Deep Nesting Support

Define schemas for deeply nested structures:

```eure
# Three+ levels of nesting
company.department.manager.$type = .string
company.department.budget.$type = .number
company.department.budget.$range = [0, 1000000]

# With meta-extensions
api.v1.endpoints.users.rateLimit.$type = .number
api.v1.endpoints.users.rateLimit.$$optional = true
```

### Convention-Based Schema Discovery

EURE automatically discovers schemas using naming conventions:
- `document.eure` → looks for `document.schema.eure`
- Searches in the same directory and parent directories

## Examples

### Complete Schema Example

```eure
# Schema for a configuration file
$schema = "./eure-schema.schema.eure"

# Global settings
$$cascade-type = .any
$$serde.rename-all = "snake_case"

# Type definitions
@ $types.ServerConfig
listen = .string
listen.$pattern = "^[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}:[0-9]+$"
workers = .number
workers.$range = [1, 100]

@ $types.DatabaseConfig
host = .string
port = .number
port.$range = [1, 65535]
username = .string
password = .string
password.$$optional = true

# Root schema
@ config
$type = .object
@ config.server
$type = .$types.ServerConfig
@ config.database
$type = .$types.DatabaseConfig
```

### Self-Describing Document

```eure
# Inline schema with data
name.$type = .string
age.$type = .number
age.$range = [0, 150]

# Actual data
name = "Alice"
age = 30
```

### Using Variants

```eure
@ $types.Event
$$variant-repr = { tag = "event_type" }

@ $$variants.user_created
username = .string
username.$length = [3, 20]
email = .code.email
timestamp = .number

@ $$variants.user_deleted
user_id = .string
reason = .string
reason.$$optional = true
timestamp = .number

# Document using the variant
@ events[]
$type = .$types.Event
$variant = "user_created"
username = "alice123"
email = "alice@example.com"
timestamp = 1234567890
```

## Best Practices

1. **Use Type Definitions**: Define reusable types in `$types` section
2. **Be Explicit**: Specify types for all fields in schemas
3. **Add Constraints**: Use constraints to catch errors early
4. **Document Types**: Add comments explaining complex types
5. **Version Schemas**: Include version information in schema files
6. **Test Schemas**: Validate example documents against schemas

## Schema Validation

The EURE schema validator provides:
- Type checking with detailed error messages
- Constraint validation (length, range, pattern, etc.)
- Required field checking
- Unknown field detection
- Preference warnings (section vs binding syntax)
- Variant validation

Error messages include:
- Field path location
- Expected vs actual types
- Constraint details
- Suggestions for fixes
