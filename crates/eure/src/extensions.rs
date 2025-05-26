//! Core extensions for EURE.

pub trait ExtensionNamespace {
    /// The name of the extension namespace.
    fn name(&self) -> &'static str;
    /// Whether the extension namespace is only allowed at the top level of a EURE document.
    fn top_level_only(&self) -> bool;
    /// The types that can be used in the extension namespace.
    fn extension_type(&self) -> ExtensionType;
    /// Parse the extension namespace from a str.
    fn parse(s: &str) -> Option<Self>
    where
        Self: Sized;
}

// assert that ExtensionNamespace is dyn-compatible
type _DynAssertion = Box<dyn ExtensionNamespace>;

pub enum ExtensionType {
    Union(&'static [ExtensionType]),
    Map(&'static [(&'static str, ExtensionType)]),
    String,
    Integer,
    Float,
    Boolean,
    Null,
    Array(Box<ExtensionType>),
    Tuple(&'static [ExtensionType]),
}

pub enum CoreExtension {
    Eure,
    Variant,
}
