# EURE vs YAML

## Introduction

This document compares EURE (formerly known as SWON) with YAML, highlighting the key differences, advantages, and disadvantages of both data serialization formats.

EURE is a minimalist, schema-friendly format with an algebraic data model that's compatible with JSON, while YAML (YAML Ain't Markup Language) is a human-friendly data serialization language that has been widely used for configuration files and data exchange.

## Overview

### EURE

EURE is a data serialization format designed with the following focus:

- Minimalist approach
- Schema-friendly design
- Algebraic data model
- JSON data model compatibility
- JSON schema compatibility
- Rich editor experience
- Human-friendly syntax
- Dedicated templating language
- Programmatically editable

### YAML

YAML is a human-friendly data serialization language with the following characteristics:

- Human-readable format
- Designed for data serialization
- Used across many programming languages
- Supports complex data structures
- Widely adopted for configuration files
- Uses indentation for structure

## Syntax Comparison

### EURE Syntax

```eure
$eure.version: https://eure.dev/versions/v0.1.0
title: test
language: en
target_window: Main

@ actions[]
$variant: use-script
script-id: title

@ actions[]
$variant: sleep
seconds = 2.0

@ actions[]
$variant: set-base-background-image
image: images/backgrounds/blank.png

@ actions[]
$variant: set-scene
scene: scenes/room_a.scn.ron

@ actions[] {
  $variant: set-text

  @ pages[]
  text: Hi,

  @ pages[]
  speaker: Ryo
  text: I'm Ryo.
}
```

### YAML Syntax

```yaml
title: test
language: en
target_window: Main
actions:
  - variant: use-script
    script-id: title
  - variant: sleep
    seconds: 2.0
  - variant: set-base-background-image
    image: images/backgrounds/blank.png
  - variant: set-scene
    scene: scenes/room_a.scn.ron
  - variant: set-text
    pages:
      - text: Hi,
      - speaker: Ryo
        text: I'm Ryo.
```

## Key Differences

### Data Model

- **EURE**: Uses an algebraic data model that's compatible with JSON. This provides more flexibility in representing complex data structures while maintaining compatibility with JSON.
- **YAML**: Uses a more traditional data model based on mappings, sequences, and scalars.

### Schema Support

- **EURE**: Designed to be schema-friendly from the ground up, with built-in support for JSON Schema.
- **YAML**: Can be used with JSON Schema, but it's not as tightly integrated.

### Syntax

- **EURE**: Uses `@` for sections, `$` for extensions, and both `:` and `=` for key-value pairs.
- **YAML**: Uses indentation for structure, `:` for key-value pairs, and `-` for sequences.

### Extensions

- **EURE**: Has a built-in extension system using the `$` prefix.
- **YAML**: Doesn't have a standardized extension mechanism.

### Sections

- **EURE**: Uses the `@` symbol to denote sections, which can be nested.
- **YAML**: Uses indentation for nesting.

### Variants

- **EURE**: Has built-in support for variants using the `$variant` extension.
- **YAML**: Doesn't have built-in support for variants.

## Specification Complexity

### YAML Specification

YAML's specification is notably complex:

- The YAML 1.2 specification is extensive and detailed, spanning multiple sections with intricate rules
- It includes complex features like anchors, aliases, and tags
- The specification has evolved through multiple versions (1.0, 1.1, 1.2) with compatibility concerns
- Parsing YAML correctly requires handling numerous edge cases and special rules
- Different implementations often interpret the specification differently, leading to compatibility issues

### EURE Specification

EURE takes a minimalist approach to specification:

- Designed with simplicity as a core principle
- Clear, concise syntax rules with fewer special cases
- Focused on being schema-friendly from the ground up
- Maintains a balance between expressiveness and simplicity
- Aims to be more predictable and consistent across implementations

## Advantages of EURE over YAML

1. **Schema Integration**: EURE is designed with schema support in mind, making it easier to validate documents.
2. **Algebraic Data Model**: Provides more flexibility in representing complex data structures.
3. **Extensions**: Built-in support for extensions makes it easier to add metadata without altering the underlying data.
4. **Variants**: Native support for variants makes it easier to represent different types of data.
5. **Templating**: Dedicated templating language for more complex use cases.
6. **Programmatic Editing**: Designed to be easily edited programmatically.
7. **Simplicity**: EURE's specification is intentionally simpler and more focused than YAML's, making it easier to implement correctly and consistently.

## Advantages of YAML over EURE

1. **Widespread Adoption**: YAML is widely used and supported in many programming languages and tools.
2. **Maturity**: YAML has been around for a long time and has a mature ecosystem.
3. **Tooling**: Extensive tooling support across many platforms and languages.
4. **Community**: Large community and extensive documentation.

## Use Cases

### When to Use EURE

- When you need schema validation
- When you're working with complex data structures
- When you need to represent variants
- When you need to add metadata without altering the underlying data
- When you need a format that's both human-readable and programmatically editable

### When to Use YAML

- When you need a widely supported format
- When simplicity is more important than advanced features
- When you're working with configuration files
- When you need a format that's supported by many tools and languages

## Conclusion

Both EURE and YAML are valuable data serialization formats with their own strengths and weaknesses. EURE offers more advanced features like schema integration, extensions, and variants, while YAML offers simplicity and widespread adoption.

The choice between EURE and YAML depends on your specific needs and constraints. If you need advanced features and are willing to adopt a newer format, EURE might be the better choice. If you need a widely supported format with extensive tooling, YAML might be more appropriate.

## References

- [EURE Project](https://eure.dev)
- [YAML Specification](https://yaml.org/spec/1.2.2/)
- [JSON Schema](https://json-schema.org)
