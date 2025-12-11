# Schema Extensions

Eure Schema uses extensions to define types and constraints within Eure documents. The schema system is self-hosted, meaning the schema language is defined using itself.

## Overview

Eure Schema is embedded within Eure documents using extension namespaces (prefixed with `$`). The meta-schema can be found in `assets/eure-schema.schema.eure`.

### Extension Levels

- `$foo` - Regular extensions used in documents
- `$ext-type.foo` - Extension type definitions in meta-schemas (defines the type of `$foo`)

---

## Core Concepts

### $root-type

Specifies the root type of target documents validated by this schema.

```eure
$schema = "my-config.schema.eure"

// This schema's root type is a record with specific fields
$root-type = {
  name = `text`
  version = `text`
}
```

### $ext-type

Defines extension types for specific paths in schemas. When users write documents, the `$ext-type` prefix is omitted.

**Schema side:**
```eure
field.$ext-type.optional = `boolean`
field.$ext-type.binding-style = `$types.binding-style`
```

**User side:**
```eure
field.$optional = true
field.$binding-style = "section"
```

### $types

Custom type definitions at document root. Types are referenced using `.$types.type-name`.

```eure
// Define a custom type
@ $types.username
$variant: text
min-length = 3
max-length = 20
pattern = "^[a-z0-9_]+$"

// Use the custom type
@ user
name = `$types.username`
```

---

## Type Definitions

All types in Eure Schema are variants of a union type (`$types.type`).

### Primitive Types

```eure
// String type
name = `text`
name.$description: User's display name

// Integer type
age = `integer`
age.$optional = true

// Float type
score = `float`

// Boolean type
active = `boolean`

// Null type
deleted = `null`

// Any type (accepts any valid Eure value)
metadata = `any`
```

**Shorthands:** `text`, `integer`, `float`, `boolean`, `null`, `any`

Shorthands are for simple types without constraints. For constraints, use the full form.

### Text Type with Constraints

```eure
// Full form (required for constraints)
@ username {
  $variant: text
  min-length = 3
  max-length = 20
  pattern = "^[a-z0-9_]+$"
}
```

### Integer Type with Constraints

```eure
@ age {
  $variant: integer
  range = "[0, 150]"
}

// Rust-style range syntax
@ index {
  $variant: integer
  range = "0..100"      // 0 <= x < 100
}

// Or using shorthand with extensions
age = `integer`
age.$range = "0..=150"  // 0 <= x <= 150
```

### Float Type with Constraints

```eure
@ probability {
  $variant: float
  range = "[0.0, 1.0)"  // 0 <= x < 1
}

// Rust-style
temperature = `float`
temperature.$range = "-273.15.."  // x >= -273.15
```

### Range Syntax

Two formats are supported:

**Rust-style** (left side always inclusive):
- `"0..100"` → 0 ≤ x < 100
- `"0..=100"` → 0 ≤ x ≤ 100
- `"0.."` → x ≥ 0
- `"..100"` → x < 100
- `"..=100"` → x ≤ 100

**Interval notation** (supports all 4 combinations):
- `"[0, 100]"` → 0 ≤ x ≤ 100 (both inclusive)
- `"[0, 100)"` → 0 ≤ x < 100 (left inclusive, right exclusive)
- `"(0, 100]"` → 0 < x ≤ 100 (left exclusive, right inclusive)
- `"(0, 100)"` → 0 < x < 100 (both exclusive)
- `"[0, )"` → x ≥ 0
- `"(, 100]"` → x ≤ 100

### Text Type with Language

Text type supports optional language specifier for code blocks or semantic text.

```eure
// Plain text
content = `text`

// Text with language
script = `text.javascript`
query = `text.sql`
email = `text.email`
url = `text.url`
```

**Shorthand:** `text`, `text.rust`, `text.email`, etc.

### Path Type

Path type for document path values.

```eure
reference = `path`

// With constraints
@ ref {
  $variant: path
  starts-with = `config`
  min-length = 2
  max-length = 10
}
```

---

## Container Types

### Record Type

Fixed named fields where each field has a specific name and type.

```eure
// Shorthand (implicit record)
@ user
name = `text`
age = `integer`
email = `text.email`

// Explicit record variant
@ user {
  $variant: record
  name = `text`
  age = `integer`
}

// With unknown fields policy
@ config {
  $variant: record
  host = `text`
  port = `integer`
  $unknown-fields = "allow"  // or "deny" (default) or a type schema
}
```

### Array Type

Ordered list of elements with the same type.

```eure
// Shorthand
tags = [`text`]

// Full form
@ tags {
  $variant: array
  item = `text`
  min-length = 1
  max-length = 10
  unique = true
  contains = "required-tag"
}

// Nested arrays
matrix = [[`integer`]]
```

### Map Type

Dynamic key-value pairs where all keys have the same type and all values have the same type.

```eure
// Full form
@ headers {
  $variant: map
  key = `text`
  value = `text`
  min-size = 0
  max-size = 100
}
```

### Tuple Type

Fixed-length, ordered elements where each position has a specific type.

```eure
// Shorthand
point = (`float`, `float`)
rgb = (`integer`, `integer`, `integer`)

// Full form
@ coordinate {
  $variant: tuple
  elements = [`float`, `float`, `float`]
}
```

### Union Type

Tagged union that accepts one of multiple variant types.

```eure
// Full form with named variants
@ $types.response {
  $variant: union
  variants.success = { data = `any` }
  variants.error = { message = `text`, code = `integer` }
}

// Using the union type
result = `$types.response`
```

**Note:** Union types always have a discriminator. Use `$variant-repr` to customize representation.

---

## Variant Representation

Union types can be represented in different ways in the data model.

### External (Default)

The variant name wraps the content.

```eure
@ $types.shape {
  $variant: union
  // Default: external representation
  variants.circle = { radius = `float` }
  variants.rectangle = { width = `float`, height = `float` }
}

// Data example:
// circle = { radius = 5.0 }
```

### Internal

Custom tag field name inside the content.

```eure
@ $types.message {
  $variant: union
  $variant-repr = { tag = "type" }
  variants.text = { content = `text` }
  variants.image = { url = `text` }
}

// Data example:
// { type = "text", content = "Hello" }
```

### Adjacent

Separate tag and content fields.

```eure
@ $types.event {
  $variant: union
  $variant-repr = { tag = "kind", content = "data" }
  variants.login = { username = `text` }
  variants.logout = { reason = `text` }
}

// Data example:
// { kind = "login", data = { username = "alice" } }
```

### Untagged

No discriminator field (type is inferred from content).

```eure
@ $types.value {
  $variant: union
  $variant-repr = "untagged"
  variants.string = `text`
  variants.number = `integer`
}

// Data example:
// "hello" or 42
```

### Priority (Ambiguity Resolution)

For untagged unions where multiple variants may match, use `priority` to specify resolution order.

```eure
@ $types.response {
  $variant: union
  $variant-repr = "untagged"
  priority: ["error", "success"]  // error takes precedence

  variants.error = { code = `integer`, message = `text` }
  variants.success = { data = `any` }
}

// If a value matches both error and success, error is selected
```

---

## Literal Type

Literal type accepts only a specific constant value.

```eure
// String literal
@ status {
  = "active"
  $variant: literal
}

// Using in unions for enum-like behavior
@ $types.log-level {
  $variant: union
  variants.debug = { = "debug", $variant: literal }
  variants.info = { = "info", $variant: literal }
  variants.warn = { = "warn", $variant: literal }
  variants.error = { = "error", $variant: literal }
}
```

---

## Type References

Reference other type definitions using path syntax.

```eure
// Define types
$types.email = `text.email`
$types.user = {
  name = `text`
  email = `$types.email`
}

// Use type reference
contact = `$types.user`
```

---

## Cross-Schema Type References

Schemas can import and reference types from other schemas for modularity and reuse.

### $import

Declare schema imports with namespace aliases:

```eure
// my-app.schema.eure
$import = {
  common => "common.schema.eure"
  auth => "./auth/types.schema.eure"
}

// Reference imported types using: .$types.<namespace>.<type-name>
@ user
name = `$types.common.username`      // from common.schema.eure
email = `$types.common.email`        // from common.schema.eure
token = `$types.auth.jwt-token`      // from auth/types.schema.eure
local-field = `$types.my-local-type` // local type (no namespace)
```

**Resolution rules:**
- Path length 2 (`$types.T`): Local type reference
- Path length 3 (`$types.N.T`): External type reference (N must be in `$import`)

**Important:** Imports are resolved at schema bundling/validation time, not at runtime. Distributed schemas should be self-contained with all imports inlined. This follows Eure's design principle that documents should be self-contained.

### $export

Optionally declare which types are exported (public):

```eure
// common.schema.eure

// Export all types (default if $export is omitted)
$export = "*"

// Or export only specific types
$export = ["username", "email", "timestamp"]

@ $types.username { ... }
@ $types.email { ... }
@ $types.timestamp { ... }
@ $types.internal-helper { ... }  // Not exported, cannot be imported
```

### Example: Modular Schema Design

**common.schema.eure:**
```eure
$export = ["username", "email"]

@ $types.username {
  $variant: text
  min-length = 3
  max-length = 20
  pattern = "^[a-z0-9_]+$"
}

@ $types.email = `text.email`
```

**user.schema.eure:**
```eure
$import = {
  common => "common.schema.eure"
}

@ $types.user {
  username = `$types.common.username`
  email = `$types.common.email`
  bio = `text`
  bio.$optional = true
}

$root-type = `$types.user`
```

### Bundling

For distribution, use the schema bundler to inline all imports:

```bash
eure-schema bundle user.schema.eure -o dist/user.schema.eure
```

The bundled schema will be self-contained with all external types inlined and renamed (e.g., `$types.common.username` → `$types.common__username`).

---

## Metadata Extensions

### $optional

Marks a field as optional. Fields are required by default.

```eure
@ user
name = `text`           // Required
bio = `text`
bio.$optional = true     // Optional
```

### $description

Field description (supports plain text or rich markdown).

```eure
@ user
email = `text.email`
email.$description: User's primary email address for authentication.

// Or with markdown
email.$description = markdown`User's **primary** email address.`
```

### $deprecated

Marks a field as deprecated.

```eure
old_field = `text`
old_field.$deprecated = true
```

### $default

Default value for optional fields.

```eure
timeout = `integer`
timeout.$optional = true
timeout.$default = 30
```

### $examples

Example values in Eure code format.

```eure
email = `text.email`
email.$examples = [eure`"user@example.com"`, eure`"admin@company.org"`]
```

---

## Binding Style

Controls how document paths are represented.

```eure
@ field {
  $binding-style = "section"  // or "nested", "binding", "section-binding", etc.
}
```

Options:
- `auto` - Automatically determine the best representation
- `passthrough` - Defer to subsequent keys
- `section` - Create a new section (`@ a.b.c`)
- `nested` - Create a nested section (`@ a.b.c { ... }`)
- `binding` - Bind value (`a.b.c = value`)
- `section-binding` - Section with block (`a.b.c { ... }`)
- `section-root-binding` - Section with root binding (`@ a.b.c = value`)

---

## Unknown Fields Policy

Controls handling of fields not defined in the schema.

```eure
@ config {
  host = `text`
  port = `integer`

  // Reject unknown fields (default)
  $unknown-fields = "deny"

  // Or allow any unknown fields
  $unknown-fields = "allow"

  // Or validate unknown fields against a schema
  $unknown-fields = `text`
}
```

---

## Complete Example

```eure
$schema = "eure-schema.schema.eure"

// Custom types
@ $types.username {
  $variant: text
  min-length = 3
  max-length = 20
  pattern = "^[a-z0-9_]+$"
}

@ $types.role {
  $variant: union
  variants.admin = { = "admin", $variant: literal }
  variants.user = { = "user", $variant: literal }
  variants.guest = { = "guest", $variant: literal }
}

// Main schema
@ $types.user {
  username = `$types.username`
  username.$description: Unique username for the account.

  email = `text.email`

  role = `$types.role`
  role.$default = "user"

  age = `integer`
  age.$optional = true
  age.$description: User's age in years.

  @ tags {
    $variant: array
    item = `text`
    unique = true
  }
  tags.$optional = true
}

// Set root type
$root-type = `$types.user`
```

---

## Design Decisions

### No Logical Operators

Eure Schema does not adopt `allOf`/`anyOf`/`oneOf`/`not` from JSON Schema. Use union for alternatives and record for composition.

### No Format Attribute

Instead of format strings, use typed text: `.text.email`, `.text.url`, `.text.uuid`, etc.

### Nullable Types

Express nullable types using union with null:

```eure
@ nullable-string {
  $variant: union
  variants.value = `text`
  variants.null = `null`
}
```

### Discriminators

Union types always have a discriminator. Customize with `$variant-repr`:
- `"external"` - Default, variant name wraps content
- `"untagged"` - No discriminator
- `{ tag = "..." }` - Internal tagging
- `{ tag = "...", content = "..." }` - Adjacent tagging

---

## Type Checking Algorithm

Eure Schema uses a structural type checking algorithm that is both **sound** (accepted values always conform to the schema) and **complete** (all conforming values are accepted).

### Core Algorithm

Type checking traverses the value structure recursively, matching each node against its corresponding schema:

```
check(value, schema) -> Result<(), TypeError>
```

| Value Type | Schema Type | Validation |
|------------|-------------|------------|
| Null | Null | Type match only |
| Bool | Boolean | const constraint |
| Integer | Integer | min/max, multiple_of, const, enum |
| Float | Float | min/max, const, enum |
| Text | Text | length, pattern, language, const, enum |
| Path | Path | path constraints |
| Hole | Any | Always passes (but marks document incomplete) |
| Array | Array | item type, min/max_items, unique, contains |
| Map | Map | key/value types, min/max_pairs |
| Tuple | Tuple | element types at each position |
| Map | Record | field types + unknown_fields_policy |

### Union Type Checking (oneOf Semantics)

Union types use **oneOf** semantics—exactly one variant must match:

```
check_union(value, union_schema):
  matching = []
  failures = []

  for (name, schema) in union_schema.variants:
    if check(value, schema).is_ok():
      matching.push(name)
    else:
      failures.push((name, error))

  match matching.len():
    0 -> return closest_error(failures)  // No match
    1 -> return Ok                        // Exactly one match
    _ -> resolve_ambiguity(matching, union_schema.priority)

resolve_ambiguity(matching, priority):
  if priority is set:
    for name in priority:
      if name in matching:
        return Ok  // First priority match wins
  return AmbiguityError(matching)
```

**Error Selection**: When no variant matches, the "closest" error is returned based on error depth (how deep into the structure the check progressed before failing).

### Hole Values

The hole value (`!`) represents an unfilled placeholder, similar to a "never" type:

- **Type checking**: Holes match any schema (always pass)
- **Completeness**: Documents containing holes are valid but not complete

Validation returns two flags:

```rust
struct ValidationResult {
    is_valid: bool,    // No type errors (holes allowed)
    is_complete: bool, // No type errors AND no holes
    errors: Vec<TypeError>,
    warnings: Vec<Warning>,
}
```

### Extension Validation

Extensions attached to nodes are validated against three sources:

1. **Schema-defined** (`$ext-type`): Extensions defined in the schema
2. **Built-in**: Extensions provided externally (e.g., `$schema`, `$variant`)
3. **Unknown**: Extensions not in either list

Unknown extensions pass validation but emit a **warning**.

```
check_extensions(node, schema):
  for (name, value) in node.extensions:
    if name in schema.ext_types:
      check(value, schema.ext_types[name])
    else if name in builtin_extensions:
      check(value, builtin_extensions[name])
    else:
      emit_warning(UnknownExtension(name))
```

---

## Meta-Schema

The Eure Schema system is self-hosted. The complete meta-schema defining all schema constructs can be found in `assets/eure-schema.schema.eure`.
