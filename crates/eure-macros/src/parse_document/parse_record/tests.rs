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
                    let rec = ctx.parse_record()?;
                    let value = User {
                        name: rec.parse_field("name")?,
                        age: rec.parse_field("age")?
                    };
                    rec.deny_unknown_fields()?;
                    ctx.deny_unknown_extensions()?;
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
                    let rec = ctx.parse_record()?;
                    let value = User {
                        name: rec.parse_field("name")?,
                        age: rec.parse_field("age")?
                    };
                    rec.deny_unknown_fields()?;
                    ctx.deny_unknown_extensions()?;
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
                    let rec = ctx.parse_record()?;
                    let value = User {
                        user_name: rec.parse_field("userName")?,
                        email_address: rec.parse_field("emailAddress")?
                    };
                    rec.deny_unknown_fields()?;
                    ctx.deny_unknown_extensions()?;
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
                    let rec = ctx.parse_record()?;
                    let value = Config {
                        max_retries: rec.parse_field("max-retries")?,
                        timeout_seconds: rec.parse_field("timeout-seconds")?
                    };
                    rec.deny_unknown_fields()?;
                    ctx.deny_unknown_extensions()?;
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
                    let value = ExtFields {
                        optional: ctx.parse_ext("optional")?,
                        deprecated: ctx.parse_ext("deprecated")?
                    };
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
                    let value = ExtFields {
                        binding_style: ctx.parse_ext("binding-style")?,
                        deny_untagged: ctx.parse_ext("deny-untagged")?
                    };
                    Ok(value)
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_flatten_field() {
    let input = generate(parse_quote! {
        struct Person {
            name: String,
            #[eure(flatten)]
            address: Address,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::ParseDocument<'doc> for Person<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = Person {
                        name: rec.parse_field("name")?,
                        address: <Address>::parse(&rec.flatten())?
                    };
                    rec.deny_unknown_fields()?;
                    ctx.deny_unknown_extensions()?;
                    Ok(value)
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_multiple_flatten_fields() {
    let input = generate(parse_quote! {
        struct Combined {
            id: i32,
            #[eure(flatten)]
            personal: PersonalInfo,
            #[eure(flatten)]
            contact: ContactInfo,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::ParseDocument<'doc> for Combined<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = Combined {
                        id: rec.parse_field("id")?,
                        personal: <PersonalInfo>::parse(&rec.flatten())?,
                        contact: <ContactInfo>::parse(&rec.flatten())?
                    };
                    rec.deny_unknown_fields()?;
                    ctx.deny_unknown_extensions()?;
                    Ok(value)
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_flatten_with_rename_all() {
    let input = generate(parse_quote! {
        #[eure(rename_all = "camelCase")]
        struct Person {
            full_name: String,
            #[eure(flatten)]
            address_info: AddressInfo,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::ParseDocument<'doc> for Person<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = Person {
                        full_name: rec.parse_field("fullName")?,
                        address_info: <AddressInfo>::parse(&rec.flatten())?
                    };
                    rec.deny_unknown_fields()?;
                    ctx.deny_unknown_extensions()?;
                    Ok(value)
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_flatten_ext_field() {
    let input = generate(parse_quote! {
        struct Person {
            name: String,
            #[eure(ext)]
            optional: bool,
            #[eure(flatten_ext)]
            address: ExtAddress,
            #[eure(flatten_ext)]
            contact: ExtContact,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::ParseDocument<'doc> for Person<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = Person {
                        name: rec.parse_field("name")?,
                        optional: ctx.parse_ext("optional")?,
                        address: <ExtAddress>::parse(&ctx.flatten_ext())?,
                        contact: <ExtContact>::parse(&ctx.flatten_ext())?
                    };
                    rec.deny_unknown_fields()?;
                    ctx.deny_unknown_extensions()?;
                    Ok(value)
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_custom_parse_error() {
    let input = generate(parse_quote! {
        #[eure(parse_error = MyCustomError)]
        struct User {
            name: String,
            age: i32,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::ParseDocument<'doc> for User<> {
                type Error = MyCustomError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = User {
                        name: rec.parse_field("name")?,
                        age: rec.parse_field("age")?
                    };
                    rec.deny_unknown_fields()?;
                    ctx.deny_unknown_extensions()?;
                    Ok(value)
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_custom_parse_error_with_path() {
    let input = generate(parse_quote! {
        #[eure(parse_error = crate::errors::MyCustomError)]
        struct User {
            name: String,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::ParseDocument<'doc> for User<> {
                type Error = crate::errors::MyCustomError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = User {
                        name: rec.parse_field("name")?
                    };
                    rec.deny_unknown_fields()?;
                    ctx.deny_unknown_extensions()?;
                    Ok(value)
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_custom_parse_error_with_custom_crate() {
    let input = generate(parse_quote! {
        #[eure(crate = ::eure_document, parse_error = MyError)]
        struct User {
            name: String,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure_document::parse::ParseDocument<'doc> for User<> {
                type Error = MyError;

                fn parse(ctx: &::eure_document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = User {
                        name: rec.parse_field("name")?
                    };
                    rec.deny_unknown_fields()?;
                    ctx.deny_unknown_extensions()?;
                    Ok(value)
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_single_type_param() {
    let input = generate(parse_quote! {
        struct Wrapper<T> {
            inner: T,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc, T: ::eure::document::parse::ParseDocument<'doc, Error = ::eure::document::parse::ParseError> > ::eure::document::parse::ParseDocument<'doc> for Wrapper<T> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = Wrapper {
                        inner: rec.parse_field("inner")?
                    };
                    rec.deny_unknown_fields()?;
                    ctx.deny_unknown_extensions()?;
                    Ok(value)
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_multiple_type_params() {
    let input = generate(parse_quote! {
        struct Pair<A, B> {
            first: A,
            second: B,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc, A: ::eure::document::parse::ParseDocument<'doc, Error = ::eure::document::parse::ParseError>, B: ::eure::document::parse::ParseDocument<'doc, Error = ::eure::document::parse::ParseError> > ::eure::document::parse::ParseDocument<'doc> for Pair<A, B> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = Pair {
                        first: rec.parse_field("first")?,
                        second: rec.parse_field("second")?
                    };
                    rec.deny_unknown_fields()?;
                    ctx.deny_unknown_extensions()?;
                    Ok(value)
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_type_param_with_existing_bounds() {
    let input = generate(parse_quote! {
        struct Wrapper<T: Clone> {
            inner: T,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc, T: Clone + ::eure::document::parse::ParseDocument<'doc, Error = ::eure::document::parse::ParseError> > ::eure::document::parse::ParseDocument<'doc> for Wrapper<T> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = Wrapper {
                        inner: rec.parse_field("inner")?
                    };
                    rec.deny_unknown_fields()?;
                    ctx.deny_unknown_extensions()?;
                    Ok(value)
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_type_param_with_custom_error() {
    let input = generate(parse_quote! {
        #[eure(parse_error = MyError)]
        struct Wrapper<T> {
            inner: T,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc, T: ::eure::document::parse::ParseDocument<'doc> > ::eure::document::parse::ParseDocument<'doc> for Wrapper<T>
            where
                MyError: From<<T as ::eure::document::parse::ParseDocument<'doc>>::Error>
            {
                type Error = MyError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = Wrapper {
                        inner: rec.parse_field("inner")?
                    };
                    rec.deny_unknown_fields()?;
                    ctx.deny_unknown_extensions()?;
                    Ok(value)
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_multiple_type_params_with_custom_error() {
    let input = generate(parse_quote! {
        #[eure(parse_error = MyError)]
        struct Pair<A, B> {
            first: A,
            second: B,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc, A: ::eure::document::parse::ParseDocument<'doc>, B: ::eure::document::parse::ParseDocument<'doc> > ::eure::document::parse::ParseDocument<'doc> for Pair<A, B>
            where
                MyError: From<<A as ::eure::document::parse::ParseDocument<'doc>>::Error>,
                MyError: From<<B as ::eure::document::parse::ParseDocument<'doc>>::Error>
            {
                type Error = MyError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = Pair {
                        first: rec.parse_field("first")?,
                        second: rec.parse_field("second")?
                    };
                    rec.deny_unknown_fields()?;
                    ctx.deny_unknown_extensions()?;
                    Ok(value)
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_newtype_struct_with_type_param() {
    let input = generate(parse_quote! {
        struct Wrapped<T>(T);
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc, T: ::eure::document::parse::ParseDocument<'doc, Error = ::eure::document::parse::ParseError> > ::eure::document::parse::ParseDocument<'doc> for Wrapped<T> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let field_0 = ctx.parse::<T>()?;
                    Ok(Wrapped(field_0))
                }
            }
        }
        .to_string()
    );
}
