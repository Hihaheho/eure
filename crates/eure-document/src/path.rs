use core::fmt::Display;

use crate::prelude_internal::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Plural)]
pub struct EurePath(pub Vec<PathSegment>);

impl EurePath {
    /// Create an empty path representing the document root
    pub fn root() -> Self {
        EurePath(Vec::new())
    }

    /// Check if this is the root path
    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathSegment {
    /// Regular identifiers like id, description
    Ident(Identifier),
    /// Extension namespace fields starting with $ like $eure, $variant
    Extension(Identifier),
    /// Arbitrary value used as key
    Value(ObjectKey),
    /// Tuple element index (0-255)
    TupleIndex(u8),
    /// Array element access
    ArrayIndex(Option<usize>),
}

impl Display for EurePath {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.0.is_empty() {
            return write!(f, "(root)");
        }
        for (i, segment) in self.0.iter().enumerate() {
            let is_first = i == 0;
            match segment {
                PathSegment::Ident(id) => {
                    if !is_first {
                        write!(f, ".")?;
                    }
                    write!(f, "{}", id)?;
                }
                PathSegment::Extension(id) => {
                    if !is_first {
                        write!(f, ".")?;
                    }
                    write!(f, "${}", id)?;
                }
                PathSegment::Value(key) => {
                    if !is_first {
                        write!(f, ".")?;
                    }
                    write!(f, "{}", key)?;
                }
                PathSegment::TupleIndex(index) => {
                    if !is_first {
                        write!(f, ".")?;
                    }
                    write!(f, "#{}", index)?;
                }
                PathSegment::ArrayIndex(Some(index)) => write!(f, "[{}]", index)?,
                PathSegment::ArrayIndex(None) => write!(f, "[]")?,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;

    use super::*;
    use crate::value::ObjectKey;

    #[test]
    fn test_display_empty_path() {
        let path = EurePath::root();
        assert_eq!(format!("{}", path), "(root)");
    }

    #[test]
    fn test_display_single_ident() {
        let path = EurePath(vec![PathSegment::Ident(Identifier::new_unchecked("name"))]);
        assert_eq!(format!("{}", path), "name");
    }

    #[test]
    fn test_display_nested_idents() {
        let path = EurePath(vec![
            PathSegment::Ident(Identifier::new_unchecked("a")),
            PathSegment::Ident(Identifier::new_unchecked("b")),
            PathSegment::Ident(Identifier::new_unchecked("c")),
        ]);
        assert_eq!(format!("{}", path), "a.b.c");
    }

    #[test]
    fn test_display_extension() {
        let path = EurePath(vec![PathSegment::Extension(Identifier::new_unchecked(
            "variant",
        ))]);
        assert_eq!(format!("{}", path), "$variant");
    }

    #[test]
    fn test_display_array_index() {
        let path = EurePath(vec![
            PathSegment::Ident(Identifier::new_unchecked("items")),
            PathSegment::ArrayIndex(Some(0)),
        ]);
        assert_eq!(format!("{}", path), "items[0]");
    }

    #[test]
    fn test_display_array_index_none() {
        let path = EurePath(vec![
            PathSegment::Ident(Identifier::new_unchecked("items")),
            PathSegment::ArrayIndex(None),
        ]);
        assert_eq!(format!("{}", path), "items[]");
    }

    #[test]
    fn test_display_tuple_index() {
        let path = EurePath(vec![
            PathSegment::Ident(Identifier::new_unchecked("point")),
            PathSegment::TupleIndex(1),
        ]);
        assert_eq!(format!("{}", path), "point.#1");
    }

    #[test]
    fn test_display_string_key() {
        let path = EurePath(vec![PathSegment::Value(ObjectKey::String(
            "hello".to_string(),
        ))]);
        assert_eq!(format!("{}", path), "\"hello\"");
    }

    #[test]
    fn test_display_string_key_with_spaces() {
        let path = EurePath(vec![PathSegment::Value(ObjectKey::String(
            "hello world".to_string(),
        ))]);
        assert_eq!(format!("{}", path), "\"hello world\"");
    }

    #[test]
    fn test_display_string_key_with_quotes() {
        let path = EurePath(vec![PathSegment::Value(ObjectKey::String(
            "say \"hi\"".to_string(),
        ))]);
        assert_eq!(format!("{}", path), "\"say \\\"hi\\\"\"");
    }

    #[test]
    fn test_display_number_key() {
        let path = EurePath(vec![PathSegment::Value(ObjectKey::Number(42.into()))]);
        assert_eq!(format!("{}", path), "42");
    }

    #[test]
    fn test_display_bool_key() {
        // Boolean identifiers in key position become string keys
        let path = EurePath(vec![PathSegment::Value(ObjectKey::String("true".into()))]);
        assert_eq!(format!("{}", path), "\"true\"");
    }

    #[test]
    fn test_display_complex_path() {
        let path = EurePath(vec![
            PathSegment::Ident(Identifier::new_unchecked("config")),
            PathSegment::Extension(Identifier::new_unchecked("eure")),
            PathSegment::Ident(Identifier::new_unchecked("items")),
            PathSegment::ArrayIndex(Some(0)),
            PathSegment::Value(ObjectKey::String("key with space".to_string())),
        ]);
        assert_eq!(
            format!("{}", path),
            "config.$eure.items[0].\"key with space\""
        );
    }
}
