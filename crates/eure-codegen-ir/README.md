# eure-codegen-ir

`eure-codegen-ir` is the canonical intermediate representation used to bridge:

- Rust derive inputs (`eure-macros`)
- Eure schema data (`eure-codegen` / `eure-schema`)
- Code generation for `FromEure`, `IntoEure`, and `BuildSchema`

This crate focuses on:

- lossless semantic representation
- explicit invariants with typed validation errors
- deterministic structural equality and structural diff

Milestone 1 intentionally excludes serde serialization to avoid freezing a wire format before adapter parity is complete.
