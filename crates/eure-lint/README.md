# eure-lint

Linter for Eure files.

This crate provides linting capabilities for Eure files, helping to enforce best practices and catch potential issues early.

The initial rule set focuses on mistakes that parse successfully but often do
not mean what the author intended:

- `no-comment-in-text-binding`
- `redundant-at-with-braces`
- `nested-at-inside-braceless-at`

Rules return byte spans, stable rule IDs, severities, help text, and optional
edits. Use `apply_fixes` with `FixMode::Safe` to apply only
semantics-preserving fixes.

Part of the [Eure](https://eure.dev) project - a minimalist, schema-friendly format with an algebraic data model that's compatible with JSON.
