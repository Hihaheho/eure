use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;

fn generate(input: syn::DeriveInput) -> TokenStream {
    crate::into_eure::derive(crate::create_context(input).expect("failed to create context"))
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
            impl ::eure::document::write::IntoEureRecord for User {
                fn write_to_record(value: Self, rec: &mut ::eure::document::write::RecordWriter<'_>) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    rec.field("name", value.name)?;
                    rec.field("age", value.age)?;
                    Ok(())
                }
            }
            impl ::eure::document::write::IntoEure for User {
                fn write(value: Self, c: &mut ::eure::document::constructor::DocumentConstructor) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    c.record(|rec| {
                        <Self as ::eure::document::write::IntoEureRecord>::write_to_record(value, rec)
                    })
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
            impl ::eure::document::write::IntoEure for Unit {
                fn write(value: Self, c: &mut ::eure::document::constructor::DocumentConstructor) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    let _ = value;
                    c.bind_primitive(::eure::document::value::PrimitiveValue::Null)?;
                    Ok(())
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
            impl ::eure::document::write::IntoEure for Point {
                fn write(value: Self, c: &mut ::eure::document::constructor::DocumentConstructor) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    let Point(field_0, field_1) = value;
                    c.tuple(|t| {
                        t.next(field_0)?;
                        t.next(field_1)?;
                        Ok(())
                    })
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_tuple_struct_with_via() {
    let input = generate(parse_quote! {
        struct Steps(usize, #[eure(via = "JumpAtProxy")] JumpAt);
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::write::IntoEure for Steps {
                fn write(value: Self, c: &mut ::eure::document::constructor::DocumentConstructor) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    let Steps(field_0, field_1) = value;
                    c.tuple(|t| {
                        t.next(field_0)?;
                        t.next_via::<JumpAtProxy, _>(field_1)?;
                        Ok(())
                    })
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
            impl ::eure::document::write::IntoEure for Name {
                fn write(value: Self, c: &mut ::eure::document::constructor::DocumentConstructor) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    let Name(inner) = value;
                    <String as ::eure::document::write::IntoEure>::write(inner, c)
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
            impl ::eure_document::write::IntoEureRecord for User {
                fn write_to_record(value: Self, rec: &mut ::eure_document::write::RecordWriter<'_>) -> ::core::result::Result<(), ::eure_document::write::WriteError> {
                    rec.field("name", value.name)?;
                    rec.field("age", value.age)?;
                    Ok(())
                }
            }
            impl ::eure_document::write::IntoEure for User {
                fn write(value: Self, c: &mut ::eure_document::constructor::DocumentConstructor) -> ::core::result::Result<(), ::eure_document::write::WriteError> {
                    c.record(|rec| {
                        <Self as ::eure_document::write::IntoEureRecord>::write_to_record(value, rec)
                    })
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
            impl ::eure::document::write::IntoEureRecord for User {
                fn write_to_record(value: Self, rec: &mut ::eure::document::write::RecordWriter<'_>) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    rec.field("userName", value.user_name)?;
                    rec.field("emailAddress", value.email_address)?;
                    Ok(())
                }
            }
            impl ::eure::document::write::IntoEure for User {
                fn write(value: Self, c: &mut ::eure::document::constructor::DocumentConstructor) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    c.record(|rec| {
                        <Self as ::eure::document::write::IntoEureRecord>::write_to_record(value, rec)
                    })
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
            impl ::eure::document::write::IntoEureRecord for Config {
                fn write_to_record(value: Self, rec: &mut ::eure::document::write::RecordWriter<'_>) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    rec.field("max-retries", value.max_retries)?;
                    rec.field("timeout-seconds", value.timeout_seconds)?;
                    Ok(())
                }
            }
            impl ::eure::document::write::IntoEure for Config {
                fn write(value: Self, c: &mut ::eure::document::constructor::DocumentConstructor) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    c.record(|rec| {
                        <Self as ::eure::document::write::IntoEureRecord>::write_to_record(value, rec)
                    })
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_ext_field() {
    let input = generate(parse_quote! {
        struct WithExt {
            name: String,
            #[eure(ext)]
            optional: bool,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::write::IntoEureRecord for WithExt {
                fn write_to_record(value: Self, rec: &mut ::eure::document::write::RecordWriter<'_>) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    rec.field("name", value.name)?;
                    rec.constructor().set_extension("optional", value.optional)?;
                    Ok(())
                }
            }
            impl ::eure::document::write::IntoEure for WithExt {
                fn write(value: Self, c: &mut ::eure::document::constructor::DocumentConstructor) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    c.record(|rec| {
                        <Self as ::eure::document::write::IntoEureRecord>::write_to_record(value, rec)
                    })
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
            impl<T: ::eure::document::write::IntoEure > ::eure::document::write::IntoEureRecord for Wrapper<T> {
                fn write_to_record(value: Self, rec: &mut ::eure::document::write::RecordWriter<'_>) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    rec.field("inner", value.inner)?;
                    Ok(())
                }
            }
            impl<T: ::eure::document::write::IntoEure > ::eure::document::write::IntoEure for Wrapper<T> {
                fn write(value: Self, c: &mut ::eure::document::constructor::DocumentConstructor) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    c.record(|rec| {
                        <Self as ::eure::document::write::IntoEureRecord>::write_to_record(value, rec)
                    })
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
            impl<A: ::eure::document::write::IntoEure, B: ::eure::document::write::IntoEure > ::eure::document::write::IntoEureRecord for Pair<A, B> {
                fn write_to_record(value: Self, rec: &mut ::eure::document::write::RecordWriter<'_>) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    rec.field("first", value.first)?;
                    rec.field("second", value.second)?;
                    Ok(())
                }
            }
            impl<A: ::eure::document::write::IntoEure, B: ::eure::document::write::IntoEure > ::eure::document::write::IntoEure for Pair<A, B> {
                fn write(value: Self, c: &mut ::eure::document::constructor::DocumentConstructor) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    c.record(|rec| {
                        <Self as ::eure::document::write::IntoEureRecord>::write_to_record(value, rec)
                    })
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
            impl<T: Clone + ::eure::document::write::IntoEure > ::eure::document::write::IntoEureRecord for Wrapper<T> {
                fn write_to_record(value: Self, rec: &mut ::eure::document::write::RecordWriter<'_>) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    rec.field("inner", value.inner)?;
                    Ok(())
                }
            }
            impl<T: Clone + ::eure::document::write::IntoEure > ::eure::document::write::IntoEure for Wrapper<T> {
                fn write(value: Self, c: &mut ::eure::document::constructor::DocumentConstructor) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    c.record(|rec| {
                        <Self as ::eure::document::write::IntoEureRecord>::write_to_record(value, rec)
                    })
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
            impl<T: ::eure::document::write::IntoEure > ::eure::document::write::IntoEure for Wrapped<T> {
                fn write(value: Self, c: &mut ::eure::document::constructor::DocumentConstructor) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    let Wrapped(inner) = value;
                    <T as ::eure::document::write::IntoEure>::write(inner, c)
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
            impl ::eure::document::write::IntoEureRecord<external::PublicConfig> for PublicConfigDef {
                fn write_to_record(value: external::PublicConfig, rec: &mut ::eure::document::write::RecordWriter<'_>) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    let _: &String = &value.host;
                    let _: &u16 = &value.port;
                    rec.field("host", value.host)?;
                    rec.field("port", value.port)?;
                    Ok(())
                }
            }
            impl ::eure::document::write::IntoEure<external::PublicConfig> for PublicConfigDef {
                fn write(value: external::PublicConfig, c: &mut ::eure::document::constructor::DocumentConstructor) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    c.record(|rec| {
                        <Self as ::eure::document::write::IntoEureRecord<external::PublicConfig>>::write_to_record(value, rec)
                    })
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
            impl ::eure::document::write::IntoEureRecord for DurationDef {
                fn write_to_record(value: Self, rec: &mut ::eure::document::write::RecordWriter<'_>) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    let _: &u64 = &value.secs;
                    let _: &u32 = &value.nanos;
                    rec.field("secs", value.secs)?;
                    rec.field("nanos", value.nanos)?;
                    Ok(())
                }
            }
            impl ::eure::document::write::IntoEure<std::time::Duration> for DurationDef {
                fn write(value: std::time::Duration, c: &mut ::eure::document::constructor::DocumentConstructor) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    let value: DurationDef = value.into();
                    c.record(|rec| {
                        <Self as ::eure::document::write::IntoEureRecord>::write_to_record(value, rec)
                    })
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
            impl ::eure::document::write::IntoEure<external::Wrapper> for WrapperDef {
                fn write(value: external::Wrapper, c: &mut ::eure::document::constructor::DocumentConstructor) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    let value: WrapperDef = value.into();
                    let WrapperDef(inner) = value;
                    <String as ::eure::document::write::IntoEure>::write(inner, c)
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
            impl ::eure::document::write::IntoEureRecord<external::Config> for ConfigDef {
                fn write_to_record(value: external::Config, rec: &mut ::eure::document::write::RecordWriter<'_>) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    let _: &i32 = &value.max_retries;
                    let _: &i32 = &value.timeout_seconds;
                    rec.field("maxRetries", value.max_retries)?;
                    rec.field("timeoutSeconds", value.timeout_seconds)?;
                    Ok(())
                }
            }
            impl ::eure::document::write::IntoEure<external::Config> for ConfigDef {
                fn write(value: external::Config, c: &mut ::eure::document::constructor::DocumentConstructor) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    c.record(|rec| {
                        <Self as ::eure::document::write::IntoEureRecord<external::Config>>::write_to_record(value, rec)
                    })
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_opaque_tuple_struct() {
    let input = generate(parse_quote! {
        #[eure(opaque = "external::Point")]
        struct PointDef(i32, i32);
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::write::IntoEure<external::Point> for PointDef {
                fn write(value: external::Point, c: &mut ::eure::document::constructor::DocumentConstructor) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    let value: PointDef = value.into();
                    let PointDef(field_0, field_1) = value;
                    c.tuple(|t| {
                        t.next(field_0)?;
                        t.next(field_1)?;
                        Ok(())
                    })
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
            impl ::eure::document::write::IntoEure<external::Marker> for MarkerDef {
                fn write(value: external::Marker, c: &mut ::eure::document::constructor::DocumentConstructor) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    let _: MarkerDef = value.into();
                    c.bind_primitive(::eure::document::value::PrimitiveValue::Null)?;
                    Ok(())
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
            impl ::eure::document::write::IntoEure<external::Point> for PointDef {
                fn write(value: external::Point, c: &mut ::eure::document::constructor::DocumentConstructor) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    let external::Point(field_0, field_1) = value;
                    c.tuple(|t| {
                        t.next(field_0)?;
                        t.next(field_1)?;
                        Ok(())
                    })
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
            impl ::eure::document::write::IntoEure<external::Marker> for MarkerDef {
                fn write(value: external::Marker, c: &mut ::eure::document::constructor::DocumentConstructor) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    let _ = value;
                    c.bind_primitive(::eure::document::value::PrimitiveValue::Null)?;
                    Ok(())
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
            impl ::eure::document::write::IntoEureRecord for Config {
                fn write_to_record(value: Self, rec: &mut ::eure::document::write::RecordWriter<'_>) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    rec.field("name", value.name)?;
                    rec.field_via::<DurationDef, _>("timeout", value.timeout)?;
                    Ok(())
                }
            }
            impl ::eure::document::write::IntoEure for Config {
                fn write(value: Self, c: &mut ::eure::document::constructor::DocumentConstructor) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    c.record(|rec| {
                        <Self as ::eure::document::write::IntoEureRecord>::write_to_record(value, rec)
                    })
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
            impl ::eure::document::write::IntoEureRecord for Config {
                fn write_to_record(value: Self, rec: &mut ::eure::document::write::RecordWriter<'_>) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    rec.field("name", value.name)?;
                    {
                        let scope = rec.constructor().begin_scope();
                        let ident: ::eure::document::identifier::Identifier = "timeout".parse()
                            .map_err(|_| ::eure::document::write::WriteError::InvalidIdentifier("timeout".into()))?;
                        rec.constructor().navigate(::eure::document::path::PathSegment::Extension(ident))?;
                        rec.constructor().write_via::<DurationDef, _>(value.timeout)?;
                        rec.constructor().end_scope(scope)?;
                    }
                    Ok(())
                }
            }
            impl ::eure::document::write::IntoEure for Config {
                fn write(value: Self, c: &mut ::eure::document::constructor::DocumentConstructor) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    c.record(|rec| {
                        <Self as ::eure::document::write::IntoEureRecord>::write_to_record(value, rec)
                    })
                }
            }
        }
        .to_string()
    );
}

// ===========================================================================
// Flatten tests
// ===========================================================================

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
            impl ::eure::document::write::IntoEureRecord for Person {
                fn write_to_record(value: Self, rec: &mut ::eure::document::write::RecordWriter<'_>) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    rec.field("name", value.name)?;
                    rec.flatten::<Address, _>(value.address)?;
                    Ok(())
                }
            }
            impl ::eure::document::write::IntoEure for Person {
                fn write(value: Self, c: &mut ::eure::document::constructor::DocumentConstructor) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    c.record(|rec| {
                        <Self as ::eure::document::write::IntoEureRecord>::write_to_record(value, rec)
                    })
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_flatten_ext_field() {
    let input = generate(parse_quote! {
        struct Record {
            name: String,
            #[eure(flatten_ext)]
            ext: ExtData,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::write::IntoEureRecord for Record {
                fn write_to_record(value: Self, rec: &mut ::eure::document::write::RecordWriter<'_>) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    rec.field("name", value.name)?;
                    rec.flatten_ext::<ExtData, _>(value.ext)?;
                    Ok(())
                }
            }
            impl ::eure::document::write::IntoEure for Record {
                fn write(value: Self, c: &mut ::eure::document::constructor::DocumentConstructor) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    c.record(|rec| {
                        <Self as ::eure::document::write::IntoEureRecord>::write_to_record(value, rec)
                    })
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_flatten_with_multiple() {
    let input = generate(parse_quote! {
        struct FullProfile {
            id: i32,
            #[eure(flatten)]
            address: Address,
            #[eure(flatten)]
            contact: ContactInfo,
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl ::eure::document::write::IntoEureRecord for FullProfile {
                fn write_to_record(value: Self, rec: &mut ::eure::document::write::RecordWriter<'_>) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    rec.field("id", value.id)?;
                    rec.flatten::<Address, _>(value.address)?;
                    rec.flatten::<ContactInfo, _>(value.contact)?;
                    Ok(())
                }
            }
            impl ::eure::document::write::IntoEure for FullProfile {
                fn write(value: Self, c: &mut ::eure::document::constructor::DocumentConstructor) -> ::core::result::Result<(), ::eure::document::write::WriteError> {
                    c.record(|rec| {
                        <Self as ::eure::document::write::IntoEureRecord>::write_to_record(value, rec)
                    })
                }
            }
        }
        .to_string()
    );
}
