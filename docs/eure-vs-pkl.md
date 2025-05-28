# Eure vs PKL: Comparison of Configuration Languages

## Introduction

This document compares Eure (formerly known as SWON) and PKL, two configuration languages. While both languages are designed for data serialization and configuration management, they each have different approaches and features.

## Overview

### Eure

Eure is a data serialization format designed to be human-friendly, programmatically editable, and compatible with JSON and JSON Schema. It offers a minimalist syntax with rich expressiveness, particularly suited for configuration files and data exchange.

### PKL

PKL (pronounced "Pickle") is an embeddable configuration language that provides rich support for data templating and validation. It can be used from the command line, integrated into a build pipeline, or embedded in a program. It scales from small to large, simple to complex, and ad-hoc to repetitive configuration tasks.

## Feature Comparison

| Feature | Eure | PKL |
|---------|------|-----|
| Focus | Minimalist, schema-friendly, human-friendly | Programmable, scalable, safe |
| Data Model | Algebraic data model, JSON compatible | Rich type system, supports multiple output formats |
| Schema Support | JSON Schema compatible | Validation through type system |
| Templating | Dedicated templating language | Templating features built into the language |
| Editor Support | Rich editor experience (syntax highlighting, linting, formatting) | IDE support, language server |
| Output Formats | JSON, TOML, YAML | JSON, YAML, Java Properties, and others |
| Programmatic Editing | Part of design goals | Built-in as language features |

## Syntax Comparison

### Eure

Eure draws inspiration from TOML's document structure and minimalism, jq's key syntax, and YAML's easy nesting and ":" syntax.

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

@ actions[] {
  $variant: set-text

  @ pages[]
  text: Hi,

  @ pages[]
  speaker: Ryo
  text: I'm Ryo.
}
```

### PKL

PKL has syntax closer to traditional programming languages, with concepts like objects, methods, and classes.

```pkl
bird.pkl
name = "Swallow"

job {
  title = "Sr. Nest Maker"
  company = "Nests R Us"
  yearsOfExperience = 2
}
```

## Use Cases

### Suitable Use Cases for Eure

- Configuration files that humans read and write
- When compatibility with JSON Schema is required
- When expressive configuration with minimal syntax is needed
- When rich editor experience is important

### Suitable Use Cases for PKL

- When programmatic configuration generation is needed
- When complex validation rules exist
- When multiple output formats (JSON, YAML, Properties, etc.) are required
- When integration with build pipelines is important

## Technical Differences

### Eure

- Implemented in Rust
- Uses Parol as a parser generator
- Architecture centered around Concrete Syntax Tree (CST)
- Serialization/deserialization through integration with Serde

### PKL

- JVM-based (Java/Kotlin)
- Multiple language bindings (Java, Go, Swift, Kotlin, etc.)
- Built-in type system and validation
- Code generation capabilities

## Conclusion

Both Eure and PKL are modern configuration languages, but with different strengths and focuses:

- **Eure** focuses on minimal syntax and human readability, with an emphasis on JSON Schema compatibility.
- **PKL** focuses on programmability and scalability, with richer language features and support for multiple output formats.

Depending on project requirements, either of these languages might be the optimal choice. If human readability and minimal syntax are important, Eure might be more suitable; if more complex configuration logic and diverse output formats are needed, PKL might be the better choice.
