use convert_case::Case;
use darling::FromMeta;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameAll {
    Lower,
    Upper,
    Pascal,
    Camel,
    Snake,
    ScreamingSnake,
    Kebab,
    Cobol,
}

impl RenameAll {
    pub fn to_case(self) -> Case<'static> {
        match self {
            RenameAll::Lower => Case::Flat,
            RenameAll::Upper => Case::UpperFlat,
            RenameAll::Pascal => Case::Pascal,
            RenameAll::Camel => Case::Camel,
            RenameAll::Snake => Case::Snake,
            RenameAll::ScreamingSnake => Case::UpperSnake,
            RenameAll::Kebab => Case::Kebab,
            RenameAll::Cobol => Case::Cobol,
        }
    }
}

impl FromMeta for RenameAll {
    fn from_string(value: &str) -> darling::Result<Self> {
        match value {
            "lowercase" => Ok(RenameAll::Lower),
            "UPPERCASE" => Ok(RenameAll::Upper),
            "PascalCase" => Ok(RenameAll::Pascal),
            "camelCase" => Ok(RenameAll::Camel),
            "snake_case" => Ok(RenameAll::Snake),
            "SCREAMING_SNAKE_CASE" => Ok(RenameAll::ScreamingSnake),
            "kebab-case" => Ok(RenameAll::Kebab),
            "SCREAMING-KEBAB-CASE" => Ok(RenameAll::Cobol),
            _ => Err(darling::Error::unknown_value(value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use convert_case::Casing as _;

    #[test]
    fn test_from_meta_lowercase() {
        let rename = RenameAll::from_string("lowercase").unwrap();
        assert_eq!(rename, RenameAll::Lower);
        assert_eq!("UserName".to_case(rename.to_case()), "username");
    }

    #[test]
    fn test_from_meta_uppercase() {
        let rename = RenameAll::from_string("UPPERCASE").unwrap();
        assert_eq!(rename, RenameAll::Upper);
        assert_eq!("userName".to_case(rename.to_case()), "USERNAME");
    }

    #[test]
    fn test_from_meta_pascal_case() {
        let rename = RenameAll::from_string("PascalCase").unwrap();
        assert_eq!(rename, RenameAll::Pascal);
        assert_eq!("user_name".to_case(rename.to_case()), "UserName");
    }

    #[test]
    fn test_from_meta_camel_case() {
        let rename = RenameAll::from_string("camelCase").unwrap();
        assert_eq!(rename, RenameAll::Camel);
        assert_eq!("user_name".to_case(rename.to_case()), "userName");
    }

    #[test]
    fn test_from_meta_snake_case() {
        let rename = RenameAll::from_string("snake_case").unwrap();
        assert_eq!(rename, RenameAll::Snake);
        assert_eq!("UserName".to_case(rename.to_case()), "user_name");
    }

    #[test]
    fn test_from_meta_screaming_snake_case() {
        let rename = RenameAll::from_string("SCREAMING_SNAKE_CASE").unwrap();
        assert_eq!(rename, RenameAll::ScreamingSnake);
        assert_eq!("userName".to_case(rename.to_case()), "USER_NAME");
    }

    #[test]
    fn test_from_meta_kebab_case() {
        let rename = RenameAll::from_string("kebab-case").unwrap();
        assert_eq!(rename, RenameAll::Kebab);
        assert_eq!("userName".to_case(rename.to_case()), "user-name");
    }

    #[test]
    fn test_from_meta_screaming_kebab_case() {
        let rename = RenameAll::from_string("SCREAMING-KEBAB-CASE").unwrap();
        assert_eq!(rename, RenameAll::Cobol);
        assert_eq!("userName".to_case(rename.to_case()), "USER-NAME");
    }

    #[test]
    fn test_from_meta_unknown_value() {
        let result = RenameAll::from_string("unknown");
        assert!(result.is_err());
    }
}
