//! Configuration for code generation.

/// Visibility of generated types.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Visibility {
    /// Public visibility (`pub`).
    #[default]
    Pub,
    /// Crate-level visibility (`pub(crate)`).
    PubCrate,
    /// Private visibility (no modifier).
    Private,
}

/// Configuration for code generation.
///
/// Use the builder pattern to construct:
/// ```ignore
/// let config = GenerationConfig::builder()
///     .serde_serialize(true)
///     .serde_deserialize(true)
///     .build();
/// ```
#[derive(Debug, Clone, bon::Builder)]
pub struct GenerationConfig {
    /// Generate `EureExtract` implementations.
    #[builder(default = true)]
    pub extract: bool,

    /// Derive `serde::Serialize` for generated types.
    #[builder(default = true)]
    pub serde_serialize: bool,

    /// Derive `serde::Deserialize` for generated types.
    #[builder(default = true)]
    pub serde_deserialize: bool,

    /// Visibility of generated types.
    #[builder(default)]
    pub visibility: Visibility,

    /// Add `#[allow(...)]` attributes to suppress warnings.
    #[builder(default = true)]
    pub allow_warnings: bool,
}
