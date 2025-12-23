use darling::FromMeta;
use syn::ExprPath;

/// Represents the `#[eure(default)]` or `#[eure(default = "...")]` attribute.
#[derive(Debug, Default, Clone)]
pub enum DefaultValue {
    /// No default attribute present
    #[default]
    None,
    /// `#[eure(default)]` - use Default::default()
    Default,
    /// `#[eure(default = "path::to::fn")]` - call custom function
    Path(ExprPath),
}

impl DefaultValue {
    pub fn is_some(&self) -> bool {
        !matches!(self, DefaultValue::None)
    }
}

impl FromMeta for DefaultValue {
    fn from_none() -> Option<Self> {
        Some(DefaultValue::None)
    }

    fn from_word() -> darling::Result<Self> {
        Ok(DefaultValue::Default)
    }

    fn from_string(value: &str) -> darling::Result<Self> {
        syn::parse_str::<ExprPath>(value)
            .map(DefaultValue::Path)
            .map_err(|e| darling::Error::custom(format!("invalid path: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use darling::FromMeta;
    use syn::parse_quote;

    #[test]
    fn test_from_none() {
        let value = DefaultValue::from_none();
        assert!(matches!(value, Some(DefaultValue::None)));
    }

    #[test]
    fn test_from_word() {
        let value = DefaultValue::from_word().unwrap();
        assert!(matches!(value, DefaultValue::Default));
    }

    #[test]
    fn test_from_string_simple() {
        let value = DefaultValue::from_string("default_value").unwrap();
        assert!(matches!(value, DefaultValue::Path(_)));
    }

    #[test]
    fn test_from_string_path() {
        let value = DefaultValue::from_string("crate::defaults::my_default").unwrap();
        assert!(matches!(value, DefaultValue::Path(_)));
    }

    #[test]
    fn test_is_some() {
        assert!(!DefaultValue::None.is_some());
        assert!(DefaultValue::Default.is_some());
        assert!(DefaultValue::Path(parse_quote!(foo)).is_some());
    }
}
