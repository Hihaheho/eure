# eure-codegen-ir Design Draft

## Purpose

`eure-codegen-ir` is an intermediate representation (IR) that serves as the canonical bridge
between Rust's type system and Eure's schema/document model. It captures all information
needed for bidirectional code generation:

```
Rust Data Type  ──(eure-macros)──>  IR  ──(codegen)──>  impl FromEure / IntoEure / BuildSchema
Eure Schema     ──(eure-codegen)──> IR  ──(codegen)──>  Rust Data Type definitions
(Schema, IR, EureDoc)               ──(codegen)──>  Rust literal expression
```

Interop path (separate concern from native union semantics):

```
json <-> serde <-> Rust Data Type <-> Eure
```

Interop metadata consumption rule:

- `UnionDef.interop.variant_repr` is consumed only when
  `GenerationConfig.serde_serialize || GenerationConfig.serde_deserialize` is `true`.
- If both serde flags are `false`, IR may keep the metadata but code emission must ignore it.
- This does not alter native Eure union semantics.

### Effects on Architecture

- **eure-macros** depends on `eure-codegen-ir`: derive attributes → IR conversion.
  The three derive macros (FromEure, IntoEure, BuildSchema) share a single IR instead of
  independently re-analyzing `syn::DeriveInput`. This eliminates duplicated logic in
  `parse_record.rs`, `write_record.rs`, `build_record.rs` etc.

- **eure-codegen** depends on `eure-codegen-ir`: schema + codegen metadata → IR conversion.
  Enables schema-to-Rust-code generation with roundtrip guarantees.

- **Consistency**: All three trait implementations are derived from the same IR, making it
  impossible for FromEure and IntoEure to disagree on wire names, field modes, or variant tags.

---

## Requirements

### Primary Requirements

**P1: Derive Macro Input → IR**

The IR must be constructable from Rust derive macro input (`syn::DeriveInput` with `#[eure(...)]`
attributes). This conversion happens in `eure-macros` and produces an IR value that captures
all attribute information. The IR itself must NOT depend on `syn`, `proc_macro2`, or `darling`.

Currently captured attributes:
- Container: `crate`, `rename_all`, `rename_all_fields`, `parse_ext`, `allow_unknown_fields`,
  `allow_unknown_extensions`, `parse_error`, `write_error`, `type_name`, `non_exhaustive`,
  `proxy`, `opaque`
- Field: `rename`, `ext`, `flatten`, `flatten_ext`, `default`, `via`
- Variant: `rename`, `allow_unknown_fields`

**P2: IR → FromEure / IntoEure / BuildSchema Code Generation**

Given an IR value, a code generator must be able to produce implementations for all three
traits. The generated code must be semantically identical to what the current per-trait derive
macros produce. The IR must contain sufficient information so that no additional analysis
of the original Rust AST is needed during code emission.

**P3: Eure Schema + Codegen Metadata → IR**

The IR must be constructable from a `SchemaDocument` augmented with `$codegen` / `$codegen-defaults`
extensions (as defined in `eure-codegen::parse`). The schema-to-IR conversion determines
Rust type names, field names, derive lists, and structural mappings.

**P4: IR → Rust Data Type Definition**

Given an IR value (produced from schema), a code generator must be able to emit complete Rust
source code: `struct`/`enum` definitions with appropriate `#[derive(...)]` and `#[eure(...)]`
attributes.

**P5: Roundtrip Guarantee**

The following roundtrips must preserve structural equivalence:
- `Rust type → IR → BuildSchema code → (compile & run) → Schema → IR → Rust type`
- `Schema → IR → Rust type → IR → BuildSchema code → (compile & run) → Schema`

"Structural equivalence" means: same fields, same wire names, same types, same modes,
same optionality, same variant structure. Cosmetic differences (formatting, ordering of
derives, exact type paths) are acceptable.

**P6: Proc-Macro Independence**

The IR crate must have zero dependency on `syn`, `proc_macro2`, `darling`, or `quote`.
It must be usable from both proc-macro context (eure-macros) and runtime context
(eure-codegen). All types must be plain Rust data structures using `String`, `Vec`,
`IndexMap`, etc.

**P7: Eure–Rust Type Bridge**

The IR must encode the complete mapping between Eure's type system and Rust's type system.
Specifically, it must capture:
- Which Eure schema type each Rust type corresponds to
- How compound types decompose (Vec<T> → array, HashMap<K,V> → map, Option<T> → optional, etc.)
- How named types reference each other ($types.X → another TypeDef)
- Which types are opaque/extern (have their own trait impls, not decomposed)

### Secondary Requirements

**S1: Literal Expression Generation**

Given a triple of (Schema, IR, EureDocument), a code generator should be able to produce
a Rust literal expression that constructs the Rust value represented by the document.

Example:
```eure
name = "Alice"
age = 30
```
With IR for `struct User { name: String, age: i32 }`, produces:
```rust
User { name: "Alice".to_string(), age: 30 }
```

This requires the IR to carry enough type information to determine construction patterns
for each type (struct literal, enum variant, Vec constructor, etc.).

**S2: Extensibility**

The IR should be designed to accommodate future additions:
- New Eure schema types
- New derive attributes
- New codegen metadata extensions
- Schema constraints (min/max, pattern, etc.) for documentation or validation codegen

---

## Design Considerations

### C1: TypeRef — The Central Challenge

The core challenge is how to reference types within the IR.

**From derive macros**: Types are Rust paths (`String`, `Vec<u32>`, `my_crate::MyType`).
At proc-macro time, we can pattern-match on known types (Option, Vec, HashMap) but cannot
resolve type aliases or determine trait implementations.

**From schemas**: Types are schema structures (text, integer, array, record, union, $types.X).
The Rust type is derived from the schema type + codegen configuration.

**For code generation**: We need exact Rust type paths to emit in generated code (e.g.,
`ctx.parse::<Vec<String>>()`, `ctx.build::<u32>()`).

**For schema generation**: We need to know the schema structure (is it an array? a map?
a record?) to generate the correct `SchemaNodeContent` variant.

**For literal generation**: We need to know construction patterns (struct literal, `vec![]`,
`.to_string()`, etc.).

The TypeRef must serve all three purposes. It needs structural decomposition for schema/literal
generation while retaining the Rust type identity for code emission.

### C2: Generic Type Parameters

Derive macros support generic types (`struct Foo<T>`). Schemas have no concept of generics.
The IR must handle generics for the derive → IR → code path but doesn't need them for
the schema → IR → code path. Generic parameters appear as opaque type references that
constrain trait bounds in generated impls.

### C3: Proxy / Opaque Pattern

The `proxy = "T"` and `opaque = "T"` attributes implement FromEure/IntoEure for a remote
type. These are purely Rust-side concerns with no schema equivalent. The IR must capture
this information for code generation but it has no effect on schema structure.

### C4: Custom Error Types

`parse_error` and `write_error` configure the associated `Error` type in generated trait
impls. These are Rust-specific and only affect code generation, not schema structure.

### C5: Via Types

`via = "MarkerType"` enables conversion through an intermediate type for remote types.
This affects both FromEure (parse via marker) and IntoEure (write via marker) code generation
but has no schema equivalent. The via type is a Rust path.

### C6: Flatten Semantics

Flatten exists in both domains:
- **Rust**: `#[eure(flatten)]` spreads fields of a nested struct into the parent record
- **Schema**: `$flatten` merges schemas into a record

The IR must align these. A flatten field has no wire name — its fields are merged into the
parent's field space. The IR tracks which fields are flattened so that:
- FromEure generates `rec.flatten()` calls
- IntoEure generates `rec.flatten::<T>()` calls
- BuildSchema adds to the `flatten: Vec<SchemaNodeId>` list

### C7: Default Values

Two origins, fundamentally different representations:
- **From derive**: `Default::default()` or `path::to::function()`
- **From schema**: An `EureDocument` representing the default value

The IR must unify these. For derive→IR, defaults are `DefaultTrait` or `Function(path)`.
For schema→IR, defaults are `Value(doc)` which would need literal generation to emit code.

### C8: Optional Fields

Two detection mechanisms:
- **From derive**: `Option<T>` type detection (heuristic on last path segment)
- **From schema**: `$ext-type.optional = true` on record field

The IR normalizes this to a boolean `optional` flag, independent of whether the Rust type
is `Option<T>`. When generating code from schema, optional fields become `Option<T>` in Rust.
When generating schema from derive, `Option<T>` fields get `optional: true` in schema.

### C9: Name Mapping

Eure uses kebab-case (`user-name`), Rust uses snake_case/PascalCase (`user_name`/`UserName`).
The IR stores both the Rust name and the wire name for every field, variant, and type.
Rename rules (`rename_all`) are resolved during IR construction — the IR contains
fully-resolved names, not rename rules.

### C10: Extension Fields

Eure has two field namespaces: record fields and extension fields (`$ext-name`).
The IR captures the field mode (Record vs Extension) which determines:
- FromEure: `rec.parse_field()` vs `ctx.parse_ext()`
- IntoEure: `rec.field()` vs `rec.set_extension()`
- BuildSchema: Extension fields are modeled via `ext_types` on schema nodes

### C11: parse_ext Mode

When `#[eure(parse_ext)]` is set on a container, ALL fields are parsed from the extension
namespace. This is a container-level flag, not a per-field flag. In this mode,
`#[eure(flatten)]` is disallowed (only `#[eure(flatten_ext)]` is valid).

### C12: Arena vs Tree for IR Structure

Schema uses arena-based storage (`SchemaNodeId`). The IR has two options:
- **Tree**: TypeDefs own their fields/variants directly. Simpler, natural for derive input.
- **Arena**: TypeDefs reference each other by ID. Matches schema structure, enables cycles.

Tree structure is simpler and sufficient — type references between TypeDefs use names
(resolved at code generation time), avoiding the need for an arena.

---

## Test Items

### Roundtrip Tests

1. **Simple struct roundtrip**: `struct User { name: String, age: u32 }` → IR → Schema → IR → struct
2. **Enum roundtrip**: `enum Shape { Circle(f64), Rect { w: f64, h: f64 } }` → IR → Schema → IR → enum
3. **Schema → IR → Rust → IR → Schema**: schema with record, union, nested types

### TypeRef Tests

4. **Primitive decomposition**: `String` → `Primitive(String)`, `i32` → `Primitive(I32)`, etc.
5. **Compound decomposition**: `Vec<String>` → `Array(Primitive(String))`
6. **Nested compounds**: `Option<Vec<u32>>` → `Optional(Array(Primitive(U32)))`
7. **HashMap decomposition**: `HashMap<String, i32>` → `Map(Primitive(String), Primitive(I32))`
8. **Opaque types**: `MyCustomType` → `Extern("MyCustomType")`
9. **Generic params**: `T` → `GenericParam("T")`
10. **Named type refs**: reference to another TypeDef → `Named("user")`

### Field Mode Tests

11. **Record field**: `name: String` with `#[eure(rename = "user-name")]`
12. **Extension field**: `#[eure(ext)] version: String`
13. **Flatten field**: `#[eure(flatten)] inner: InnerType`
14. **FlattenExt field**: `#[eure(flatten_ext)] exts: ExtType`
15. **Mode conflicts**: reject `flatten + ext`, `flatten + flatten_ext`, etc.

### Default Value Tests

16. **No default**: field without `#[eure(default)]`
17. **Default trait**: `#[eure(default)]` → `DefaultTrait`
18. **Custom function**: `#[eure(default = "my_fn")]` → `Function("my_fn")`
19. **Schema default value**: schema with `$ext-type.default` → `Value(doc)`

### Rename Tests

20. **Container rename_all**: `#[eure(rename_all = "kebab-case")]` resolves field wire names
21. **Field rename**: `#[eure(rename = "custom-name")]` overrides container rule
22. **Variant rename**: `#[eure(rename = "custom")]` on enum variant
23. **Enum rename_all_fields**: affects struct variant field names

### Struct Shape Tests

24. **Named struct**: `struct Foo { a: T, b: U }` → `Record`
25. **Newtype struct**: `struct Foo(T)` → `Newtype`
26. **Tuple struct**: `struct Foo(T, U)` → `Tuple`
27. **Unit struct**: `struct Foo` → `Unit`

### Variant Shape Tests

28. **Unit variant**: `enum E { A }` → variant with `Unit` shape
29. **Newtype variant**: `enum E { A(T) }` → variant with `Newtype` shape
30. **Tuple variant**: `enum E { A(T, U) }` → variant with `Tuple` shape
31. **Struct variant**: `enum E { A { x: T } }` → variant with `Record` shape

### Proxy / Opaque Tests

32. **Proxy config**: `#[eure(proxy = "ext::Type")]` captured in IR
33. **Opaque config**: `#[eure(opaque = "ext::Type")]` captured in IR
34. **Mutual exclusion**: reject `proxy + opaque` on same type

### Via Type Tests

35. **Field via**: `#[eure(via = "Marker")]` on record field
36. **Newtype via**: `#[eure(via = "Marker")]` on newtype inner field
37. **Variant via**: `#[eure(via = "Marker")]` on enum variant field
38. **Via + flatten conflict**: reject `via + flatten`

### Schema → IR Tests

39. **Text schema → String TypeRef**
40. **Integer schema → integer TypeRef (i64 default, configurable)**
41. **Float schema → f64/f32 TypeRef based on precision**
42. **Record schema → TypeDef with Record shape**
43. **Union schema → TypeDef with Union shape**
44. **Array schema → Array TypeRef**
45. **Map schema → Map TypeRef**
46. **Tuple schema → Tuple TypeRef**
47. **Optional field → field with optional=true and Optional TypeRef**
48. **$types reference → Named TypeRef**
49. **Codegen type name override**: `$codegen.type = "MyType"` → rust_name
50. **Codegen field name override**: `$codegen.name = "my_field"` → rust_name
51. **Codegen variant_types**: union codegen settings captured
51a. **Union interop metadata**: `$interop.variant-repr` → `UnionDef.interop.variant_repr`
51b. **Legacy extension rejection**: `$variant-repr` is rejected before IR construction
51c. **Serde disabled ignores interop**: with `serde_serialize=false` and
     `serde_deserialize=false`, `variant_repr` does not affect emitted code
51d. **Serialize-only consumes interop**: with `serde_serialize=true`,
     `serde_deserialize=false`, mapping is applied
51e. **Deserialize-only consumes interop**: with `serde_serialize=false`,
     `serde_deserialize=true`, mapping is applied
51f. **External mapping**: `None` and `External` both emit no enum-level tag attribute
51g. **Internal mapping**: emits `#[serde(tag = \"...\")]`
51h. **Adjacent mapping**: emits `#[serde(tag = \"...\", content = \"...\")]`
51i. **Untagged mapping**: emits `#[serde(untagged)]`

### Code Generation Tests

52. **IR → FromEure impl**: correct parse calls, field assignments, unknown field checks
53. **IR → IntoEure impl**: correct write calls, flatten support, variant matching
54. **IR → BuildSchema impl**: correct schema node construction, type registration
55. **IR → Rust type definition**: struct/enum with derives and eure attributes

### Literal Generation Tests

56. **Primitive literals**: `"hello"` → `"hello".to_string()`, `42` → `42i32`
57. **Record literal**: `{ name = "Alice" }` → `User { name: "Alice".to_string() }`
58. **Enum literal**: `$variant = "circle"` + `0.5` → `Shape::Circle(0.5)`
59. **Array literal**: `[1, 2, 3]` → `vec![1, 2, 3]`
60. **Nested literal**: record containing array of records

### Edge Cases

61. **Empty struct**: no fields
62. **Single-field struct vs newtype**: named vs unnamed
63. **Recursive types**: `struct Node { children: Vec<Node> }` → Named self-reference
64. **Generic type with bounds**: `struct Foo<T: Display>`
65. **Multiple flatten fields**: flatten + flatten_ext on same struct

---

## Design Proposal

### Module Structure

```
crates/eure-codegen-ir/
├── Cargo.toml
├── src/
│   ├── lib.rs          # Public API: TypeDef, TypeRef, FieldDef, etc.
│   ├── types.rs        # TypeRef, PrimitiveKind, CompoundType
│   ├── typedef.rs      # TypeDef, TypeShape, RecordDef, UnionDef
│   ├── field.rs        # FieldDef, FieldMode, DefaultDef
│   ├── config.rs       # ContainerConfig, CodegenConfig
│   └── visit.rs        # Visitor trait for IR traversal
```

### Core Types

```rust
/// A module of type definitions — the top-level IR unit.
/// Contains one or more related type definitions that can reference each other.
pub struct IrModule {
    /// All type definitions in this module.
    pub types: Vec<TypeDef>,
    /// Module-level codegen defaults (from $codegen-defaults).
    pub defaults: CodegenDefaults,
}

/// A single type definition mapping a Rust type to an Eure structure.
pub struct TypeDef {
    /// Rust type name (PascalCase). e.g., "UserProfile"
    pub rust_name: String,
    /// Schema type name for $types registration (kebab-case). e.g., "user-profile"
    /// None for anonymous/inline types.
    pub schema_name: Option<String>,
    /// The structural shape of this type.
    pub shape: TypeShape,
    /// Container-level configuration.
    pub config: ContainerConfig,
    /// Codegen-specific configuration (derives, visibility, etc.)
    pub codegen: TypeCodegenConfig,
}

/// The structural shape of a type definition.
pub enum TypeShape {
    /// Named struct with fields → Eure record.
    /// `struct Foo { name: String, age: u32 }`
    Record(RecordDef),

    /// Newtype struct → delegates to inner type.
    /// `struct Foo(Bar)`
    Newtype(NewtypeDef),

    /// Tuple struct → Eure tuple.
    /// `struct Foo(String, u32)`
    Tuple(TupleDef),

    /// Unit struct → Eure null.
    /// `struct Foo;`
    Unit,

    /// Enum → Eure union.
    /// `enum Foo { A, B(String), C { x: i32 } }`
    Union(UnionDef),
}
```

### Record and Field Types

```rust
pub struct RecordDef {
    pub fields: Vec<FieldDef>,
}

/// A single field in a record or struct variant.
pub struct FieldDef {
    /// Rust field name (snake_case). e.g., "user_name"
    pub rust_name: String,
    /// Wire name in Eure document (kebab-case). e.g., "user-name"
    /// Fully resolved (rename_all already applied).
    pub wire_name: String,
    /// Type of this field.
    pub ty: TypeRef,
    /// Where this field lives in the Eure document.
    pub mode: FieldMode,
    /// Whether this field is optional (maps to Option<T> in Rust,
    /// `$ext-type.optional = true` in schema).
    pub optional: bool,
    /// Default value specification.
    pub default: DefaultDef,
    /// Via type for remote type conversion.
    /// The string is a Rust type path (e.g., "crate::MyMarker").
    pub via: Option<String>,
}

/// Where a field's data comes from in an Eure document.
pub enum FieldMode {
    /// Regular record field (key-value in map).
    Record,
    /// Extension field ($ext-name).
    Extension,
    /// Flatten: merge nested record fields into parent.
    Flatten,
    /// Flatten extensions: merge nested extension fields into parent.
    FlattenExt,
}

/// Default value specification for a field.
pub enum DefaultDef {
    /// No default — field is required (unless optional).
    None,
    /// Use `Default::default()`.
    DefaultTrait,
    /// Call a named function. e.g., "crate::defaults::my_default"
    Function(String),
    /// A constant value from schema. Contains the Eure source text
    /// for the default value (to be parsed or used for literal generation).
    Value(String),
}
```

### Union and Variant Types

```rust
pub struct UnionDef {
    pub variants: Vec<VariantDef>,
    /// Interop-only metadata (does not affect native Eure union semantics).
    /// Consumed by codegen only when serde derive is enabled.
    pub interop: UnionInteropDef,
}

pub struct VariantDef {
    /// Rust variant name (PascalCase). e.g., "Circle"
    pub rust_name: String,
    /// Wire name / variant tag (kebab-case). e.g., "circle"
    /// Fully resolved (rename_all already applied).
    pub wire_name: String,
    /// Shape of this variant's data.
    pub shape: VariantShape,
    /// Allow unknown fields (only meaningful for Record variants).
    pub allow_unknown_fields: bool,
}

pub enum VariantShape {
    /// `Variant` — no data, serialized as literal text.
    Unit,
    /// `Variant(T)` — single inner value.
    Newtype {
        ty: TypeRef,
        via: Option<String>,
    },
    /// `Variant(T1, T2)` — positional fields.
    Tuple(Vec<TupleElement>),
    /// `Variant { field: T }` — named fields.
    Record(Vec<FieldDef>),
}

pub struct TupleElement {
    pub ty: TypeRef,
    pub via: Option<String>,
}

/// Interop metadata for unions.
pub struct UnionInteropDef {
    /// Optional external representation hint.
    /// None means: no interop override specified and serde default behavior
    /// (externally tagged) is used.
    pub variant_repr: Option<VariantRepr>,
}

/// How union variants are represented for interop bridges
/// (JSON/Serde/codegen targets).
/// Mirrors `eure_schema::interop::VariantRepr`.
pub enum VariantRepr {
    External,
    Internal { tag: String },
    Adjacent { tag: String, content: String },
    Untagged,
}
```

### Newtype and Tuple Struct Types

```rust
pub struct NewtypeDef {
    /// The inner type.
    pub inner: TypeRef,
    /// Via type for remote type conversion.
    pub via: Option<String>,
}

pub struct TupleDef {
    /// Elements of the tuple struct.
    pub elements: Vec<TupleElement>,
}
```

### TypeRef — The Type Bridge

```rust
/// Reference to a type, bridging Eure's type system and Rust's type system.
///
/// TypeRef captures enough structure to:
/// 1. Emit Rust type paths in generated code
/// 2. Determine the corresponding Eure schema type
/// 3. Generate literal construction expressions
///
/// When constructed from derive macros, known types (String, Vec, Option, etc.)
/// are decomposed into structural variants. Unknown types become `Extern`.
///
/// When constructed from schemas, all types are fully structural (no Extern).
pub enum TypeRef {
    // ── Primitives ──────────────────────────────────────────────
    /// `String` or `&str` ↔ Eure text
    String,
    /// `bool` ↔ Eure boolean
    Bool,
    /// Integer types ↔ Eure integer
    Integer(IntegerKind),
    /// Float types ↔ Eure float
    Float(FloatKind),
    /// `()` ↔ Eure null
    Unit,
    /// `eure_document::Text` ↔ Eure text (with language tag)
    Text,

    // ── Compounds ───────────────────────────────────────────────
    /// `Vec<T>` ↔ Eure array
    Array(Box<TypeRef>),
    /// `HashMap<K, V>` / `BTreeMap<K, V>` / `IndexMap<K, V>` ↔ Eure map
    Map {
        key: Box<TypeRef>,
        value: Box<TypeRef>,
        impl_type: MapImplType,
    },
    /// `(T1, T2, ...)` ↔ Eure tuple
    Tuple(Vec<TypeRef>),
    /// `Option<T>` ↔ optional field / union { some(T), none(null) }
    Optional(Box<TypeRef>),
    /// `Result<T, E>` ↔ union { ok(T), err(E) }
    Result {
        ok: Box<TypeRef>,
        err: Box<TypeRef>,
    },
    /// `Box<T>`, `Rc<T>`, `Arc<T>` — transparent wrappers
    Wrapper {
        inner: Box<TypeRef>,
        wrapper: WrapperKind,
    },

    // ── References ──────────────────────────────────────────────
    /// Reference to a named TypeDef in this IrModule.
    /// Corresponds to `$types.X` in schema.
    Named(String),

    /// Opaque Rust type path — not decomposed by the IR.
    /// Used for user-defined types that have their own FromEure/IntoEure/BuildSchema impls.
    /// e.g., "my_crate::CustomType", "chrono::NaiveDate"
    Extern(String),

    /// Generic type parameter. e.g., "T"
    /// Only used in derive macro context.
    GenericParam(String),
}

pub enum IntegerKind {
    I8, I16, I32, I64, I128,
    U8, U16, U32, U64, U128,
    Isize, Usize,
}

pub enum FloatKind {
    F32,
    F64,
}

pub enum MapImplType {
    HashMap,
    BTreeMap,
    IndexMap,
}

pub enum WrapperKind {
    Box,
    Rc,
    Arc,
}
```

### Container Configuration

```rust
/// Container-level configuration affecting code generation behavior.
/// These settings come from `#[eure(...)]` on the container type or from
/// codegen metadata in schemas.
pub struct ContainerConfig {
    /// Parse all fields from extension namespace instead of record fields.
    pub parse_ext: bool,
    /// Allow unknown record fields (instead of denying).
    pub allow_unknown_fields: bool,
    /// Allow unknown extensions (instead of denying).
    pub allow_unknown_extensions: bool,
    /// Custom error type path for FromEure impl.
    pub parse_error: Option<String>,
    /// Custom error type path for IntoEure impl.
    pub write_error: Option<String>,
    /// Proxy configuration for implementing traits on remote types.
    pub proxy: Option<ProxyDef>,
    /// Treat enum as non-exhaustive (adds wildcard arm in IntoEure).
    pub non_exhaustive: bool,
}

pub struct ProxyDef {
    /// The target type to implement traits for.
    pub target: String,
    /// If true, uses From conversion (opaque). If false, uses direct struct literal (proxy).
    pub is_opaque: bool,
}
```

### Codegen Configuration

```rust
/// Codegen-specific settings that affect Rust source generation (from schemas).
/// These come from `$codegen` and `$codegen-defaults` extensions.
pub struct CodegenDefaults {
    /// Default derive macros for all generated types.
    pub derive: Vec<String>,
    /// Prefix for extension type field names. e.g., "ext_"
    pub ext_types_field_prefix: Option<String>,
    /// Prefix for extension type names. e.g., "Ext"
    pub ext_types_type_prefix: Option<String>,
    /// Field name for storing document node ID.
    pub document_node_id_field: Option<String>,
}

/// Per-type codegen settings.
pub struct TypeCodegenConfig {
    /// Override derive macros for this type.
    pub derive: Option<Vec<String>>,
    /// For unions: generate separate types for each variant.
    pub variant_types: Option<bool>,
    /// For unions: suffix for generated variant type names.
    pub variant_types_suffix: Option<String>,
}
```

### Serde Attribute Mapping for `variant_repr`

Apply this mapping only when
`GenerationConfig.serde_serialize || GenerationConfig.serde_deserialize` is `true`.
If both serde flags are `false`, keep IR metadata but do not emit serde tagging attributes.

- `None` or `Some(VariantRepr::External)`:
  no enum-level serde tag attribute (serde default externally tagged)
- `Some(VariantRepr::Internal { tag })`:
  `#[serde(tag = "...")]`
- `Some(VariantRepr::Adjacent { tag, content })`:
  `#[serde(tag = "...", content = "...")]`
- `Some(VariantRepr::Untagged)`:
  `#[serde(untagged)]`

### Generic Type Parameters (derive-only)

```rust
/// Generic type parameter information (only relevant for derive macros).
/// Captured from `struct Foo<T: Bound>`.
pub struct GenericParam {
    /// Parameter name. e.g., "T"
    pub name: String,
    /// Trait bounds as Rust path strings. e.g., ["Display", "Clone"]
    pub bounds: Vec<String>,
}
```

### TypeRef Conversion from syn::Type (in eure-macros)

The conversion from `syn::Type` to `TypeRef` is best-effort pattern matching:

```rust
// Pseudocode for syn::Type → TypeRef conversion (lives in eure-macros, not in IR crate)
fn syn_type_to_typeref(ty: &syn::Type) -> TypeRef {
    match last_segment_name(ty) {
        "String"    => TypeRef::String,
        "str"       => TypeRef::String,
        "bool"      => TypeRef::Bool,
        "i32"       => TypeRef::Integer(IntegerKind::I32),
        // ... other primitives ...
        "Vec"       => TypeRef::Array(Box::new(convert_first_generic_arg(ty))),
        "Option"    => TypeRef::Optional(Box::new(convert_first_generic_arg(ty))),
        "HashMap"   => TypeRef::Map { ... },
        "Box"       => TypeRef::Wrapper { inner: ..., wrapper: WrapperKind::Box },
        // ... other known types ...
        "Text"      => TypeRef::Text,
        name        => {
            if is_single_ident_uppercase(name) && is_generic_param(name) {
                TypeRef::GenericParam(name.to_string())
            } else {
                TypeRef::Extern(type_to_path_string(ty))
            }
        }
    }
}
```

### TypeRef Conversion from SchemaNodeContent (in eure-codegen)

```rust
// Pseudocode for SchemaNodeContent → TypeRef conversion (lives in eure-codegen)
fn schema_to_typeref(content: &SchemaNodeContent, codegen: &CodegenConfig) -> TypeRef {
    match content {
        SchemaNodeContent::Text(_)    => TypeRef::String,  // or TypeRef::Text based on config
        SchemaNodeContent::Integer(_) => TypeRef::Integer(codegen.default_integer_kind()),
        SchemaNodeContent::Float(f)   => match f.precision {
            FloatPrecision::F32 => TypeRef::Float(FloatKind::F32),
            FloatPrecision::F64 => TypeRef::Float(FloatKind::F64),
        },
        SchemaNodeContent::Boolean    => TypeRef::Bool,
        SchemaNodeContent::Null       => TypeRef::Unit,
        SchemaNodeContent::Array(a)   => TypeRef::Array(Box::new(convert(a.item))),
        SchemaNodeContent::Map(m)     => TypeRef::Map { key: convert(m.key), value: convert(m.value), .. },
        SchemaNodeContent::Tuple(t)   => TypeRef::Tuple(t.elements.iter().map(convert).collect()),
        SchemaNodeContent::Record(_)  => TypeRef::Named(infer_type_name(...)),
        SchemaNodeContent::Union(_)   => TypeRef::Named(infer_type_name(...)),
        SchemaNodeContent::Reference(r) => TypeRef::Named(r.name.to_string()),
        SchemaNodeContent::Any        => TypeRef::Extern("eure_document::document::EureDocument".into()),
    }
}
```

For union definitions, schema → IR conversion also copies interop metadata from
`UnionSchema.interop.variant_repr` into `UnionDef.interop.variant_repr`.
This metadata is for JSON/Serde/codegen bridges only and does not alter native Eure
union semantics. Code emitters consume it only when
`serde_serialize || serde_deserialize` is `true`.

### TypeRef → Rust Type Path (for code emission)

```rust
impl TypeRef {
    /// Returns the Rust type path string for use in generated code.
    pub fn to_rust_path(&self) -> String {
        match self {
            TypeRef::String => "String".into(),
            TypeRef::Bool => "bool".into(),
            TypeRef::Integer(IntegerKind::I32) => "i32".into(),
            // ...
            TypeRef::Array(inner) => format!("Vec<{}>", inner.to_rust_path()),
            TypeRef::Optional(inner) => format!("Option<{}>", inner.to_rust_path()),
            TypeRef::Map { key, value, impl_type: MapImplType::HashMap } =>
                format!("std::collections::HashMap<{}, {}>", key.to_rust_path(), value.to_rust_path()),
            TypeRef::Named(name) => to_pascal_case(name),
            TypeRef::Extern(path) => path.clone(),
            TypeRef::GenericParam(name) => name.clone(),
            // ...
        }
    }
}
```

---

## Analysis: Requirements Satisfaction

### What this design satisfies

| Requirement | Status | Notes |
|---|---|---|
| P1: Derive → IR | **Satisfied** | `syn::Type` → `TypeRef`, attrs → `FieldDef`/`ContainerConfig` |
| P2: IR → trait impls | **Satisfied** | All info for FromEure/IntoEure/BuildSchema is captured |
| P3: Schema → IR | **Satisfied** | `SchemaNodeContent` → `TypeRef`, codegen → `TypeDef` |
| P4: IR → Rust type def | **Satisfied** | `TypeDef` + `TypeRef.to_rust_path()` + `CodegenConfig` |
| P5: Roundtrip | **Satisfied** | Same IR from both directions; structural equivalence |
| P6: No proc-macro deps | **Satisfied** | All types are plain Rust (`String`, `Vec`, enums) |
| P7: Type bridge | **Satisfied** | `TypeRef` encodes full Eure↔Rust mapping |
| S1: Literal generation | **Partially** | `TypeRef` structure enables it; `DefaultDef::Value` needs work |
| S2: Extensibility | **Satisfied** | Enum-based design allows adding variants |

### What needs further design work

1. **Literal generation (S1)**: The `DefaultDef::Value(String)` stores Eure source text.
   For literal generation, we need to actually parse the Eure document and walk the IR
   to produce Rust expressions. This requires a separate visitor/emitter that pairs IR
   nodes with document nodes. The IR itself provides the structural info; the algorithm
   is in the consumer.

2. **Extension type schemas**: The current schema has `ext_types: IndexMap<Identifier, ExtTypeSchema>`
   on each node. The IR doesn't model this directly. For types parsed with `parse_ext`,
   the fields already capture extension access. For schema codegen, extension type
   definitions would need to be generated as part of the schema metadata. This could be
   added to `TypeDef` if needed:
   ```rust
   pub struct TypeDef {
       // ...
       pub ext_type_schemas: Vec<ExtTypeDef>,
   }
   ```

3. **Schema constraints**: The IR currently doesn't capture validation constraints
   (min/max length, pattern, range, unique, etc.). These are schema-only concerns and
   don't affect Rust type structure or FromEure/IntoEure codegen. For BuildSchema, they
   would need to be passed through to `SchemaNodeContent`. This could be added as an
   optional `constraints` field on TypeRef or as metadata on TypeDef.

4. **Union interop metadata in derive macros (settled)**: Native Eure semantics do not
   require `variant-repr`. For derive → IR, `UnionDef.interop.variant_repr` defaults to
   `None`. This metadata is consumed only when
   `serde_serialize || serde_deserialize` is `true`, and remains non-semantic for native
   Eure parsing/validation.

5. **Schema metadata passthrough**: `SchemaMetadata` (description, deprecated, default,
   examples) is not modeled in the IR. For schema→IR→Rust generation, descriptions could
   become doc comments. For IR→BuildSchema, metadata needs to be emitted. Adding optional
   metadata to `TypeDef` and `FieldDef`:
   ```rust
   pub struct TypeDef {
       // ...
       pub description: Option<String>,
       pub deprecated: bool,
   }
   ```

---

## Dependencies

```toml
[package]
name = "eure-codegen-ir"
version = "0.1.0"
edition = "2024"

[dependencies]
indexmap = "2"
```

The crate should be minimal — no `syn`, no `eure-document`, no `eure-schema`.
Conversions to/from those types live in the consumer crates (`eure-macros`, `eure-codegen`).

---

## Migration Path

1. **Create `eure-codegen-ir`** with the types described above.
2. **Add conversion in `eure-macros`**: `syn::DeriveInput` → `IrModule` (replaces current
   `analyze_common_named_fields` + per-macro dispatch).
3. **Add code emitters in `eure-macros`**: `IrModule` → `TokenStream` for each trait.
   Initially these replicate existing behavior exactly.
4. **Add conversion in `eure-codegen`**: `SchemaDocument` + codegen config → `IrModule`.
5. **Add Rust type emitter in `eure-codegen`**: `IrModule` → Rust source code.
6. **Verify roundtrip**: schema → IR → Rust → IR → schema produces equivalent schemas.
7. **Add literal generation**: `(IrModule, EureDocument)` → Rust expression source.
