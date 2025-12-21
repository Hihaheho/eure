use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;

fn generate(input: syn::DeriveInput) -> TokenStream {
    crate::parse_document::derive(crate::create_context(input))
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
            impl<'doc,> ::eure::document::parse::ParseDocument<'doc> for TestEnum<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .variant("Unit", ::eure::document::parse::DocumentParserExt::map(::eure::document::parse::VariantLiteralParser("Unit"), |_| TestEnum::Unit))
                        .parse()
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
            impl<'doc,> ::eure::document::parse::ParseDocument<'doc> for TestEnum<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .parse_variant::<(i32, bool,)>("Tuple", |(field_0, field_1,)| Ok(TestEnum::Tuple(field_0, field_1)))
                        .parse()
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
            impl<'doc,> ::eure::document::parse::ParseDocument<'doc> for TestEnum<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .variant("Struct", |ctx: &::eure::document::parse::ParseContext<'_>| {
                            let mut rec = ctx.parse_record()?;
                            let value = TestEnum::Struct {
                                a: rec.parse_field("a")?,
                                b: rec.parse_field("b")?
                            };
                            rec.deny_unknown_fields()?;
                            Ok(value)
                        })
                        .parse()
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
            impl<'doc,> ::eure::document::parse::ParseDocument<'doc> for TestEnum<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .parse_variant::<String>("Newtype", |field_0| Ok(TestEnum::Newtype(field_0)))
                        .parse()
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
            impl<'doc,> ::eure::document::parse::ParseDocument<'doc> for TestEnum<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .variant("Unit", ::eure::document::parse::DocumentParserExt::map(::eure::document::parse::VariantLiteralParser("Unit"), |_| TestEnum::Unit))
                        .parse_variant::<(i32, bool,)>("Tuple", |(field_0, field_1,)| Ok(TestEnum::Tuple(field_0, field_1)))
                        .variant("Struct", |ctx: &::eure::document::parse::ParseContext<'_>| {
                            let mut rec = ctx.parse_record()?;
                            let value = TestEnum::Struct {
                                a: rec.parse_field("a")?,
                                b: rec.parse_field("b")?
                            };
                            rec.deny_unknown_fields()?;
                            Ok(value)
                        })
                        .parse_variant::<String>("Newtype", |field_0| Ok(TestEnum::Newtype(field_0)))
                        .parse()
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
            impl<'doc,> ::eure_document::parse::ParseDocument<'doc> for TestEnum<> {
                type Error = ::eure_document::parse::ParseError;

                fn parse(ctx: &::eure_document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure_document::data_model::VariantRepr::default())?
                        .variant("Unit", ::eure_document::parse::DocumentParserExt::map(::eure_document::parse::VariantLiteralParser("Unit"), |_| TestEnum::Unit))
                        .parse_variant::<(i32, bool,)>("Tuple", |(field_0, field_1,)| Ok(TestEnum::Tuple(field_0, field_1)))
                        .variant("Struct", |ctx: &::eure_document::parse::ParseContext<'_>| {
                            let mut rec = ctx.parse_record()?;
                            let value = TestEnum::Struct {
                                a: rec.parse_field("a")?,
                                b: rec.parse_field("b")?
                            };
                            rec.deny_unknown_fields()?;
                            Ok(value)
                        })
                        .parse_variant::<String>("Newtype", |field_0| Ok(TestEnum::Newtype(field_0)))
                        .parse()
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
            impl<'doc,> ::eure::document::parse::ParseDocument<'doc> for Event<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .variant("user_created", ::eure::document::parse::DocumentParserExt::map(::eure::document::parse::VariantLiteralParser("user_created"), |_| Event::UserCreated))
                        .variant("order_placed", ::eure::document::parse::DocumentParserExt::map(::eure::document::parse::VariantLiteralParser("order_placed"), |_| Event::OrderPlaced))
                        .parse()
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
            impl<'doc,> ::eure::document::parse::ParseDocument<'doc> for Event<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .variant("userCreated", |ctx: &::eure::document::parse::ParseContext<'_>| {
                            let mut rec = ctx.parse_record()?;
                            let value = Event::UserCreated {
                                user_id: rec.parse_field("user_id")?,
                                created_at: rec.parse_field("created_at")?
                            };
                            rec.deny_unknown_fields()?;
                            Ok(value)
                        })
                        .parse()
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
            impl<'doc,> ::eure::document::parse::ParseDocument<'doc> for Event<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .variant("UserCreated", |ctx: &::eure::document::parse::ParseContext<'_>| {
                            let mut rec = ctx.parse_record()?;
                            let value = Event::UserCreated {
                                user_id: rec.parse_field("userId")?,
                                created_at: rec.parse_field("createdAt")?
                            };
                            rec.deny_unknown_fields()?;
                            Ok(value)
                        })
                        .parse()
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
            impl<'doc,> ::eure::document::parse::ParseDocument<'doc> for Event<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .variant("user_created", |ctx: &::eure::document::parse::ParseContext<'_>| {
                            let mut rec = ctx.parse_record()?;
                            let value = Event::UserCreated {
                                user_id: rec.parse_field("userId")?,
                                created_at: rec.parse_field("createdAt")?
                            };
                            rec.deny_unknown_fields()?;
                            Ok(value)
                        })
                        .parse()
                }
            }
        }
        .to_string()
    );
}
