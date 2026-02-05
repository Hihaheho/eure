use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;

fn generate(input: syn::DeriveInput) -> TokenStream {
    crate::from_eure::derive(crate::create_context(input).expect("failed to create context"))
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
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for TestEnum<> {
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
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for TestEnum<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .parse_variant::<(i32, bool,)>("Tuple", |(field_0, field_1,)| { let value : TestEnum = TestEnum::Tuple(field_0, field_1) ; Ok(value) })
                        .parse()
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
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for TestEnum<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .variant("Tuple", |ctx: &::eure::document::parse::ParseContext<'doc>| {
                            let mut tuple = ctx.parse_tuple()?;
                            tuple.expect_len(2)?;
                            let field_0 = tuple.next::<usize>()?;
                            let field_1 = tuple.next_via::<JumpAtProxy, JumpAt>()?;
                            let value : TestEnum = TestEnum::Tuple(field_0, field_1);
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
fn test_struct_variant() {
    let input = generate(parse_quote! {
        enum TestEnum {
            Struct { a: i32, b: bool },
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for TestEnum<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .variant("Struct", |ctx: &::eure::document::parse::ParseContext<'doc>| {
                            let mut rec = ctx.parse_record()?;
                            let value = TestEnum::Struct {
                                a: rec.parse_field::<i32>("a")?,
                                b: rec.parse_field::<bool>("b")?
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
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for TestEnum<> {
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
fn test_newtype_variant_with_via() {
    let input = generate(parse_quote! {
        enum TestEnum {
            Newtype(#[eure(via = "JumpAtProxy")] JumpAt),
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for TestEnum<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .variant("Newtype", |ctx: &::eure::document::parse::ParseContext<'doc>| {
                            let field_0 = ctx.parse_via::<JumpAtProxy, JumpAt>()?;
                            Ok(TestEnum::Newtype(field_0))
                        })
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
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for TestEnum<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .variant("Unit", ::eure::document::parse::DocumentParserExt::map(::eure::document::parse::VariantLiteralParser("Unit"), |_| TestEnum::Unit))
                        .parse_variant::<(i32, bool,)>("Tuple", |(field_0, field_1,)| { let value : TestEnum = TestEnum::Tuple(field_0, field_1) ; Ok(value) })
                        .variant("Struct", |ctx: &::eure::document::parse::ParseContext<'doc>| {
                            let mut rec = ctx.parse_record()?;
                            let value = TestEnum::Struct {
                                a: rec.parse_field::<i32>("a")?,
                                b: rec.parse_field::<bool>("b")?
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
            impl<'doc,> ::eure_document::parse::FromEure<'doc> for TestEnum<> {
                type Error = ::eure_document::parse::ParseError;

                fn parse(ctx: &::eure_document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure_document::data_model::VariantRepr::default())?
                        .variant("Unit", ::eure_document::parse::DocumentParserExt::map(::eure_document::parse::VariantLiteralParser("Unit"), |_| TestEnum::Unit))
                        .parse_variant::<(i32, bool,)>("Tuple", |(field_0, field_1,)| { let value : TestEnum = TestEnum::Tuple(field_0, field_1) ; Ok(value) })
                        .variant("Struct", |ctx: &::eure_document::parse::ParseContext<'doc>| {
                            let mut rec = ctx.parse_record()?;
                            let value = TestEnum::Struct {
                                a: rec.parse_field::<i32>("a")?,
                                b: rec.parse_field::<bool>("b")?
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
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for Event<> {
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
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for Event<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .variant("userCreated", |ctx: &::eure::document::parse::ParseContext<'doc>| {
                            let mut rec = ctx.parse_record()?;
                            let value = Event::UserCreated {
                                user_id: rec.parse_field::<i32>("user_id")?,
                                created_at: rec.parse_field::<String>("created_at")?
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
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for Event<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .variant("UserCreated", |ctx: &::eure::document::parse::ParseContext<'doc>| {
                            let mut rec = ctx.parse_record()?;
                            let value = Event::UserCreated {
                                user_id: rec.parse_field::<i32>("userId")?,
                                created_at: rec.parse_field::<String>("createdAt")?
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
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for Event<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .variant("user_created", |ctx: &::eure::document::parse::ParseContext<'doc>| {
                            let mut rec = ctx.parse_record()?;
                            let value = Event::UserCreated {
                                user_id: rec.parse_field::<i32>("userId")?,
                                created_at: rec.parse_field::<String>("createdAt")?
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
fn test_struct_variant_with_flatten() {
    let input = generate(parse_quote! {
        enum Entity {
            Person {
                name: String,
                #[eure(flatten)]
                details: PersonDetails,
            },
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for Entity<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .variant("Person", |ctx: &::eure::document::parse::ParseContext<'doc>| {
                            let mut rec = ctx.parse_record()?;
                            let value = Entity::Person {
                                name: rec.parse_field::<String>("name")?,
                                details: <PersonDetails>::parse(&rec.flatten())?
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
fn test_enum_custom_parse_error() {
    let input = generate(parse_quote! {
        #[eure(parse_error = MyCustomError)]
        enum TestEnum {
            Unit,
            Tuple(i32, bool),
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for TestEnum<> {
                type Error = MyCustomError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .variant("Unit", ::eure::document::parse::DocumentParserExt::map(::eure::document::parse::VariantLiteralParser("Unit"), |_| TestEnum::Unit))
                        .parse_variant::<(i32, bool,)>("Tuple", |(field_0, field_1,)| { let value : TestEnum = TestEnum::Tuple(field_0, field_1) ; Ok(value) })
                        .parse()
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
            impl<'doc, T: ::eure::document::parse::FromEure<'doc, Error = ::eure::document::parse::ParseError> > ::eure::document::parse::FromEure<'doc> for Item<T> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .parse_variant::<T>("Normal", |field_0| Ok(Item::Normal(field_0)))
                        .parse_variant::<Vec<T> >("List", |field_0| Ok(Item::List(field_0)))
                        .parse()
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
            impl<'doc, L: ::eure::document::parse::FromEure<'doc, Error = ::eure::document::parse::ParseError>, R: ::eure::document::parse::FromEure<'doc, Error = ::eure::document::parse::ParseError> > ::eure::document::parse::FromEure<'doc> for Either<L, R> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .parse_variant::<L>("Left", |field_0| Ok(Either::Left(field_0)))
                        .parse_variant::<R>("Right", |field_0| Ok(Either::Right(field_0)))
                        .parse()
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
            impl<'doc, T: Clone + ::eure::document::parse::FromEure<'doc, Error = ::eure::document::parse::ParseError> > ::eure::document::parse::FromEure<'doc> for Item<T> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .parse_variant::<T>("Normal", |field_0| Ok(Item::Normal(field_0)))
                        .parse()
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_enum_type_param_with_custom_error() {
    let input = generate(parse_quote! {
        #[eure(parse_error = MyError)]
        enum Item<T> {
            Normal(T),
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc, T: ::eure::document::parse::FromEure<'doc> > ::eure::document::parse::FromEure<'doc> for Item<T>
            where
                MyError: From<<T as ::eure::document::parse::FromEure<'doc>>::Error>
            {
                type Error = MyError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .parse_variant::<T>("Normal", |field_0| Ok(Item::Normal(field_0)))
                        .parse()
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_enum_multiple_type_params_with_custom_error() {
    let input = generate(parse_quote! {
        #[eure(parse_error = MyError)]
        enum Either<L, R> {
            Left(L),
            Right(R),
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc, L: ::eure::document::parse::FromEure<'doc>, R: ::eure::document::parse::FromEure<'doc> > ::eure::document::parse::FromEure<'doc> for Either<L, R>
            where
                MyError: From<<L as ::eure::document::parse::FromEure<'doc>>::Error>,
                MyError: From<<R as ::eure::document::parse::FromEure<'doc>>::Error>
            {
                type Error = MyError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .parse_variant::<L>("Left", |field_0| Ok(Either::Left(field_0)))
                        .parse_variant::<R>("Right", |field_0| Ok(Either::Right(field_0)))
                        .parse()
                }
            }
        }
        .to_string()
    );
}

// =============================================================================
// Struct variant with flatten + ext (no regular fields)
// =============================================================================

/// When a struct variant has ONLY flatten field(s), no parse_record() is needed.
/// The flatten field should be parsed via ctx.flatten() (not rec.flatten()).
#[test]
fn test_struct_variant_flatten_only() {
    let input = generate(parse_quote! {
        enum Content {
            Text {
                #[eure(flatten)]
                value: TextValue,
            },
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for Content<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .variant("Text", |ctx: &::eure::document::parse::ParseContext<'doc>| {
                            let value = Content::Text {
                                value: <TextValue>::parse(&ctx.flatten())?
                            };
                            ctx.deny_unknown_extensions()?;
                            Ok(value)
                        })
                        .parse()
                }
            }
        }
        .to_string()
    );
}

/// When a struct variant has ONLY ext field(s), no parse_record() is needed.
#[test]
fn test_struct_variant_ext_only() {
    let input = generate(parse_quote! {
        enum Item {
            WithMeta {
                #[eure(ext)]
                meta: MetaData,
            },
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for Item<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .variant("WithMeta", |ctx: &::eure::document::parse::ParseContext<'doc>| {
                            let value = Item::WithMeta {
                                meta: ctx.parse_ext::<MetaData>("meta")?
                            };
                            ctx.deny_unknown_extensions()?;
                            Ok(value)
                        })
                        .parse()
                }
            }
        }
        .to_string()
    );
}

/// When a struct variant has ONLY ext field(s) with default, use parse_ext_optional.
#[test]
fn test_struct_variant_ext_with_default() {
    let input = generate(parse_quote! {
        enum Item {
            WithMeta {
                #[eure(ext, default)]
                meta: Option<MetaData>,
            },
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for Item<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .variant("WithMeta", |ctx: &::eure::document::parse::ParseContext<'doc>| {
                            let value = Item::WithMeta {
                                meta: ctx.parse_ext_optional::<Option<MetaData> >("meta")?.unwrap_or_else(<Option<MetaData> as ::core::default::Default>::default)
                            };
                            ctx.deny_unknown_extensions()?;
                            Ok(value)
                        })
                        .parse()
                }
            }
        }
        .to_string()
    );
}

/// Main reproduction case: struct variant with flatten + ext fields (no regular fields).
/// This should NOT call parse_record() since:
/// - flatten field parses via ctx.flatten() (not rec.flatten())
/// - ext field parses from extensions via ctx.parse_ext()
#[test]
fn test_struct_variant_flatten_and_ext() {
    let input = generate(parse_quote! {
        enum TextOrNested {
            Text {
                #[eure(flatten)]
                text: TextValue,
                #[eure(ext, default)]
                mark: MarkOptions,
            },
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for TextOrNested<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .variant("Text", |ctx: &::eure::document::parse::ParseContext<'doc>| {
                            let value = TextOrNested::Text {
                                text: <TextValue>::parse(&ctx.flatten())?,
                                mark: ctx.parse_ext_optional::<MarkOptions>("mark")?.unwrap_or_else(<MarkOptions as ::core::default::Default>::default)
                            };
                            ctx.deny_unknown_extensions()?;
                            Ok(value)
                        })
                        .parse()
                }
            }
        }
        .to_string()
    );
}

/// When struct variant has ext + regular fields, parse_record() IS needed.
#[test]
fn test_struct_variant_ext_with_regular_fields() {
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
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for Item<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .variant("WithMeta", |ctx: &::eure::document::parse::ParseContext<'doc>| {
                            let mut rec = ctx.parse_record()?;
                            let value = Item::WithMeta {
                                name: rec.parse_field::<String>("name")?,
                                meta: ctx.parse_ext::<MetaData>("meta")?
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

/// Struct variant with flatten_ext only (no regular fields).
#[test]
fn test_struct_variant_flatten_ext_only() {
    let input = generate(parse_quote! {
        enum Item {
            WithMeta {
                #[eure(flatten_ext)]
                meta: MetaData,
            },
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::FromEure<'doc> for Item<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .variant("WithMeta", |ctx: &::eure::document::parse::ParseContext<'doc>| {
                            let value = Item::WithMeta {
                                meta: <MetaData>::parse(&ctx.flatten_ext())?
                            };
                            ctx.deny_unknown_extensions()?;
                            Ok(value)
                        })
                        .parse()
                }
            }
        }
        .to_string()
    );
}

// =============================================================================
// Proxy enum tests
// =============================================================================

/// Proxy enum: constructs target type directly (unit variants).
#[test]
fn test_proxy_enum_unit_variant() {
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
            impl<'doc,> ::eure::document::parse::FromEure<'doc, external::Status> for StatusDef<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<external::Status, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .variant("Active", ::eure::document::parse::DocumentParserExt::map(::eure::document::parse::VariantLiteralParser("Active"), |_| external::Status::Active))
                        .variant("Inactive", ::eure::document::parse::DocumentParserExt::map(::eure::document::parse::VariantLiteralParser("Inactive"), |_| external::Status::Inactive))
                        .parse()
                }
            }
        }
        .to_string()
    );
}

/// Proxy enum: constructs target type directly (newtype variants).
#[test]
fn test_proxy_enum_newtype_variant() {
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
            impl<'doc,> ::eure::document::parse::FromEure<'doc, external::Value> for ValueDef<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<external::Value, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .parse_variant::<String>("Text", |field_0| Ok(external::Value::Text(field_0)))
                        .parse_variant::<i32>("Number", |field_0| Ok(external::Value::Number(field_0)))
                        .parse()
                }
            }
        }
        .to_string()
    );
}

// =============================================================================
// Opaque enum tests
// =============================================================================

/// Opaque enum: constructs definition type then converts via .into() (unit variants).
#[test]
fn test_opaque_enum_unit_variant() {
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
            impl<'doc,> ::eure::document::parse::FromEure<'doc, external::Status> for StatusDef<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<external::Status, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .variant("Active", ::eure::document::parse::DocumentParserExt::map(::eure::document::parse::VariantLiteralParser("Active"), |_| StatusDef::Active.into()))
                        .variant("Inactive", ::eure::document::parse::DocumentParserExt::map(::eure::document::parse::VariantLiteralParser("Inactive"), |_| StatusDef::Inactive.into()))
                        .parse()
                }
            }
        }
        .to_string()
    );
}

/// Opaque enum: constructs definition type then converts via .into() (newtype variants).
#[test]
fn test_opaque_enum_newtype_variant() {
    let input = generate(parse_quote! {
        #[eure(opaque = "external::Value")]
        enum ValueDef {
            Text(String),
            Number(i32),
        }
    });
    assert_eq!(
        input.to_string(),
        quote! {
            impl<'doc,> ::eure::document::parse::FromEure<'doc, external::Value> for ValueDef<> {
                type Error = ::eure::document::parse::ParseError;

                fn parse(ctx: &::eure::document::parse::ParseContext<'doc>) -> Result<external::Value, Self::Error> {
                    ctx.parse_union(::eure::document::data_model::VariantRepr::default())?
                        .parse_variant::<String>("Text", |field_0| Ok(ValueDef::Text(field_0).into()))
                        .parse_variant::<i32>("Number", |field_0| Ok(ValueDef::Number(field_0).into()))
                        .parse()
                }
            }
        }
        .to_string()
    );
}
