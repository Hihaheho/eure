//! Utility functions shared across the crate

use eure_value::value::{Path, PathSegment};

/// Convert a string to camelCase
pub fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;

    for (i, ch) in s.chars().enumerate() {
        if ch == '_' || ch == '-' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(ch.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else if i == 0 {
            result.push(ch.to_lowercase().next().unwrap());
        } else {
            result.push(ch);
        }
    }

    result
}

/// Convert a string to snake_case
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();

    for (i, ch) in s.chars().enumerate() {
        if i > 0 && ch.is_uppercase() {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap());
    }

    result.replace('-', "_")
}

/// Convert a string to PascalCase
pub fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for ch in s.chars() {
        if ch == '_' || ch == '-' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(ch.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }

    result
}

/// Convert a string to kebab-case
pub fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();

    for (i, ch) in s.chars().enumerate() {
        if i > 0 && ch.is_uppercase() {
            result.push('-');
        }
        result.push(ch.to_lowercase().next().unwrap());
    }

    result.replace('_', "-")
}

/// Convert a Path to a string representation
pub fn path_to_string(path: &Path) -> String {
    let mut parts = Vec::new();
    
    for segment in &path.0 {
        match segment {
            PathSegment::Ident(id) => parts.push(id.to_string()),
            PathSegment::Extension(id) => parts.push(format!("${}", id)),
            PathSegment::MetaExt(id) => parts.push(format!("$${}", id)),
            _ => {} // Skip other segment types for now
        }
    }
    
    format!(".{}", parts.join("."))
}

/// Convert path segments to a string for error messages
pub fn path_segments_to_string(segments: &[PathSegment]) -> String {
    segments.iter()
        .map(|s| match s {
            PathSegment::Ident(id) => id.to_string(),
            PathSegment::Extension(id) => format!("${}", id),
            PathSegment::MetaExt(id) => format!("$${}", id),
            _ => String::new(),
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
        assert_eq!(to_camel_case("UPPERCASE"), "uPPERCASE");
        
        // Test snake_case
        assert_eq!(to_snake_case("camelCase"), "camel_case");
        assert_eq!(to_snake_case("PascalCase"), "pascal_case");
        assert_eq!(to_snake_case("kebab-case"), "kebab_case");
        assert_eq!(to_snake_case("UPPERCASE"), "u_p_p_e_r_c_a_s_e");
        
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