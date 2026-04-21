# eure-mark

`eure-mark` is a generic renderer for articles and pages authored in Eure.

It provides:

- a standard `article.schema.eure`
- parsing for `$frontmatter` + nested heading article documents
- HTML rendering for markdown, code blocks, TOC, alerts, and trusted HTML blocks
- generic syntax highlighting for common languages
- optional Eure-specific semantic highlighting via the `eure-highlight` feature

This crate is intended as a building block for blogs, documentation sites, and static site generators that want Eure-authored content without inheriting product-specific site policy.

## License

Unlike the rest of the main Eure workspace, `eure-mark` is licensed under **MPL-2.0**.

This crate is intentionally licensed separately because it depends on `giallo`, which is copyleft-licensed.

Part of the [Eure](https://eure.dev) project.
