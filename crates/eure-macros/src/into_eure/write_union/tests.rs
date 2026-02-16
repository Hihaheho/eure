use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;

fn generate(input: syn::DeriveInput) -> TokenStream {
    crate::into_eure::derive(crate::create_context(input).expect("failed to create context"))
}

#[test]
fn test_unit_variant() {
    let input = generate(parse_quote! {
        enum TestEnum {
            Unit,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::write::IntoEure for TestEnum {
                type Error = ::eure::document::write::WriteError;
                fn write(
                    value: Self,
                    c: &mut ::eure::document::constructor::DocumentConstructor
                ) -> ::core::result::Result<(), Self::Error> {
                    match value {
                        TestEnum::Unit => {
                            c.set_variant("Unit")?;
                            c.bind_primitive(::eure::document::value::PrimitiveValue::Text(
                                ::eure::document::text::Text::plaintext("Unit")
                            ))
                            .map_err(::eure::document::write::WriteError::from)?;
                            Ok(())
                        }
                    }
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_tuple_variant() {
    let input = generate(parse_quote! {
        enum TestEnum {
            Tuple(i32, bool),
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::write::IntoEure for TestEnum {
                type Error = ::eure::document::write::WriteError;
                fn write(
                    value: Self,
                    c: &mut ::eure::document::constructor::DocumentConstructor
                ) -> ::core::result::Result<(), Self::Error> {
                    match value {
                        TestEnum::Tuple(field_0, field_1) => {
                            c.set_variant("Tuple")?;
                            c.tuple(|t| {
                                t.next(field_0)?;
                                t.next(field_1)?;
                                Ok(())
                            })
                        }
                    }
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_tuple_variant_with_via() {
    let input = generate(parse_quote! {
        enum TestEnum {
            Tuple(usize, #[eure(via = "JumpAtProxy")] JumpAt),
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::write::IntoEure for TestEnum {
                type Error = ::eure::document::write::WriteError;
                fn write(
                    value: Self,
                    c: &mut ::eure::document::constructor::DocumentConstructor
                ) -> ::core::result::Result<(), Self::Error> {
                    match value {
                        TestEnum::Tuple(field_0, field_1) => {
                            c.set_variant("Tuple")?;
                            c.tuple(|t| {
                                t.next(field_0)?;
                                t.next_via::<JumpAtProxy, _>(field_1)?;
                                Ok(())
                            })
                        }
                    }
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_struct_variant() {
    let input = generate(parse_quote! {
        enum TestEnum {
            Struct { a: i32, b: bool },
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::write::IntoEure for TestEnum {
                type Error = ::eure::document::write::WriteError;
                fn write(
                    value: Self,
                    c: &mut ::eure::document::constructor::DocumentConstructor
                ) -> ::core::result::Result<(), Self::Error> {
                    match value {
                        TestEnum::Struct { a, b } => {
                            c.set_variant("Struct")?;
                            c.record(|rec| {
                                rec.field("a", a)?;
                                rec.field("b", b)?;
                                Ok(())
                            })
                        }
                    }
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_newtype_variant() {
    let input = generate(parse_quote! {
        enum TestEnum {
            Newtype(String),
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::write::IntoEure for TestEnum {
                type Error = ::eure::document::write::WriteError;
                fn write(
                    value: Self,
                    c: &mut ::eure::document::constructor::DocumentConstructor
                ) -> ::core::result::Result<(), Self::Error> {
                    match value {
                        TestEnum::Newtype(inner) => {
                            c.set_variant("Newtype")?;
                            <String as ::eure::document::write::IntoEure>::write(inner, c)
                        }
                    }
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_newtype_variant_with_via() {
    let input = generate(parse_quote! {
        enum TestEnum {
            Newtype(#[eure(via = "JumpAtProxy")] JumpAt),
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::write::IntoEure for TestEnum {
                type Error = ::eure::document::write::WriteError;
                fn write(
                    value: Self,
                    c: &mut ::eure::document::constructor::DocumentConstructor
                ) -> ::core::result::Result<(), Self::Error> {
                    match value {
                        TestEnum::Newtype(inner) => {
                            c.set_variant("Newtype")?;
                            c.write_via::<JumpAtProxy, _>(inner)
                        }
                    }
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_mixed_variants() {
    let input = generate(parse_quote! {
        enum TestEnum {
            Unit,
            Tuple(i32, bool),
            Struct { a: i32, b: bool },
            Newtype(String),
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::write::IntoEure for TestEnum {
                type Error = ::eure::document::write::WriteError;
                fn write(
                    value: Self,
                    c: &mut ::eure::document::constructor::DocumentConstructor
                ) -> ::core::result::Result<(), Self::Error> {
                    match value {
                        TestEnum::Unit => {
                            c.set_variant("Unit")?;
                            c.bind_primitive(::eure::document::value::PrimitiveValue::Text(
                                ::eure::document::text::Text::plaintext("Unit")
                            ))
                            .map_err(::eure::document::write::WriteError::from)?;
                            Ok(())
                        }
                        TestEnum::Tuple(field_0, field_1) => {
                            c.set_variant("Tuple")?;
                            c.tuple(|t| {
                                t.next(field_0)?;
                                t.next(field_1)?;
                                Ok(())
                            })
                        }
                        TestEnum::Struct { a, b } => {
                            c.set_variant("Struct")?;
                            c.record(|rec| {
                                rec.field("a", a)?;
                                rec.field("b", b)?;
                                Ok(())
                            })
                        }
                        TestEnum::Newtype(inner) => {
                            c.set_variant("Newtype")?;
                            <String as ::eure::document::write::IntoEure>::write(inner, c)
                        }
                    }
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_mixed_variants_with_custom_crate() {
    let input = generate(parse_quote! {
        #[eure(crate = ::eure_document)]
        enum TestEnum {
            Unit,
            Tuple(i32, bool),
            Struct { a: i32, b: bool },
            Newtype(String),
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure_document::write::IntoEure for TestEnum {
                type Error = ::eure_document::write::WriteError;
                fn write(
                    value: Self,
                    c: &mut ::eure_document::constructor::DocumentConstructor
                ) -> ::core::result::Result<(), Self::Error> {
                    match value {
                        TestEnum::Unit => {
                            c.set_variant("Unit")?;
                            c.bind_primitive(::eure_document::value::PrimitiveValue::Text(
                                ::eure_document::text::Text::plaintext("Unit")
                            ))
                            .map_err(::eure_document::write::WriteError::from)?;
                            Ok(())
                        }
                        TestEnum::Tuple(field_0, field_1) => {
                            c.set_variant("Tuple")?;
                            c.tuple(|t| {
                                t.next(field_0)?;
                                t.next(field_1)?;
                                Ok(())
                            })
                        }
                        TestEnum::Struct { a, b } => {
                            c.set_variant("Struct")?;
                            c.record(|rec| {
                                rec.field("a", a)?;
                                rec.field("b", b)?;
                                Ok(())
                            })
                        }
                        TestEnum::Newtype(inner) => {
                            c.set_variant("Newtype")?;
                            <String as ::eure_document::write::IntoEure>::write(inner, c)
                        }
                    }
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_unit_variant_with_rename_all_snake_case() {
    let input = generate(parse_quote! {
        #[eure(rename_all = "snake_case")]
        enum Event {
            UserCreated,
            OrderPlaced,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::write::IntoEure for Event {
                type Error = ::eure::document::write::WriteError;
                fn write(
                    value: Self,
                    c: &mut ::eure::document::constructor::DocumentConstructor
                ) -> ::core::result::Result<(), Self::Error> {
                    match value {
                        Event::UserCreated => {
                            c.set_variant("user_created")?;
                            c.bind_primitive(::eure::document::value::PrimitiveValue::Text(
                                ::eure::document::text::Text::plaintext("user_created")
                            ))
                            .map_err(::eure::document::write::WriteError::from)?;
                            Ok(())
                        }
                        Event::OrderPlaced => {
                            c.set_variant("order_placed")?;
                            c.bind_primitive(::eure::document::value::PrimitiveValue::Text(
                                ::eure::document::text::Text::plaintext("order_placed")
                            ))
                            .map_err(::eure::document::write::WriteError::from)?;
                            Ok(())
                        }
                    }
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_struct_variant_with_rename_all_camel_case() {
    // Container-level rename_all only renames variant names, not struct variant fields
    // (matching serde's behavior)
    let input = generate(parse_quote! {
        #[eure(rename_all = "camelCase")]
        enum Event {
            UserCreated { user_id: i32, created_at: String },
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::write::IntoEure for Event {
                type Error = ::eure::document::write::WriteError;
                fn write(
                    value: Self,
                    c: &mut ::eure::document::constructor::DocumentConstructor
                ) -> ::core::result::Result<(), Self::Error> {
                    match value {
                        Event::UserCreated {
                            user_id,
                            created_at
                        } => {
                            c.set_variant("userCreated")?;
                            c.record(|rec| {
                                rec.field("user_id", user_id)?;
                                rec.field("created_at", created_at)?;
                                Ok(())
                            })
                        }
                    }
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_struct_variant_with_rename_all_fields() {
    // rename_all_fields only renames struct variant fields, not variant names
    let input = generate(parse_quote! {
        #[eure(rename_all_fields = "camelCase")]
        enum Event {
            UserCreated { user_id: i32, created_at: String },
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::write::IntoEure for Event {
                type Error = ::eure::document::write::WriteError;
                fn write(
                    value: Self,
                    c: &mut ::eure::document::constructor::DocumentConstructor
                ) -> ::core::result::Result<(), Self::Error> {
                    match value {
                        Event::UserCreated {
                            user_id,
                            created_at
                        } => {
                            c.set_variant("UserCreated")?;
                            c.record(|rec| {
                                rec.field("userId", user_id)?;
                                rec.field("createdAt", created_at)?;
                                Ok(())
                            })
                        }
                    }
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_struct_variant_with_both_rename_all_and_rename_all_fields() {
    // Both rename_all and rename_all_fields can be used together
    let input = generate(parse_quote! {
        #[eure(rename_all = "snake_case", rename_all_fields = "camelCase")]
        enum Event {
            UserCreated { user_id: i32, created_at: String },
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::write::IntoEure for Event {
                type Error = ::eure::document::write::WriteError;
                fn write(
                    value: Self,
                    c: &mut ::eure::document::constructor::DocumentConstructor
                ) -> ::core::result::Result<(), Self::Error> {
                    match value {
                        Event::UserCreated {
                            user_id,
                            created_at
                        } => {
                            c.set_variant("user_created")?;
                            c.record(|rec| {
                                rec.field("userId", user_id)?;
                                rec.field("createdAt", created_at)?;
                                Ok(())
                            })
                        }
                    }
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_enum_single_type_param() {
    let input = generate(parse_quote! {
        enum Item<T> {
            Normal(T),
            List(Vec<T>),
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<T: ::eure::document::write::IntoEure> ::eure::document::write::IntoEure for Item<T>
            where
                ::eure::document::write::WriteError:
                    ::core::convert::From<<T as ::eure::document::write::IntoEure>::Error>
            {
                type Error = ::eure::document::write::WriteError;
                fn write(
                    value: Self,
                    c: &mut ::eure::document::constructor::DocumentConstructor
                ) -> ::core::result::Result<(), Self::Error> {
                    match value {
                        Item::Normal(inner) => {
                            c.set_variant("Normal")?;
                            <T as ::eure::document::write::IntoEure>::write(inner, c)
                        }
                        Item::List(inner) => {
                            c.set_variant("List")?;
                            <Vec<T> as ::eure::document::write::IntoEure>::write(inner, c)
                        }
                    }
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_enum_multiple_type_params() {
    let input = generate(parse_quote! {
        enum Either<L, R> {
            Left(L),
            Right(R),
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<L: ::eure::document::write::IntoEure, R: ::eure::document::write::IntoEure>
                ::eure::document::write::IntoEure for Either<L, R>
            where
                ::eure::document::write::WriteError:
                    ::core::convert::From<<L as ::eure::document::write::IntoEure>::Error>,
                ::eure::document::write::WriteError:
                    ::core::convert::From<<R as ::eure::document::write::IntoEure>::Error>
            {
                type Error = ::eure::document::write::WriteError;
                fn write(
                    value: Self,
                    c: &mut ::eure::document::constructor::DocumentConstructor
                ) -> ::core::result::Result<(), Self::Error> {
                    match value {
                        Either::Left(inner) => {
                            c.set_variant("Left")?;
                            <L as ::eure::document::write::IntoEure>::write(inner, c)
                        }
                        Either::Right(inner) => {
                            c.set_variant("Right")?;
                            <R as ::eure::document::write::IntoEure>::write(inner, c)
                        }
                    }
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_enum_type_param_with_existing_bounds() {
    let input = generate(parse_quote! {
        enum Item<T: Clone> {
            Normal(T),
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<T: Clone + ::eure::document::write::IntoEure> ::eure::document::write::IntoEure for Item<T>
            where
                ::eure::document::write::WriteError:
                    ::core::convert::From<<T as ::eure::document::write::IntoEure>::Error>
            {
                type Error = ::eure::document::write::WriteError;
                fn write(
                    value: Self,
                    c: &mut ::eure::document::constructor::DocumentConstructor
                ) -> ::core::result::Result<(), Self::Error> {
                    match value {
                        Item::Normal(inner) => {
                            c.set_variant("Normal")?;
                            <T as ::eure::document::write::IntoEure>::write(inner, c)
                        }
                    }
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_struct_variant_with_ext_field() {
    let input = generate(parse_quote! {
        enum Item {
            WithMeta {
                name: String,
                #[eure(ext)]
                meta: MetaData,
            },
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::write::IntoEure for Item {
                type Error = ::eure::document::write::WriteError;
                fn write(
                    value: Self,
                    c: &mut ::eure::document::constructor::DocumentConstructor
                ) -> ::core::result::Result<(), Self::Error> {
                    match value {
                        Item::WithMeta { name, meta } => {
                            c.set_variant("WithMeta")?;
                            c.record(|rec| {
                                rec.field("name", name)?;
                                rec.constructor().set_extension("meta", meta)?;
                                Ok(())
                            })
                        }
                    }
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_opaque_enum() {
    let input = generate(parse_quote! {
        #[eure(opaque = "external::Status")]
        enum StatusDef {
            Active,
            Inactive,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::write::IntoEure<external::Status> for StatusDef {
                type Error = ::eure::document::write::WriteError;
                fn write(
                    value: external::Status,
                    c: &mut ::eure::document::constructor::DocumentConstructor
                ) -> ::core::result::Result<(), Self::Error> {
                    let value: StatusDef = value.into();
                    match value {
                        StatusDef::Active => {
                            c.set_variant("Active")?;
                            c.bind_primitive(::eure::document::value::PrimitiveValue::Text(
                                ::eure::document::text::Text::plaintext("Active")
                            ))
                            .map_err(::eure::document::write::WriteError::from)?;
                            Ok(())
                        }
                        StatusDef::Inactive => {
                            c.set_variant("Inactive")?;
                            c.bind_primitive(::eure::document::value::PrimitiveValue::Text(
                                ::eure::document::text::Text::plaintext("Inactive")
                            ))
                            .map_err(::eure::document::write::WriteError::from)?;
                            Ok(())
                        }
                    }
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_struct_variant_with_via_field() {
    let input = generate(parse_quote! {
        enum Config {
            Timed {
                name: String,
                #[eure(via = "DurationDef")]
                timeout: Duration,
            },
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::write::IntoEure for Config {
                type Error = ::eure::document::write::WriteError;
                fn write(
                    value: Self,
                    c: &mut ::eure::document::constructor::DocumentConstructor
                ) -> ::core::result::Result<(), Self::Error> {
                    match value {
                        Config::Timed { name, timeout } => {
                            c.set_variant("Timed")?;
                            c.record(|rec| {
                                rec.field("name", name)?;
                                rec.field_via::<DurationDef, _>("timeout", timeout)?;
                                Ok(())
                            })
                        }
                    }
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_proxy_enum() {
    // Proxy enum: value type is the target type, so match patterns use target type
    let input = generate(parse_quote! {
        #[eure(proxy = "external::Status")]
        enum StatusDef {
            Active,
            Inactive,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::write::IntoEure<external::Status> for StatusDef {
                type Error = ::eure::document::write::WriteError;
                fn write(
                    value: external::Status,
                    c: &mut ::eure::document::constructor::DocumentConstructor
                ) -> ::core::result::Result<(), Self::Error> {
                    match value {
                        external::Status::Active => {
                            c.set_variant("Active")?;
                            c.bind_primitive(::eure::document::value::PrimitiveValue::Text(
                                ::eure::document::text::Text::plaintext("Active")
                            ))
                            .map_err(::eure::document::write::WriteError::from)?;
                            Ok(())
                        }
                        external::Status::Inactive => {
                            c.set_variant("Inactive")?;
                            c.bind_primitive(::eure::document::value::PrimitiveValue::Text(
                                ::eure::document::text::Text::plaintext("Inactive")
                            ))
                            .map_err(::eure::document::write::WriteError::from)?;
                            Ok(())
                        }
                    }
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_proxy_enum_newtype_variant() {
    // Proxy enum with newtype variant: match patterns use target type
    let input = generate(parse_quote! {
        #[eure(proxy = "external::Value")]
        enum ValueDef {
            Text(String),
            Number(i32),
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::write::IntoEure<external::Value> for ValueDef {
                type Error = ::eure::document::write::WriteError;
                fn write(
                    value: external::Value,
                    c: &mut ::eure::document::constructor::DocumentConstructor
                ) -> ::core::result::Result<(), Self::Error> {
                    match value {
                        external::Value::Text(inner) => {
                            c.set_variant("Text")?;
                            <String as ::eure::document::write::IntoEure>::write(inner, c)
                        }
                        external::Value::Number(inner) => {
                            c.set_variant("Number")?;
                            <i32 as ::eure::document::write::IntoEure>::write(inner, c)
                        }
                    }
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_proxy_enum_non_exhaustive() {
    let input = generate(parse_quote! {
        #[non_exhaustive]
        #[eure(proxy = "external::Status")]
        enum StatusDef {
            Active,
            Inactive,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::write::IntoEure<external::Status> for StatusDef {
                type Error = ::eure::document::write::WriteError;
                fn write(
                    value: external::Status,
                    c: &mut ::eure::document::constructor::DocumentConstructor
                ) -> ::core::result::Result<(), Self::Error> {
                    match value {
                        external::Status::Active => {
                            c.set_variant("Active")?;
                            c.bind_primitive(::eure::document::value::PrimitiveValue::Text(
                                ::eure::document::text::Text::plaintext("Active")
                            ))
                            .map_err(::eure::document::write::WriteError::from)?;
                            Ok(())
                        }
                        external::Status::Inactive => {
                            c.set_variant("Inactive")?;
                            c.bind_primitive(::eure::document::value::PrimitiveValue::Text(
                                ::eure::document::text::Text::plaintext("Inactive")
                            ))
                            .map_err(::eure::document::write::WriteError::from)?;
                            Ok(())
                        }
                        _ => Err(::eure::document::write::WriteError::NonExhaustiveVariant {
                            type_name: ::core::any::type_name::<external::Status>()
                        , }
                        .into())
                    }
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_proxy_enum_non_exhaustive_eure_attr() {
    let input = generate(parse_quote! {
        #[eure(proxy = "external::Status", non_exhaustive)]
        enum StatusDef {
            Active,
            Inactive,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::write::IntoEure<external::Status> for StatusDef {
                type Error = ::eure::document::write::WriteError;
                fn write(
                    value: external::Status,
                    c: &mut ::eure::document::constructor::DocumentConstructor
                ) -> ::core::result::Result<(), Self::Error> {
                    match value {
                        external::Status::Active => {
                            c.set_variant("Active")?;
                            c.bind_primitive(::eure::document::value::PrimitiveValue::Text(
                                ::eure::document::text::Text::plaintext("Active")
                            ))
                            .map_err(::eure::document::write::WriteError::from)?;
                            Ok(())
                        }
                        external::Status::Inactive => {
                            c.set_variant("Inactive")?;
                            c.bind_primitive(::eure::document::value::PrimitiveValue::Text(
                                ::eure::document::text::Text::plaintext("Inactive")
                            ))
                            .map_err(::eure::document::write::WriteError::from)?;
                            Ok(())
                        }
                        _ => Err(::eure::document::write::WriteError::NonExhaustiveVariant {
                            type_name: ::core::any::type_name::<external::Status>()
                        , }
                        .into())
                    }
                }
            }
        }
        .to_string()
    );
}

// ===========================================================================
// Flatten tests for enum struct variants
// ===========================================================================

#[test]
fn test_struct_variant_with_flatten() {
    let input = generate(parse_quote! {
        enum Composite {
            WithAddress {
                name: String,
                #[eure(flatten)]
                address: Address,
            },
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::write::IntoEure for Composite {
                type Error = ::eure::document::write::WriteError;
                fn write(
                    value: Self,
                    c: &mut ::eure::document::constructor::DocumentConstructor
                ) -> ::core::result::Result<(), Self::Error> {
                    match value {
                        Composite::WithAddress { name, address } => {
                            c.set_variant("WithAddress")?;
                            c.record(|rec| {
                                rec.field("name", name)?;
                                rec.flatten::<Address, _>(address)?;
                                Ok(())
                            })
                        }
                    }
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_struct_variant_with_flatten_ext() {
    let input = generate(parse_quote! {
        enum Composite {
            WithExt {
                name: String,
                #[eure(flatten_ext)]
                ext: ExtData,
            },
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::write::IntoEure for Composite {
                type Error = ::eure::document::write::WriteError;
                fn write(
                    value: Self,
                    c: &mut ::eure::document::constructor::DocumentConstructor
                ) -> ::core::result::Result<(), Self::Error> {
                    match value {
                        Composite::WithExt { name, ext } => {
                            c.set_variant("WithExt")?;
                            c.record(|rec| {
                                rec.field("name", name)?;
                                rec.flatten_ext::<ExtData, _>(ext)?;
                                Ok(())
                            })
                        }
                    }
                }
            }
        }
        .to_string()
    );
}
