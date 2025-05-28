# EURE vs TOML

This document provides a comparison between EURE (Schema-friendly Web Object Notation) and TOML (Tom's Obvious Minimal Language). EURE is inspired by TOML but offers a richer set of features for data serialization.

## Overview

### EURE

EURE is a data serialization format with the following characteristics:

- Minimalist design
- Schema-friendly
- Algebraic data model
- JSON data model compatible
- JSON Schema compatible
- Rich editor experience
- Human-friendly
- Dedicated templating language
- Programmatically editable

### TOML

TOML stands for "Tom's Obvious Minimal Language" and has the following characteristics:

- Human-readable configuration file format
- Clear semantics
- Unambiguous mapping to a hash table
- Easy to parse into data structures in a variety of languages
- Minimal syntax

## Syntax Comparison

### Basic Keys and Values

#### EURE

```eure
title = "test"
language = "en"
target_window = "Main"
```

#### TOML

```toml
title = "test"
language = "en"
target_window = "Main"
```

Basic key-value pairs are very similar in both formats.

### Sections and Tables

#### EURE (Sections)

```eure
@ actions[]
$variant: use-script
script-id = "title"

@ actions[]
$variant: sleep
seconds = 2.0
```

#### TOML (Tables)

```toml
[[actions]]
variant = "use-script"
script-id = "title"

[[actions]]
variant = "sleep"
seconds = 2.0
```

EURE uses the `@` symbol to define sections, while TOML uses `[]` or `[[]]` to define tables or arrays of tables.

### Nested Structures

#### EURE

```eure
@ actions[] {
  $variant: set-text

  @ pages[]
  text = "Hi,"

  @ pages[]
  speaker = "Ryo"
  text = "I'm Ryo."
}
```

#### TOML

```toml
[[actions]]
variant = "set-text"

[[actions.pages]]
text = "Hi,"

[[actions.pages]]
speaker = "Ryo"
text = "I'm Ryo."
```

EURE can explicitly express nested structures using curly braces `{}`, while TOML uses dot notation to express nesting.

### Extensions

#### EURE

```eure
$eure.version = "https://eure.dev/versions/v0.1.0"
$variant = "use-script"

$my-extension {
  $eure.schema = "https://example.com/schema"
  $comment = "This is a comment to this extension."
}
```

#### TOML

TOML doesn't have the concept of extensions, and all keys are treated as regular keys. If you want to add metadata, you need to define it as regular keys:

```toml
[metadata]
eure_version = "https://eure.dev/versions/v0.1.0"
variant = "use-script"

[metadata.my_extension]
schema = "https://example.com/schema"
comment = "This is a comment to this extension."
```

### Arrays

#### EURE

```eure
ports = [8000, 8001, 8002]
data = ["delta", "phi", [3.14]]
```

#### TOML

```toml
ports = [8000, 8001, 8002]
data = ["delta", "phi", [3.14]]
```

Array representation is very similar in both formats.

### Objects/Inline Tables

#### EURE

```eure
temp_targets = { cpu = 79.5, case = 72.0 }
```

#### TOML

```toml
temp_targets = { cpu = 79.5, case = 72.0 }
```

Inline object/table representation is also similar in both formats.

### Comments

#### EURE

```eure
# This is a comment
key = "value" # This is an end-of-line comment
```

#### TOML

```toml
# This is a comment
key = "value" # This is an end-of-line comment
```

Comment syntax is the same in both formats.

### Typed Strings

#### EURE

```eure
url_value = url"https://example.com"
email_value = email"user@example.com"
```

#### TOML

TOML doesn't have the concept of typed strings, and all strings are treated as regular strings:

```toml
url_value = "https://example.com"
email_value = "user@example.com"
```

### Code Blocks

#### EURE

````eure
code = ```rust
fn main() {
    println!("Hello, world!");
}
```
````

#### TOML

TOML doesn't have special syntax for code blocks and requires using multi-line strings:

```toml
code = '''
fn main() {
    println!("Hello, world!");
}
'''
```

### Variants

#### EURE

```eure
@ actions[]
$variant: set-text
speaker = "ryo"
lines = ["aaa", "bbb"]

@ actions[]
$variant: set-choices
description = "aaa"
```

#### TOML

TOML doesn't have the concept of variants and requires using regular keys to express them:

```toml
[[actions]]
variant = "set-text"
speaker = "ryo"
lines = ["aaa", "bbb"]

[[actions]]
variant = "set-choices"
description = "aaa"
```

## Feature Comparison

| Feature | EURE | TOML |
|---------|------|------|
| Basic Key-Value Pairs | ✅ | ✅ |
| Nested Structures | ✅ (using `@` symbol and `{}`) | ✅ (using dot notation and `[]`/`[[]]`) |
| Arrays | ✅ | ✅ |
| Objects/Tables | ✅ | ✅ |
| Comments | ✅ (using `#`) | ✅ (using `#`) |
| Extensions (Metadata) | ✅ (using `$` prefix) | ❌ (expressed as regular keys) |
| Typed Strings | ✅ (using `type"string"` syntax) | ❌ |
| Code Blocks | ✅ (using triple backticks) | ❌ (expressed as multi-line strings) |
| Variants | ✅ (using `$variant` extension) | ❌ (expressed as regular keys) |
| Schema Support | ✅ (built-in) | ❌ (requires external tools) |
| Date and Time | ✅ | ✅ |
| Multi-line Strings | ✅ | ✅ |
| Inline Code | ✅ (using backticks) | ❌ |
| Templating Features | ✅ | ❌ |

## Data Type Comparison

### EURE

EURE supports the following data types:

- String (`"string"`)
- Typed String (`url"string"`)
- Code (inline and block)
- Integer (`1`)
- Float (`1.1`)
- Decimal (`1.1`)
- Boolean (`true`/`false`)
- Array (`[1, 2, 3]`)
- Object (`{ a = 1, b = 2 }`)
- Enum
- Variant
- Tuple
- Unit
- Null (`null`)
- Path

It also provides the following specialized types:

- URI (`uri"uri"`)
- URL (`url"url"`)
- Email (`email"email"`)
- UUID (`uuid"uuid"`)
- Datetime (`datetime"datetime"`)
- Second-wise duration
- Calendar-wise duration

### TOML

TOML supports the following data types:

- String
- Integer
- Float
- Boolean
- Datetime
- Array
- Table
- Inline Table

## Use Case Comparison

### Suitable Use Cases for EURE

- Configuration files where schema validation is important
- When a rich editor experience is needed
- When metadata or annotations are required
- When code blocks or typed strings are needed
- When complex data structures or variants are needed
- When templating features are required

### Suitable Use Cases for TOML

- Simple configuration files
- When a widely adopted format is needed
- When minimal syntax is sufficient
- When implementations in many languages are needed
- When a gentle learning curve is desired

## Conversion

Conversion between EURE and TOML can be done using the `eure-toml` crate (under development). Common features like basic key-value pairs, arrays, and tables can be directly mapped, but advanced features of EURE like extensions and variants may lose information when converted to TOML.

## Conclusion

EURE is inspired by TOML but provides a richer set of features. In particular, its schema-friendly design, extensions, typed strings, code blocks, and variants make it suitable for more complex data structures and configuration files.

TOML focuses on simplicity and clarity, providing sufficient features for basic configuration files. It is widely adopted and implemented in many languages, making it suitable when compatibility is important.

Choosing the appropriate format depends on the requirements of your project. If you need complex data structures and schema validation, EURE is recommended. If you need simplicity and wide compatibility, TOML is recommended.
