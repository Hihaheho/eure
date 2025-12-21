use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;

fn generate(input: syn::DeriveInput) -> TokenStream {
    crate::parse_document::derive(crate::create_context(input))
}

#[test]
fn test_named_fields_struct() {
    let input = generate(parse_quote! {
        struct User {
            name: String,
            age: i32,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::ParseDocument<'doc> for User<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let mut rec = ctx.parse_record()?;
                    let value = User {
                        name: rec.parse_field("name")?,
                        age: rec.parse_field("age")?
                    };
                    rec.deny_unknown_fields()?;
                    Ok(value)
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_unit_struct() {
    let input = generate(parse_quote! {
        struct Unit;
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::ParseDocument<'doc> for Unit<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse::<()>()?;
                    Ok(Unit)
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_tuple_struct() {
    let input = generate(parse_quote! {
        struct Point(i32, i32);
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::ParseDocument<'doc> for Point<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let (field_0, field_1,) = ctx.parse::<(i32, i32,)>()?;
                    Ok(Point(field_0, field_1))
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_newtype_struct() {
    let input = generate(parse_quote! {
        struct Name(String);
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::ParseDocument<'doc> for Name<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let field_0 = ctx.parse::<String>()?;
                    Ok(Name(field_0))
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_named_fields_struct_with_custom_crate() {
    let input = generate(parse_quote! {
        #[eure(crate = ::eure_document)]
        struct User {
            name: String,
            age: i32,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure_document::parse::ParseDocument<'doc> for User<> {
                type Error = ::eure_document::parse::ParseError;

                fn parse(ctx: &::eure_document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let mut rec = ctx.parse_record()?;
                    let value = User {
                        name: rec.parse_field("name")?,
                        age: rec.parse_field("age")?
                    };
                    rec.deny_unknown_fields()?;
                    Ok(value)
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_named_fields_struct_with_rename_all_camel_case() {
    let input = generate(parse_quote! {
        #[eure(rename_all = "camelCase")]
        struct User {
            user_name: String,
            email_address: String,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::ParseDocument<'doc> for User<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let mut rec = ctx.parse_record()?;
                    let value = User {
                        user_name: rec.parse_field("userName")?,
                        email_address: rec.parse_field("emailAddress")?
                    };
                    rec.deny_unknown_fields()?;
                    Ok(value)
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_named_fields_struct_with_rename_all_kebab_case() {
    let input = generate(parse_quote! {
        #[eure(rename_all = "kebab-case")]
        struct Config {
            max_retries: i32,
            timeout_seconds: i32,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::ParseDocument<'doc> for Config<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let mut rec = ctx.parse_record()?;
                    let value = Config {
                        max_retries: rec.parse_field("max-retries")?,
                        timeout_seconds: rec.parse_field("timeout-seconds")?
                    };
                    rec.deny_unknown_fields()?;
                    Ok(value)
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_parse_ext_basic() {
    let input = generate(parse_quote! {
        #[eure(parse_ext)]
        struct ExtFields {
            optional: bool,
            deprecated: bool,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::ParseDocument<'doc> for ExtFields<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let mut ext = ctx.parse_extension();
                    let value = ExtFields {
                        optional: ext.parse_ext("optional")?,
                        deprecated: ext.parse_ext("deprecated")?
                    };
                    ext.allow_unknown_extensions();
                    Ok(value)
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_parse_ext_with_rename_all() {
    let input = generate(parse_quote! {
        #[eure(parse_ext, rename_all = "kebab-case")]
        struct ExtFields {
            binding_style: String,
            deny_untagged: bool,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::ParseDocument<'doc> for ExtFields<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let mut ext = ctx.parse_extension();
                    let value = ExtFields {
                        binding_style: ext.parse_ext("binding-style")?,
                        deny_untagged: ext.parse_ext("deny-untagged")?
                    };
                    ext.allow_unknown_extensions();
                    Ok(value)
                }
            }
        }
        .to_string()
    );
}
