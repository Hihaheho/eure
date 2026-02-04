use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;

fn generate(input: syn::DeriveInput) -> TokenStream {
    crate::from_eure::derive(crate::create_context(input).expect("failed to create context"))
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
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for User<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = User {
                        name: rec.parse_field::<String>("name")?,
                        age: rec.parse_field::<i32>("age")?
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
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for Unit<> {
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
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for Point<> {
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
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for Name<> {
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
            impl<'doc,> ::eure_document::parse::FromEure<'doc> for User<> {
                type Error = ::eure_document::parse::ParseError;

                fn parse(ctx: &::eure_document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = User {
                        name: rec.parse_field::<String>("name")?,
                        age: rec.parse_field::<i32>("age")?
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
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for User<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = User {
                        user_name: rec.parse_field::<String>("userName")?,
                        email_address: rec.parse_field::<String>("emailAddress")?
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
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for Config<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = Config {
                        max_retries: rec.parse_field::<i32>("max-retries")?,
                        timeout_seconds: rec.parse_field::<i32>("timeout-seconds")?
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
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for ExtFields<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let value = ExtFields {
                        optional: ctx.parse_ext::<bool>("optional")?,
                        deprecated: ctx.parse_ext::<bool>("deprecated")?
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
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for ExtFields<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let value = ExtFields {
                        binding_style: ctx.parse_ext::<String>("binding-style")?,
                        deny_untagged: ctx.parse_ext::<bool>("deny-untagged")?
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
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for Person<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = Person {
                        name: rec.parse_field::<String>("name")?,
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
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for Combined<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = Combined {
                        id: rec.parse_field::<i32>("id")?,
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
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for Person<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = Person {
                        full_name: rec.parse_field::<String>("fullName")?,
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
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for Person<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = Person {
                        name: rec.parse_field::<String>("name")?,
                        optional: ctx.parse_ext::<bool>("optional")?,
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
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for User<> {
                type Error = MyCustomError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = User {
                        name: rec.parse_field::<String>("name")?,
                        age: rec.parse_field::<i32>("age")?
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
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for User<> {
                type Error = crate::errors::MyCustomError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = User {
                        name: rec.parse_field::<String>("name")?
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
            impl<'doc,> ::eure_document::parse::FromEure<'doc> for User<> {
                type Error = MyError;

                fn parse(ctx: &::eure_document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = User {
                        name: rec.parse_field::<String>("name")?
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
            impl<'doc, T: ::eure::document::parse::FromEure<'doc, Error = ::eure::document::parse::ParseError> > ::eure::document::parse::FromEure<'doc> for Wrapper<T> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = Wrapper {
                        inner: rec.parse_field::<T>("inner")?
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
            impl<'doc, A: ::eure::document::parse::FromEure<'doc, Error = ::eure::document::parse::ParseError>, B: ::eure::document::parse::FromEure<'doc, Error = ::eure::document::parse::ParseError> > ::eure::document::parse::FromEure<'doc> for Pair<A, B> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = Pair {
                        first: rec.parse_field::<A>("first")?,
                        second: rec.parse_field::<B>("second")?
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
            impl<'doc, T: Clone + ::eure::document::parse::FromEure<'doc, Error = ::eure::document::parse::ParseError> > ::eure::document::parse::FromEure<'doc> for Wrapper<T> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = Wrapper {
                        inner: rec.parse_field::<T>("inner")?
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
            impl<'doc, T: ::eure::document::parse::FromEure<'doc> > ::eure::document::parse::FromEure<'doc> for Wrapper<T>
            where
                MyError: From<<T as ::eure::document::parse::FromEure<'doc>>::Error>
            {
                type Error = MyError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = Wrapper {
                        inner: rec.parse_field::<T>("inner")?
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
            impl<'doc, A: ::eure::document::parse::FromEure<'doc>, B: ::eure::document::parse::FromEure<'doc> > ::eure::document::parse::FromEure<'doc> for Pair<A, B>
            where
                MyError: From<<A as ::eure::document::parse::FromEure<'doc>>::Error>,
                MyError: From<<B as ::eure::document::parse::FromEure<'doc>>::Error>
            {
                type Error = MyError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = Pair {
                        first: rec.parse_field::<A>("first")?,
                        second: rec.parse_field::<B>("second")?
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
            impl<'doc, T: ::eure::document::parse::FromEure<'doc, Error = ::eure::document::parse::ParseError> > ::eure::document::parse::FromEure<'doc> for Wrapped<T> {
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

// ===========================================================================
// Proxy type support tests (transparent proxy with public fields)
// ===========================================================================

#[test]
fn test_proxy_basic() {
    let input = generate(parse_quote! {
        #[eure(proxy = "external::PublicConfig")]
        struct PublicConfigDef {
            host: String,
            port: u16,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::FromEure<'doc, external::PublicConfig> for PublicConfigDef<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<external::PublicConfig, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = external::PublicConfig {
                        host: rec.parse_field::<String>("host")?,
                        port: rec.parse_field::<u16>("port")?
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

// ===========================================================================
// Opaque type support tests (private fields, needs From impl)
// ===========================================================================

#[test]
fn test_opaque_basic() {
    let input = generate(parse_quote! {
        #[eure(opaque = "std::time::Duration")]
        struct DurationDef {
            secs: u64,
            nanos: u32,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::FromEure<'doc, std::time::Duration> for DurationDef<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<std::time::Duration, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = DurationDef {
                        secs: rec.parse_field::<u64>("secs")?,
                        nanos: rec.parse_field::<u32>("nanos")?
                    };
                    rec.deny_unknown_fields()?;
                    ctx.deny_unknown_extensions()?;
                    let value: std::time::Duration = value.into();
                    Ok(value)
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_opaque_newtype() {
    let input = generate(parse_quote! {
        #[eure(opaque = "external::Wrapper")]
        struct WrapperDef(String);
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::FromEure<'doc, external::Wrapper> for WrapperDef<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<external::Wrapper, Self::Error> {
                    let field_0 = ctx.parse::<String>()?;
                    let value: external::Wrapper = WrapperDef(field_0).into();
                    Ok(value)
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_proxy_with_rename_all() {
    let input = generate(parse_quote! {
        #[eure(proxy = "external::Config", rename_all = "camelCase")]
        struct ConfigDef {
            max_retries: i32,
            timeout_seconds: i32,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::FromEure<'doc, external::Config> for ConfigDef<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<external::Config, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = external::Config {
                        max_retries: rec.parse_field::<i32>("maxRetries")?,
                        timeout_seconds: rec.parse_field::<i32>("timeoutSeconds")?
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

/// Test that proxy and opaque are mutually exclusive (now handled via trybuild compile_fail test)

#[test]
fn test_opaque_tuple_struct() {
    let input = generate(parse_quote! {
        #[eure(opaque = "external::Point")]
        struct PointDef(i32, i32);
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::FromEure<'doc, external::Point> for PointDef<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<external::Point, Self::Error> {
                    let (field_0, field_1,) = ctx.parse::<(i32, i32,)>()?;
                    let value: external::Point = PointDef(field_0, field_1).into();
                    Ok(value)
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_opaque_unit_struct() {
    let input = generate(parse_quote! {
        #[eure(opaque = "external::Marker")]
        struct MarkerDef;
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::FromEure<'doc, external::Marker> for MarkerDef<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<external::Marker, Self::Error> {
                    ctx.parse::<()>()?;
                    let value: external::Marker = MarkerDef.into();
                    Ok(value)
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_opaque_with_parse_ext() {
    let input = generate(parse_quote! {
        #[eure(opaque = "external::ExtConfig", parse_ext)]
        struct ExtConfigDef {
            optional: bool,
            deprecated: bool,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::FromEure<'doc, external::ExtConfig> for ExtConfigDef<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<external::ExtConfig, Self::Error> {
                    let value = ExtConfigDef {
                        optional: ctx.parse_ext::<bool>("optional")?,
                        deprecated: ctx.parse_ext::<bool>("deprecated")?
                    };
                    let value: external::ExtConfig = value.into();
                    Ok(value)
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_proxy_with_parse_ext() {
    let input = generate(parse_quote! {
        #[eure(proxy = "external::ExtConfig", parse_ext)]
        struct ExtConfigDef {
            optional: bool,
            deprecated: bool,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::FromEure<'doc, external::ExtConfig> for ExtConfigDef<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<external::ExtConfig, Self::Error> {
                    let value = external::ExtConfig {
                        optional: ctx.parse_ext::<bool>("optional")?,
                        deprecated: ctx.parse_ext::<bool>("deprecated")?
                    };
                    Ok(value)
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_proxy_tuple_struct() {
    let input = generate(parse_quote! {
        #[eure(proxy = "external::Point")]
        struct PointDef(i32, i32);
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::FromEure<'doc, external::Point> for PointDef<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<external::Point, Self::Error> {
                    let (field_0, field_1,) = ctx.parse::<(i32, i32,)>()?;
                    Ok(external::Point(field_0, field_1))
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_proxy_unit_struct() {
    let input = generate(parse_quote! {
        #[eure(proxy = "external::Marker")]
        struct MarkerDef;
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::FromEure<'doc, external::Marker> for MarkerDef<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<external::Marker, Self::Error> {
                    ctx.parse::<()>()?;
                    Ok(external::Marker)
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_via_attribute_on_field() {
    let input = generate(parse_quote! {
        struct Config {
            name: String,
            #[eure(via = "DurationDef")]
            timeout: Duration,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for Config<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = Config {
                        name: rec.parse_field::<String>("name")?,
                        timeout: rec.parse_field_with("timeout", <DurationDef as ::eure::document::parse::FromEure<'doc, Duration>>::parse)?
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
fn test_via_attribute_with_container_type() {
    let input = generate(parse_quote! {
        struct Config {
            #[eure(via = "Option<DurationDef>")]
            timeout: Option<Duration>,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for Config<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = Config {
                        timeout: rec.parse_field_with("timeout", <Option<DurationDef> as ::eure::document::parse::FromEure<'doc, Option<Duration> >>::parse)?
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
fn test_via_attribute_with_default() {
    let input = generate(parse_quote! {
        struct Config {
            #[eure(via = "DurationDef", default)]
            timeout: Duration,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for Config<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = Config {
                        timeout: rec.parse_field_optional_with("timeout", <DurationDef as ::eure::document::parse::FromEure<'doc, Duration>>::parse)?
                            .unwrap_or_else(<Duration as ::core::default::Default>::default)
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
fn test_via_attribute_on_ext_field() {
    let input = generate(parse_quote! {
        struct Config {
            name: String,
            #[eure(ext, via = "DurationDef")]
            timeout: Duration,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for Config<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    let rec = ctx.parse_record()?;
                    let value = Config {
                        name: rec.parse_field::<String>("name")?,
                        timeout: ctx.parse_ext_with("timeout", <DurationDef as ::eure::document::parse::FromEure<'doc, Duration>>::parse)?
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
