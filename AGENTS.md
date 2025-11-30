# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## About Eure

Eure is a minimalist, schema-friendly data format and language ecosystem designed for configuration and data exchange. It combines JSON compatibility with TOML-like minimalism, featuring algebraic data models, rich editor support, and templating capabilities.

## Commands

**Build and Test:**
```bash
cargo check          # Build all crates
cargo test           # Run all tests
cargo clippy         # Run linting
cargo run -p test-suite # Run Eure test suite
```

**Local checks**

You must ensure those commands succeeds before commit.

```bash
cargo clippy
cargo test
cargo fmt --check
cargo run -p test-suite
```

**Individual Crate Development:**
```bash
cargo run -p eure-parol-gen # Regenerate eure-parol and eure-tree based on @crates/eure-parol/eure.par
cargo run --bin eure -- <commands> # Run eure CLI for validating file or conversion reasons.
```

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

**Eure Syntax Features:**
- Array indexing with `[]` notation (e.g., `actions[]`)
- Extension namespaces with `$` prefix (e.g., `$variant`, `$eure.version`)
- Multi-line text and code blocks with language tagging
- Block syntax with `{}` for complex nested structures
- Comments with `#`

**Language Server (eure-ls):**
Implements LSP for IDE integration with semantic tokens, diagnostics, and formatting support.

**Development Notes:**
- Refer @crates/eure-parol/eure.par to understand the latest grammar.
- Refer EureDocument struct in crates/eure-value for understanding the data model.
