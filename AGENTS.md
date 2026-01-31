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

You must ensure this command succeeds before commit.

```bash
make check
```

**Individual Crate Development:**
```bash
cargo run -p eure-gen # Regenerate eure-parol and eure-tree based on @crates/eure-parol/eure.par
cargo run --bin eure -- <commands> # Run eure CLI for validating file or conversion reasons.
cargo run --bin eure -- check <file> # Validate a Eure file syntax and against schema.
cargo run -p test-suite -- [--filter <filter>]# Run Eure test suite.
```

## Architecture

**Workspace Structure:**
- 18 crates organized by functionality in a Rust workspace
- Core libraries: `eure-document` (data types), `eure-tree` (AST/CST), `eure-parol` (parser)
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
- Comments with `//` and `/* */`

**Language Server (eure-ls):**
Implements LSP for IDE integration with semantic tokens, diagnostics, and formatting support.
`crates/eure-ls` is the main crate and `editors/vscode` is the user of the wasm module.

**Development Notes:**
- Refer @crates/eure-parol/eure.par to understand the latest grammar.
- Refer EureDocument struct in crates/eure-document for understanding the data model.
- Use module files named after their module (e.g., `foo.rs` instead of `foo/mod.rs`).

**Testing Guidelines:**
- Use `eure!` macro for both test input and expected documents
- Compare entire documents with `assert_eq!`, not manual tree traversal
- NEVER use manual document construction or `doc.node(...).as_map().unwrap().get_node_id(...)` patterns

```rust
// Good
let expected = eure!({ name = "Alice", active = true });
assert_eq!(actual_doc, expected);

// Bad: manual traversal
let name_id = doc.node(root_id).as_map().unwrap().get_node_id(&"name".into()).unwrap();
assert_eq!(doc.node(name_id).as_primitive().unwrap().as_str(), Some("Alice"));
```

**Error Handling**

- MUST handle errors properly, and NEVER ignore or fallback errors or invalid states.
- Error data must be enum with thiserror, never use String.

**FromEure API:**
- Use `FromEure` trait for type-safe extraction from `EureDocument`. Avoid manual `node.content` matching.
- Primitives: `doc.parse::<&str>(node_id)`, `doc.parse::<i32>(node_id)`, `doc.parse::<bool>(node_id)`
- Records: `let rec = doc.parse_record(node_id)?; rec.field::<T>("name")?; rec.field_optional::<T>("opt")?`
- Extensions: `let ext = doc.parse_extension(node_id); ext.field_optional::<T>("ext-name")?`
- Collections: `doc.parse::<Vec<T>>(node_id)`, `doc.parse::<Map<K, V>>(node_id)`
- Implement `FromEure` for custom types; see `crates/eure-schema/src/parse.rs` for examples.
