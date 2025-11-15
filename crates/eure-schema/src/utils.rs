//! Utility functions shared across the crate

use convert_case::{Case, Casing};
use eure_value::value::{EurePath, PathSegment};

/// Convert a string to camelCase
pub fn to_camel_case(s: &str) -> String {
    s.to_case(Case::Camel)
}

/// Convert a string to snake_case
pub fn to_snake_case(s: &str) -> String {
    s.to_case(Case::Snake)
}

/// Convert a string to PascalCase
pub fn to_pascal_case(s: &str) -> String {
    s.to_case(Case::Pascal)
}

/// Convert a string to kebab-case
pub fn to_kebab_case(s: &str) -> String {
    s.to_case(Case::Kebab)
}

/// Convert a Path to a string representation for display purposes only
/// Note: This is only for error messages and debugging, not for lookups
pub fn path_to_display_string(path: &EurePath) -> String {
    let mut parts = Vec::new();

    for segment in &path.0 {
        match segment {
            PathSegment::Ident(id) => parts.push(id.to_string()),
            PathSegment::Extension(id) => parts.push(format!("${id}")),
            PathSegment::MetaExt(id) => parts.push(format!("$${id}")),
            PathSegment::TupleIndex(idx) => parts.push(format!("[{idx}]")),
            PathSegment::Value(val) => parts.push(format!("{val:?}")),
            PathSegment::ArrayIndex(idx) => {
                if let Some(index) = *idx {
                    parts.push(format!("[{index}]"));
                } else {
                    parts.push("[]".to_string());
                }
            }
        }
    }

    format!(".{}", parts.join("."))
}

/// Convert path segments to a string for error messages only
/// Note: This is only for display purposes, not for lookups
pub fn path_segments_to_display_string(segments: &[PathSegment]) -> String {
    segments
        .iter()
        .map(|s| match s {
            PathSegment::Ident(id) => id.to_string(),
            PathSegment::Extension(id) => format!("${id}"),
            PathSegment::MetaExt(id) => format!("$${id}"),
            PathSegment::TupleIndex(idx) => format!("[{idx}]"),
            PathSegment::Value(val) => format!("{val:?}"),
            PathSegment::ArrayIndex(idx) => {
                if let Some(index) = *idx {
                    format!("[{index}]")
                } else {
                    "[]".to_string()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_conversions() {
        // Test camelCase
        assert_eq!(to_camel_case("snake_case"), "snakeCase");
        assert_eq!(to_camel_case("kebab-case"), "kebabCase");
        assert_eq!(to_camel_case("PascalCase"), "pascalCase");
        assert_eq!(to_camel_case("UPPERCASE"), "uppercase");

        // Test snake_case
        assert_eq!(to_snake_case("camelCase"), "camel_case");
        assert_eq!(to_snake_case("PascalCase"), "pascal_case");
        assert_eq!(to_snake_case("kebab-case"), "kebab_case");
        assert_eq!(to_snake_case("UPPERCASE"), "uppercase");

        // Test PascalCase
        assert_eq!(to_pascal_case("snake_case"), "SnakeCase");
        assert_eq!(to_pascal_case("camelCase"), "CamelCase");
        assert_eq!(to_pascal_case("kebab-case"), "KebabCase");

        // Test kebab-case
        assert_eq!(to_kebab_case("camelCase"), "camel-case");
        assert_eq!(to_kebab_case("PascalCase"), "pascal-case");
        assert_eq!(to_kebab_case("snake_case"), "snake-case");
    }
}
