use darling::FromField;
use syn::Type;

use super::DefaultValue;

#[derive(Debug, Default, FromField)]
#[darling(default, attributes(eure))]
pub struct FieldAttrs {
    /// Flatten a nested record type (shares record field access tracking).
    /// Only valid in non-parse_ext context.
    pub flatten: bool,
    /// Flatten a nested extension type (shares extension access tracking).
    /// Valid in both parse_ext and non-parse_ext contexts.
    pub flatten_ext: bool,
    /// Parse this field from extensions instead of record fields.
    /// Only valid in non-parse_ext context.
    pub ext: bool,
    /// Use default value when field is missing.
    /// - `#[eure(default)]` uses `Default::default()`
    /// - `#[eure(default = "path::to::fn")]` calls custom function
    pub default: DefaultValue,
    /// Explicit rename for this field (overrides rename_all/rename_all_fields)
    pub rename: Option<String>,
    /// Parse this field using a strategy type that implements `FromEure<'doc, T>`.
    ///
    /// This enables parsing remote types (types from external crates) that can't
    /// implement `FromEure` directly due to Rust's orphan rule.
    ///
    /// Example:
    /// ```ignore
    /// #[eure(via = "DurationDef")]
    /// timeout: Duration,
    ///
    /// #[eure(via = "Option<DurationDef>")]
    /// optional_timeout: Option<Duration>,
    /// ```
    pub via: Option<Type>,
}
