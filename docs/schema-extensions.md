# Schema Extensions

EURE Schema uses extensions to define types and constraints within EURE documents. The schema system is self-hosted, meaning the schema language is defined using itself.

## Overview

EURE Schema is embedded within EURE documents using extension namespaces (prefixed with `$`). The meta-schema can be found in `assets/eure-schema.schema.eure`.

### Extension Levels

- `$foo` - Regular extensions used in documents
- `$ext-type.foo` - Extension type definitions in meta-schemas (defines the type of `$foo`)
- `$cascade-ext-type` - Extensions that cascade down the document tree

---

## Core Concepts

### $root-type

Specifies the root type of target documents validated by this schema.

```eure
$schema = "my-config.schema.eure"

// This schema's root type is a record with specific fields
$root-type = {
  name = .string
  version = .string
}
```

### $ext-type

Defines extension types for specific paths in schemas. When users write documents, the `$ext-type` prefix is omitted.

**Schema side:**
```eure
field.$ext-type.optional = .boolean
field.$ext-type.binding-style = .$types.binding-style
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
$variant: string
min-length = 3
max-length = 20
pattern = "^[a-z0-9_]+$"

// Use the custom type
@ user
name = .$types.username
```

### $cascade-ext-type

Extensions that cascade down the document tree. Unlike `$ext-type`, these are available at any nesting level.

Built-in cascading extensions:
- `cascade-ext-type` - Allows users to define custom cascading extensions
- `description` - Field documentation (plain text or markdown)
- `deprecated` - Marks a field as deprecated
- `default` - Default value for optional fields
- `examples` - Example values in Eure code format

---

## Type Definitions

All types in EURE Schema are variants of a union type (`$types.type`).

### Primitive Types

```eure
// String type
name = .string
name.$description: User's display name

// Integer type
age = .integer
age.$optional = true

// Float type
score = .float

// Boolean type
active = .boolean

// Null type
deleted = .null

// Any type (accepts any valid EURE value)
metadata = .any
```

**Shorthands:** `.string`, `.integer`, `.float`, `.boolean`, `.null`, `.any`

### String Type with Constraints

```eure
// Full form
@ username {
  $variant: string
  min-length = 3
  max-length = 20
  pattern = "^[a-z0-9_]+$"
}

// Or using shorthand with extensions
username = .string
username.$min-length = 3
username.$max-length = 20
username.$pattern = "^[a-z0-9_]+$"
```

### Integer Type with Constraints

```eure
@ age {
  $variant: integer
  min = 0
  max = 150
}

// Or with exclusive bounds
@ temperature {
  $variant: integer
  exclusive-min = -273
  exclusive-max = 1000
  multiple-of = 1
}
```

### Float Type with Constraints

```eure
@ probability {
  $variant: float
  min = 0.0
  max = 1.0
}
```

### Code Type

Code type for code blocks or inline code with optional language specifier.

```eure
// Plaintext code
content = .code

// Code with language
script = .code.javascript
query = .code.sql
email = .code.email
url = .code.url
```

**Shorthand:** `.code`, `.code.rust`, `.code.email`, etc.

### Path Type

Path type for document path values.

```eure
reference = .path

// With constraints
@ ref {
  $variant: path
  starts-with = .config
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
name = .string
age = .integer
email = .code.email

// Explicit record variant
@ user {
  $variant: record
  name = .string
  age = .integer
}

// With unknown fields policy
@ config {
  $variant: record
  host = .string
  port = .integer
  $unknown-fields = "allow"  // or "deny" (default) or a type schema
}
```

### Array Type

Ordered list of elements with the same type.

```eure
// Shorthand
tags = [.string]

// Full form
@ tags {
  $variant: array
  item = .string
  min-length = 1
  max-length = 10
  unique = true
  contains = "required-tag"
}

// Nested arrays
matrix = [[.integer]]
```

### Map Type

Dynamic key-value pairs where all keys have the same type and all values have the same type.

```eure
// Full form
@ headers {
  $variant: map
  key = .string
  value = .string
  min-size = 0
  max-size = 100
}
```

### Tuple Type

Fixed-length, ordered elements where each position has a specific type.

```eure
// Shorthand
point = (.float, .float)
rgb = (.integer, .integer, .integer)

// Full form
@ coordinate {
  $variant: tuple
  elements = [.float, .float, .float]
}
```

### Union Type

Tagged union that accepts one of multiple variant types.

```eure
// Full form with named variants
@ $types.response {
  $variant: union
  variants.success = { data = .any }
  variants.error = { message = .string, code = .integer }
}

// Using the union type
result = .$types.response
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
  variants.circle = { radius = .float }
  variants.rectangle = { width = .float, height = .float }
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
  variants.text = { content = .string }
  variants.image = { url = .string }
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
  variants.login = { username = .string }
  variants.logout = { reason = .string }
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
  variants.string = .string
  variants.number = .integer
}

// Data example:
// "hello" or 42
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
$types.email = .code.email
$types.user = {
  name = .string
  email = .$types.email
}

// Use type reference
contact = .$types.user
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
name = .$types.common.username      // from common.schema.eure
email = .$types.common.email        // from common.schema.eure
token = .$types.auth.jwt-token      // from auth/types.schema.eure
local-field = .$types.my-local-type // local type (no namespace)
```

**Resolution rules:**
- Path length 2 (`.$types.T`): Local type reference
- Path length 3 (`.$types.N.T`): External type reference (N must be in `$import`)

**Important:** Imports are resolved at schema bundling/validation time, not at runtime. Distributed schemas should be self-contained with all imports inlined. This follows EURE's design principle that documents should be self-contained.

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
  $variant: string
  min-length = 3
  max-length = 20
  pattern = "^[a-z0-9_]+$"
}

@ $types.email = .code.email
```

**user.schema.eure:**
```eure
$import = {
  common => "common.schema.eure"
}

@ $types.user {
  username = .$types.common.username
  email = .$types.common.email
  bio = .string
  bio.$optional = true
}

$root-type = .$types.user
```

### Bundling

For distribution, use the schema bundler to inline all imports:

```bash
eure-schema bundle user.schema.eure -o dist/user.schema.eure
```

The bundled schema will be self-contained with all external types inlined and renamed (e.g., `.$types.common.username` → `.$types.common__username`).

---

## Metadata Extensions

### $optional

Marks a field as optional. Fields are required by default.

```eure
@ user
name = .string           // Required
bio = .string
bio.$optional = true     // Optional
```

### $description

Field description (supports plain text or rich markdown).

```eure
@ user
email = .code.email
email.$description: User's primary email address for authentication.

// Or with markdown
email.$description = markdown`User's **primary** email address.`
```

### $deprecated

Marks a field as deprecated.

```eure
old_field = .string
old_field.$deprecated = true
```

### $default

Default value for optional fields.

```eure
timeout = .integer
timeout.$optional = true
timeout.$default = 30
```

### $examples

Example values in Eure code format.

```eure
email = .code.email
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
  host = .string
  port = .integer

  // Reject unknown fields (default)
  $unknown-fields = "deny"

  // Or allow any unknown fields
  $unknown-fields = "allow"

  // Or validate unknown fields against a schema
  $unknown-fields = .string
}
```

---

## Complete Example

```eure
$schema = "eure-schema.schema.eure"

// Custom types
@ $types.username {
  $variant: string
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
  username = .$types.username
  username.$description: Unique username for the account.

  email = .code.email

  role = .$types.role
  role.$default = "user"

  age = .integer
  age.$optional = true
  age.$description: User's age in years.

  tags = [.string]
  tags.$optional = true
  tags.$unique = true
}

// Set root type
$root-type = .$types.user
```

---

## Design Decisions

### No Logical Operators

EURE Schema does not adopt `allOf`/`anyOf`/`oneOf`/`not` from JSON Schema. Use union for alternatives and record for composition.

### No Format Attribute

Instead of format strings, use typed code strings: `.code.email`, `.code.url`, `.code.uuid`, etc.

### Nullable Types

Express nullable types using union with null:

```eure
@ nullable-string {
  $variant: union
  variants.value = .string
  variants.null = .null
}
```

### Discriminators

Union types always have a discriminator. Customize with `$variant-repr`:
- `"external"` - Default, variant name wraps content
- `"untagged"` - No discriminator
- `{ tag = "..." }` - Internal tagging
- `{ tag = "...", content = "..." }` - Adjacent tagging

---

## Meta-Schema

The EURE Schema system is self-hosted. The complete meta-schema defining all schema constructs can be found in `assets/eure-schema.schema.eure`.
