# EURE Schema Quick Reference

## Basic Types
```eure
field.$type = .string          # Text
field.$type = .number          # Numbers
field.$type = .boolean         # true/false
field.$type = .null            # null
field.$type = .path            # Path like .some.path
field.$type = .object          # Object container
field.$type = .array           # Array container
field.$type = .any             # Any type
```

## Typed Strings
```eure
email.$type = .code.email
url.$type = .code.url
date.$type = .code.date
uuid.$type = .code.uuid
```

## Type Definitions
```eure
# Define a type
@ $types.Person
name = .string
age = .number
age.$$optional = true

# Use the type
user.$type = .$types.Person
```

## Constraints
```eure
# String constraints
name.$length = [3, 50]
name.$pattern = "^[A-Za-z]+$"

# Number constraints
age.$range = [0, 150]
price.$exclusive-min = true

# Array constraints
items.$min-items = 1
items.$max-items = 10
items.$unique = true
```

## Meta-Extensions ($$)
```eure
# Optional fields
field.$$optional = true

# Preferences
config.$$prefer.section = true
items.$$prefer.array = false

# Serialization
field.$$serde.rename = "fieldName"
$$serde.rename-all = "camelCase"

# Arrays
tags.$$array = .string

# Default type for undefined fields
$$cascade-type = .string
```

## Variants (Tagged Unions)
```eure
@ $types.Action
$$variant-repr = { tag = "type" }

@ $$variants.create
name = .string

@ $$variants.delete
id = .number

# Usage
action.$type = .$types.Action
action.$variant = "create"
```

## Schema Reference
```eure
# Reference external schema
$schema = "./my-schema.eure"

# Inline schema (same file)
field.$type = .string
field = "value"
```

## Deep Nesting
```eure
# Nested paths
company.dept.manager.name.$type = .string
api.v1.endpoints.users.$type = .object
```

## Common Patterns

### Required vs Optional
```eure
# Required (default)
name.$type = .string

# Optional
email.$type = .string
email.$$optional = true
```

### Array of Objects
```eure
@ $types.Item
id = .number
name = .string

items.$type = .array
items.$$array = .$types.Item
```

### Union Types
```eure
@ $types.StringOrNumber
$union = [.string, .number]

value.$type = .$types.StringOrNumber
```

### Nested Objects
```eure
@ person
$type = .object

@ person.address
$type = .object
street = .string
city = .string
```
