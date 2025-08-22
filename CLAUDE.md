# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## About EURE

EURE is a minimalist, schema-friendly data format and language ecosystem designed for configuration and data exchange. It combines JSON compatibility with TOML-like minimalism, featuring algebraic data models, rich editor support, and templating capabilities.

## Commands

**Build and Test:**
```bash
cargo build          # Build all crates
cargo test           # Run all tests
cargo clippy         # Run linting
```

**Individual Crate Development:**
```bash
cargo build -p eure-fmt     # Build specific crate
cargo test -p eure-tree     # Test specific crate
```

**Parser Development:**
The parser uses a custom fork of Parol on the `parse2` branch. Grammar changes require regenerating the parser.

## Architecture

**Workspace Structure:**
- 18 crates organized by functionality in a Rust workspace
- Core libraries: `eure-value` (data types), `eure-tree` (AST/CST), `eure-parol` (parser)
- Format support: `eure-json`, `eure-toml`, `eure-yaml`, `serde-eure`
- Tooling: `eure-cli`, `eure-ls` (LSP), `eure-fmt` (formatter), `eure-lint`
- Schema system: `eure-schema`, `eure-json-schema`, `eure-template`

**Key Patterns:**
- **Visitor Pattern:** Extensive use of `CstVisitor` trait for AST/CST traversal
- **Parser-First Design:** Uses Parol LL(k) parser generator with type-safe structures
- **Modular Format Support:** Each format converter is a separate crate sharing core data structures

**EURE Syntax Features:**
- Array indexing with `[]` notation (e.g., `actions[]`)
- Extension namespaces with `$` prefix (e.g., `$variant`, `$eure.version`)
- Multi-line text and code blocks with language tagging
- Block syntax with `{}` for complex nested structures
- Comments with `#`

**Language Server (eure-ls):**
Implements LSP for IDE integration with semantic tokens, diagnostics, and formatting support.

**Development Notes:**
- Uses custom Parol fork - ensure you're on the `parse2` branch when updating parser
- The project is marked as "Under Construction" with many TODOs in active development
- Formatter includes both format and "unformat" capabilities