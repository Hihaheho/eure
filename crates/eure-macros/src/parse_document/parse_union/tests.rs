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
                        .parse_variant::<()>("Unit", |_| Ok(TestEnum::Unit))
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
                        .parse_variant::<()>("Unit", |_| Ok(TestEnum::Unit))
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
                        .parse_variant::<()>("Unit", |_| Ok(TestEnum::Unit))
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
