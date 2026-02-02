use darling::FromDeriveInput;
use syn::{Path, Type};

use super::RenameAll;

#[derive(Debug, Default, FromDeriveInput)]
#[darling(attributes(eure), default)]
pub struct ContainerAttrs {
    #[darling(rename = "crate")]
    pub crate_path: Option<Path>,
    pub rename_all: Option<RenameAll>,
    /// Renames all struct variant fields in an enum.
    /// Unlike `rename_all`, this only applies to fields within struct variants.
    pub rename_all_fields: Option<RenameAll>,
    /// Parse fields from extension namespace ($ext-type) instead of record fields.
    /// When true, uses `ctx.parse_extension()` and `ext.parse_ext()` instead of
    /// `ctx.parse_record()` and `rec.parse_field()`.
    pub parse_ext: bool,
    /// Allow unknown fields instead of denying them.
    /// By default (false), unknown fields cause a parse error.
    /// When true, uses `allow_unknown_fields()` instead of `deny_unknown_fields()`.
    pub allow_unknown_fields: bool,
    /// Allow unknown extensions instead of denying them.
    /// By default (false), unknown extensions cause a parse error.
    /// When true, skips the `deny_unknown_extensions()` check.
    pub allow_unknown_extensions: bool,
    /// Custom error type for the FromEure impl.
    /// When specified, the generated `type Error` is set to this type instead of `ParseError`.
    /// The custom error type must implement `From<ParseError>` for `?` to work.
    pub parse_error: Option<Path>,
    /// Type name for BuildSchema registration in `$types` namespace.
    /// When specified, the type is registered as `$types.<type_name>`.
    /// Example: `#[eure(type_name = "user")]` registers as `$types.user`.
    pub type_name: Option<String>,
    /// Transparent proxy type for types with public fields.
    ///
    /// Generates `FromEure<'doc, TargetType>` using direct struct literal syntax.
    /// Use this when the target type has public fields that match the proxy struct.
    ///
    /// Example:
    /// ```ignore
    /// #[derive(FromEure)]
    /// #[eure(proxy = "external::PublicConfig")]
    /// struct PublicConfigDef {
    ///     host: String,
    ///     port: u16,
    /// }
    /// // Generates: impl FromEure<'doc, external::PublicConfig> for PublicConfigDef
    /// // Uses: external::PublicConfig { host: ..., port: ... }
    /// ```
    pub proxy: Option<Type>,
    /// Opaque proxy type for types with private fields.
    ///
    /// Generates `FromEure<'doc, TargetType>` using From conversion.
    /// Use this when the target type has private fields and requires
    /// `From<ProxyDef> for TargetType` to be implemented.
    ///
    /// Example:
    /// ```ignore
    /// #[derive(FromEure)]
    /// #[eure(opaque = "std::time::Duration")]
    /// struct DurationDef {
    ///     secs: u64,
    ///     nanos: u32,
    /// }
    /// impl From<DurationDef> for std::time::Duration {
    ///     fn from(def: DurationDef) -> Self {
    ///         Duration::new(def.secs, def.nanos)
    ///     }
    /// }
    /// // Generates: impl FromEure<'doc, std::time::Duration> for DurationDef
    /// // Uses: DurationDef { secs: ..., nanos: ... }.into()
    /// ```
    pub opaque: Option<Type>,
}
