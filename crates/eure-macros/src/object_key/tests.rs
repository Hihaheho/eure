use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;

fn generate(input: syn::DeriveInput) -> TokenStream {
    crate::object_key::derive(crate::create_context(input).expect("failed to create context"))
}

#[test]
fn test_basic_enum() {
    let input = generate(parse_quote! {
        enum Direction {
            North,
            South,
            East,
            West,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::parse::ParseObjectKey<'_> for Direction {
                fn from_object_key(key: &::eure::document::value::ObjectKey) -> Result<Self, ::eure::document::parse::ParseErrorKind> {
                    match key {
                        ::eure::document::value::ObjectKey::String(s) => match s.as_str() {
                            "North" => Ok(Direction::North),
                            "South" => Ok(Direction::South),
                            "East" => Ok(Direction::East),
                            "West" => Ok(Direction::West),
                            other => Err(::eure::document::parse::ParseErrorKind::UnknownVariant(other.to_string())),
                        },
                        _ => Err(::eure::document::parse::ParseErrorKind::TypeMismatch {
                            expected: ::eure::document::value::ValueKind::Text,
                            actual: match key {
                                ::eure::document::value::ObjectKey::Number(_) => ::eure::document::value::ValueKind::Integer,
                                ::eure::document::value::ObjectKey::String(_) => ::eure::document::value::ValueKind::Text,
                                ::eure::document::value::ObjectKey::Tuple(_) => ::eure::document::value::ValueKind::Tuple,
                            },
                        }),
                    }
                }

                fn from_extension_ident(ident: &::eure::document::identifier::Identifier) -> Result<Self, ::eure::document::parse::ParseErrorKind> {
                    match ident.as_ref() {
                        "North" => Ok(Direction::North),
                        "South" => Ok(Direction::South),
                        "East" => Ok(Direction::East),
                        "West" => Ok(Direction::West),
                        other => Err(::eure::document::parse::ParseErrorKind::UnknownVariant(other.to_string())),
                    }
                }
            }

            impl From<Direction> for ::eure::document::value::ObjectKey {
                fn from(value: Direction) -> Self {
                    ::eure::document::value::ObjectKey::String(match value {
                        Direction::North => "North",
                        Direction::South => "South",
                        Direction::East => "East",
                        Direction::West => "West",
                    }.to_string())
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_rename_all_snake_case() {
    let input = generate(parse_quote! {
        #[eure(rename_all = "snake_case")]
        enum HttpMethod {
            GetAll,
            PostNew,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::parse::ParseObjectKey<'_> for HttpMethod {
                fn from_object_key(key: &::eure::document::value::ObjectKey) -> Result<Self, ::eure::document::parse::ParseErrorKind> {
                    match key {
                        ::eure::document::value::ObjectKey::String(s) => match s.as_str() {
                            "get_all" => Ok(HttpMethod::GetAll),
                            "post_new" => Ok(HttpMethod::PostNew),
                            other => Err(::eure::document::parse::ParseErrorKind::UnknownVariant(other.to_string())),
                        },
                        _ => Err(::eure::document::parse::ParseErrorKind::TypeMismatch {
                            expected: ::eure::document::value::ValueKind::Text,
                            actual: match key {
                                ::eure::document::value::ObjectKey::Number(_) => ::eure::document::value::ValueKind::Integer,
                                ::eure::document::value::ObjectKey::String(_) => ::eure::document::value::ValueKind::Text,
                                ::eure::document::value::ObjectKey::Tuple(_) => ::eure::document::value::ValueKind::Tuple,
                            },
                        }),
                    }
                }

                fn from_extension_ident(ident: &::eure::document::identifier::Identifier) -> Result<Self, ::eure::document::parse::ParseErrorKind> {
                    match ident.as_ref() {
                        "get_all" => Ok(HttpMethod::GetAll),
                        "post_new" => Ok(HttpMethod::PostNew),
                        other => Err(::eure::document::parse::ParseErrorKind::UnknownVariant(other.to_string())),
                    }
                }
            }

            impl From<HttpMethod> for ::eure::document::value::ObjectKey {
                fn from(value: HttpMethod) -> Self {
                    ::eure::document::value::ObjectKey::String(match value {
                        HttpMethod::GetAll => "get_all",
                        HttpMethod::PostNew => "post_new",
                    }.to_string())
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_variant_rename() {
    let input = generate(parse_quote! {
        enum Color {
            #[eure(rename = "red-color")]
            Red,
            Blue,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::parse::ParseObjectKey<'_> for Color {
                fn from_object_key(key: &::eure::document::value::ObjectKey) -> Result<Self, ::eure::document::parse::ParseErrorKind> {
                    match key {
                        ::eure::document::value::ObjectKey::String(s) => match s.as_str() {
                            "red-color" => Ok(Color::Red),
                            "Blue" => Ok(Color::Blue),
                            other => Err(::eure::document::parse::ParseErrorKind::UnknownVariant(other.to_string())),
                        },
                        _ => Err(::eure::document::parse::ParseErrorKind::TypeMismatch {
                            expected: ::eure::document::value::ValueKind::Text,
                            actual: match key {
                                ::eure::document::value::ObjectKey::Number(_) => ::eure::document::value::ValueKind::Integer,
                                ::eure::document::value::ObjectKey::String(_) => ::eure::document::value::ValueKind::Text,
                                ::eure::document::value::ObjectKey::Tuple(_) => ::eure::document::value::ValueKind::Tuple,
                            },
                        }),
                    }
                }

                fn from_extension_ident(ident: &::eure::document::identifier::Identifier) -> Result<Self, ::eure::document::parse::ParseErrorKind> {
                    match ident.as_ref() {
                        "red-color" => Ok(Color::Red),
                        "Blue" => Ok(Color::Blue),
                        other => Err(::eure::document::parse::ParseErrorKind::UnknownVariant(other.to_string())),
                    }
                }
            }

            impl From<Color> for ::eure::document::value::ObjectKey {
                fn from(value: Color) -> Self {
                    ::eure::document::value::ObjectKey::String(match value {
                        Color::Red => "red-color",
                        Color::Blue => "Blue",
                    }.to_string())
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_rename_overrides_rename_all() {
    let input = generate(parse_quote! {
        #[eure(rename_all = "snake_case")]
        enum Status {
            #[eure(rename = "ACTIVE")]
            IsActive,
            IsInactive,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::parse::ParseObjectKey<'_> for Status {
                fn from_object_key(key: &::eure::document::value::ObjectKey) -> Result<Self, ::eure::document::parse::ParseErrorKind> {
                    match key {
                        ::eure::document::value::ObjectKey::String(s) => match s.as_str() {
                            "ACTIVE" => Ok(Status::IsActive),
                            "is_inactive" => Ok(Status::IsInactive),
                            other => Err(::eure::document::parse::ParseErrorKind::UnknownVariant(other.to_string())),
                        },
                        _ => Err(::eure::document::parse::ParseErrorKind::TypeMismatch {
                            expected: ::eure::document::value::ValueKind::Text,
                            actual: match key {
                                ::eure::document::value::ObjectKey::Number(_) => ::eure::document::value::ValueKind::Integer,
                                ::eure::document::value::ObjectKey::String(_) => ::eure::document::value::ValueKind::Text,
                                ::eure::document::value::ObjectKey::Tuple(_) => ::eure::document::value::ValueKind::Tuple,
                            },
                        }),
                    }
                }

                fn from_extension_ident(ident: &::eure::document::identifier::Identifier) -> Result<Self, ::eure::document::parse::ParseErrorKind> {
                    match ident.as_ref() {
                        "ACTIVE" => Ok(Status::IsActive),
                        "is_inactive" => Ok(Status::IsInactive),
                        other => Err(::eure::document::parse::ParseErrorKind::UnknownVariant(other.to_string())),
                    }
                }
            }

            impl From<Status> for ::eure::document::value::ObjectKey {
                fn from(value: Status) -> Self {
                    ::eure::document::value::ObjectKey::String(match value {
                        Status::IsActive => "ACTIVE",
                        Status::IsInactive => "is_inactive",
                    }.to_string())
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_custom_crate_path() {
    let input = generate(parse_quote! {
        #[eure(crate = ::eure_document)]
        enum Side {
            Left,
            Right,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure_document::parse::ParseObjectKey<'_> for Side {
                fn from_object_key(key: &::eure_document::value::ObjectKey) -> Result<Self, ::eure_document::parse::ParseErrorKind> {
                    match key {
                        ::eure_document::value::ObjectKey::String(s) => match s.as_str() {
                            "Left" => Ok(Side::Left),
                            "Right" => Ok(Side::Right),
                            other => Err(::eure_document::parse::ParseErrorKind::UnknownVariant(other.to_string())),
                        },
                        _ => Err(::eure_document::parse::ParseErrorKind::TypeMismatch {
                            expected: ::eure_document::value::ValueKind::Text,
                            actual: match key {
                                ::eure_document::value::ObjectKey::Number(_) => ::eure_document::value::ValueKind::Integer,
                                ::eure_document::value::ObjectKey::String(_) => ::eure_document::value::ValueKind::Text,
                                ::eure_document::value::ObjectKey::Tuple(_) => ::eure_document::value::ValueKind::Tuple,
                            },
                        }),
                    }
                }

                fn from_extension_ident(ident: &::eure_document::identifier::Identifier) -> Result<Self, ::eure_document::parse::ParseErrorKind> {
                    match ident.as_ref() {
                        "Left" => Ok(Side::Left),
                        "Right" => Ok(Side::Right),
                        other => Err(::eure_document::parse::ParseErrorKind::UnknownVariant(other.to_string())),
                    }
                }
            }

            impl From<Side> for ::eure_document::value::ObjectKey {
                fn from(value: Side) -> Self {
                    ::eure_document::value::ObjectKey::String(match value {
                        Side::Left => "Left",
                        Side::Right => "Right",
                    }.to_string())
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_single_variant() {
    let input = generate(parse_quote! {
        enum Only {
            One,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::parse::ParseObjectKey<'_> for Only {
                fn from_object_key(key: &::eure::document::value::ObjectKey) -> Result<Self, ::eure::document::parse::ParseErrorKind> {
                    match key {
                        ::eure::document::value::ObjectKey::String(s) => match s.as_str() {
                            "One" => Ok(Only::One),
                            other => Err(::eure::document::parse::ParseErrorKind::UnknownVariant(other.to_string())),
                        },
                        _ => Err(::eure::document::parse::ParseErrorKind::TypeMismatch {
                            expected: ::eure::document::value::ValueKind::Text,
                            actual: match key {
                                ::eure::document::value::ObjectKey::Number(_) => ::eure::document::value::ValueKind::Integer,
                                ::eure::document::value::ObjectKey::String(_) => ::eure::document::value::ValueKind::Text,
                                ::eure::document::value::ObjectKey::Tuple(_) => ::eure::document::value::ValueKind::Tuple,
                            },
                        }),
                    }
                }

                fn from_extension_ident(ident: &::eure::document::identifier::Identifier) -> Result<Self, ::eure::document::parse::ParseErrorKind> {
                    match ident.as_ref() {
                        "One" => Ok(Only::One),
                        other => Err(::eure::document::parse::ParseErrorKind::UnknownVariant(other.to_string())),
                    }
                }
            }

            impl From<Only> for ::eure::document::value::ObjectKey {
                fn from(value: Only) -> Self {
                    ::eure::document::value::ObjectKey::String(match value {
                        Only::One => "One",
                    }.to_string())
                }
            }
        }
        .to_string()
    );
}
