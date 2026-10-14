# eure-reader

Reads Eure source text into an `EureDocument`.

This crate turns the concrete syntax tree produced by `eure-parol` into the data model defined by `eure-document`, along with an `OriginMap` for mapping document nodes back to source spans. It depends only on the core crates (`eure-document`, `eure-parol`, `eure-tree`), so format converters and other lightweight consumers can read Eure without pulling in the schema, formatter, linter, or query runtime that the `eure` crate brings.

- `eure-parol`: text → concrete syntax tree
- `eure-reader`: text → `EureDocument`

Part of the [Eure](https://eure.dev) project - a minimalist, schema-friendly format with an algebraic data model that's compatible with JSON.
