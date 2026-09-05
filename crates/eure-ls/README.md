# eure-ls

Language Server Protocol (LSP) implementation for Eure.

This crate provides a language server that can be integrated with various code editors to provide rich features for working with Eure files.

**This crate is still under development and published for name reservation purpose.**

Part of the [Eure](https://eure.dev) project - a minimalist, schema-friendly format with an algebraic data model that's compatible with JSON.

## Definition navigation

`textDocument/definition` returns schema field declarations, schema/import file
references, and named type definitions. Clients with
`textDocument.definition.linkSupport` receive `LocationLink[]`; other clients
receive `Location[]`. Targets retain their original `file:` or `https:` URI.

Clients displaying remote sources can request `eure/schemaContent` with
`{ "uri": "https://eure.dev/…" }`. The response is the source text as a JSON string.
This uses the same asset resolver and host policy as schema analysis; missing
assets suspend the request until loaded, and retrieval errors fail the request.
Remote assets remain loaded when their editor is closed. They are not refreshed
merely by opening a definition.

The VS Code extension maps HTTPS URIs to read-only `eure-schema:` documents and
back on the protocol boundary. Both native and WASM clients use this provider;
relative references are resolved against the original HTTPS URL.
