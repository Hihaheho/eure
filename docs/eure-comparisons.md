# EURE: Configuration Language Comparisons

## Introduction

EURE is a minimalist, schema-friendly configuration language and data format designed for human readability and programmatic editing. It combines JSON compatibility with TOML-like minimalism, featuring algebraic data models, rich editor support, and templating capabilities.

### Core Design Principles

- **Minimalist syntax** with rich expressiveness
- **Schema-friendly** with built-in JSON Schema compatibility
- **Human-readable** and **programmatically editable**
- **Algebraic data model** compatible with JSON
- **Rich editor experience** with LSP support

## EURE Language Features

Based on the official grammar, EURE supports:

### Basic Syntax

```eure
# Comments start with #
title = "Hello World"
port = 8080
enabled = true
debug = null
```

### Sections and Arrays

```eure
# Section with array indexing
@ actions[]
$variant = "sleep"
seconds = 2.0

@ actions[]
$variant = "set-text"
speaker = "Ryo"
```

### Extensions (Metadata)

```eure
# Extensions use $ prefix
$eure.version = "https://eure.dev/versions/v0.1.0"
$variant = "use-script"
```

### Text Blocks

```eure
# Multi-line text using : (no quotes needed, like PKL)
description: This is a multi-line
description that continues here

# Unambiguous unlike YAML - : only means text in this context
title: Hello World
subtitle: A simple example
```

### Nested Structures

```eure
@ actions[] {
  $variant = "set-text"
  
  @ pages[]
  text = "Hi,"
  
  @ pages[]
  speaker = "Ryo"
  text = "I'm Ryo."
}
```

### Code Blocks and Typed Strings

```eure
# Code blocks with language tags
script = ```bash
echo "Hello World"
```

# Typed strings
homepage = url"https://eure.dev"
contact = email"user@example.com"

# Inline code
command = `ls -la`
```

### Objects and Arrays

```eure
# Objects
config = { host = "localhost", port = 8080 }

# Arrays
ports = [8000, 8001, 8002]
mixed = ["string", 42, true, null]
```

## EURE vs YAML

### Syntax Comparison

**EURE:**
```eure
$eure.version = "https://eure.dev/versions/v0.1.0"
title = "test"
language = "en"

@ actions[]
$variant = "use-script"
script-id = "title"

@ actions[]
$variant = "sleep"  
seconds = 2.0

@ actions[] {
  $variant = "set-text"
  
  @ pages[]
  text = "Hi,"
  
  @ pages[]
  speaker = "Ryo"
  text = "I'm Ryo."
}
```

**YAML:**
```yaml
title: test
language: en
actions:
  - variant: use-script
    script-id: title
  - variant: sleep
    seconds: 2.0
  - variant: set-text
    pages:
      - text: Hi,
      - speaker: Ryo
        text: I'm Ryo.
```

### Key Differences

| Feature | EURE | YAML |
|---------|------|------|
| **Structure** | Sections with `@`, blocks with `{}` | Indentation-based |
| **Text syntax** | `:` for unquoted text (unambiguous) | `:` overloaded (maps, strings, etc.) |
| **Extensions** | Built-in `$` prefix system | No standardized extension mechanism |
| **Variants** | Native `$variant` support | Requires manual variant field |
| **Schema** | JSON Schema compatible by design | External schema validation |
| **Specification** | Minimalist, focused | Complex with many edge cases |
| **Code blocks** | Native syntax with language tags | Multi-line strings only |
| **Typed strings** | Built-in (url, email, etc.) | All strings are plain |

### Advantages of EURE over YAML

1. **Simpler specification** - Fewer edge cases and parsing ambiguities
2. **Built-in extensions** - Metadata without polluting data structure
3. **Native variants** - Algebraic data types built into the language
4. **Schema integration** - Designed for validation from the ground up
5. **Code-friendly** - Better support for configuration-as-code scenarios

## EURE vs TOML

### Syntax Comparison

**EURE:**
```eure
title = "test"
language = "en"

@ database
server = "192.168.1.1"
ports = [8001, 8001, 8002]

@ servers[]
name = "alpha"
ip = "10.0.0.1"

@ servers[]
name = "beta" 
ip = "10.0.0.2"
```

**TOML:**
```toml
title = "test"
language = "en"

[database]
server = "192.168.1.1"
ports = [8001, 8001, 8002]

[[servers]]
name = "alpha"
ip = "10.0.0.1"

[[servers]]
name = "beta"
ip = "10.0.0.2"
```

### Key Differences

| Feature | EURE | TOML |
|---------|------|------|
| **Section syntax** | `@ section[]` | `[[section]]` |
| **Section nesting** | ✅ Supports nested sections with `{}` | ❌ No section nesting allowed |
| **Extensions** | `$` prefix for metadata | No extension system |
| **Nesting** | Explicit with `{}` blocks | Dot notation only |
| **Variants** | Native `$variant` | Manual variant fields |
| **Code blocks** | Triple backticks with language | Multi-line strings |
| **Typed strings** | Built-in type prefixes | All strings are plain |
| **Text blocks** | `:` syntax (unquoted, unambiguous) | Limited multi-line support |

### Advantages of EURE over TOML

1. **Section nesting** - EURE allows nested sections, TOML does not
2. **Unquoted text** - `:` syntax allows natural text without quotes (like PKL)
3. **Unambiguous syntax** - Text `:` is context-specific, unlike YAML's overloaded `:`
4. **Richer data model** - Algebraic types and variants
5. **Extension system** - Built-in metadata support
6. **Code integration** - Native code block support
7. **Schema support** - Built-in validation capabilities

## EURE vs PKL

### Philosophy Comparison

**EURE:**
- Configuration language focused on human readability
- Schema-driven validation
- Minimal syntax with rich features
- Static configuration with templating

**PKL:**
- Programmable configuration language
- Type system-based validation  
- Rich programming constructs
- Dynamic configuration generation

### Syntax Comparison

**EURE:**
```eure
$variant = "database-config"
host = "localhost"
port = 5432
ssl = true

@ connections[]
name = "primary"
url = url"postgresql://localhost:5432/app"

@ connections[]
name = "replica"
url = url"postgresql://replica:5432/app"
```

**PKL:**
```pkl
host = "localhost"
port = 5432
ssl = true

connections {
  new {
    name = "primary"
    url = "postgresql://localhost:5432/app"
  }
  new {
    name = "replica" 
    url = "postgresql://replica:5432/app"
  }
}
```

### Key Differences

| Feature | EURE | PKL |
|---------|------|-----|
| **Focus** | Human-readable configuration | Programmable configuration |
| **Validation** | JSON Schema compatible | Rich type system |
| **Templating** | Separate templating language | Built-in language features |
| **Complexity** | Minimal, focused syntax | Full programming language |
| **Output** | JSON, TOML, YAML | Multiple formats via code generation |
| **Learning curve** | Minimal | Requires programming knowledge |

### Use Case Comparison

**Choose EURE when:**
- Configuration files need to be human-readable and editable
- Schema validation is important
- Minimal syntax is preferred
- Rich editor experience is valued
- Working with JSON Schema ecosystems

**Choose PKL when:**
- Complex configuration logic is needed
- Programmatic configuration generation is required
- Multiple output formats are necessary
- Type safety is critical
- Configuration involves computation

## Migration and Interoperability

### Converting Between Formats

EURE provides conversion tools through its ecosystem:

- **eure-json** - Bidirectional JSON conversion
- **eure-toml** - TOML import/export
- **eure-yaml** - YAML import/export

### Migration Strategies

**From YAML to EURE:**
1. Convert basic key-value pairs directly
2. Transform indentation-based nesting to `@` sections
3. Add `$variant` fields for algebraic data types
4. Utilize extensions for metadata

**From TOML to EURE:**
1. Convert `[section]` to `@ section`
2. Convert `[[array]]` to `@ array[]`
3. Add extensions for metadata
4. Utilize richer data types

**From PKL to EURE:**
1. Extract static configuration values
2. Convert computed values to static equivalents
3. Use EURE templating for dynamic aspects
4. Leverage schema validation for type safety

## Tooling and Ecosystem

### EURE Tooling

- **eure-ls** - Language Server Protocol implementation
- **eure-fmt** - Code formatter
- **eure-lint** - Linting and validation
- **eure-cli** - Command-line tools
- **eure-schema** - Schema validation system

### Editor Support

- Syntax highlighting
- Real-time validation
- Auto-completion
- Format-on-save
- Error diagnostics

## Conclusion

EURE occupies a unique position in the configuration language landscape:

- **More expressive than TOML** while maintaining simplicity
- **Simpler than YAML** with fewer parsing edge cases  
- **More declarative than PKL** while supporting rich data models
- **Schema-first design** unlike most alternatives

Choose EURE when you need a human-friendly configuration language that scales from simple key-value pairs to complex, validated data structures while maintaining excellent tooling support and editor integration.