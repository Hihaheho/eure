use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields, Attribute, Lit};
use convert_case::{Case, Casing};

/// Derive macro for generating ToEureSchema implementations
/// 
/// This macro generates an implementation of the `ToEureSchema` trait for
/// a struct or enum, creating EURE schema definitions that match the structure.
/// 
/// # Example
/// 
/// ```rust
/// use eure_derive::Eure;
/// use serde::{Serialize, Deserialize};
/// 
/// #[derive(Eure, Serialize, Deserialize)]
/// struct User {
///     #[eure(length(min = 3, max = 20), pattern = "^[a-z]+$")]
///     name: String,
///     #[serde(rename = "user_age")]
///     #[eure(range(min = 0, max = 150))]
///     age: u32,
///     #[eure(description = "User's email address")]
///     email: Option<String>,
/// }
/// ```
#[proc_macro_derive(Eure, attributes(serde, eure))]
pub fn derive_eure(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    
    match generate_to_eure_schema_impl(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn generate_to_eure_schema_impl(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    
    // Add ToEureSchema bound to all type parameters
    let where_clause = add_trait_bounds(where_clause, &input.generics);
    
    let schema_expr = match &input.data {
        Data::Struct(data_struct) => generate_struct_schema(data_struct, &input.attrs)?,
        Data::Enum(data_enum) => generate_enum_schema(data_enum, &input.attrs)?,
        Data::Union(_) => {
            return Err(syn::Error::new_spanned(
                input,
                "Union types are not supported by Eure derive",
            ))
        }
    };
    
    let type_name = name.to_string();
    
    Ok(quote! {
        impl #impl_generics ::eure_schema::ToEureSchema for #name #ty_generics #where_clause {
            fn eure_schema() -> ::eure_schema::FieldSchema {
                #schema_expr
            }
            
            fn type_name() -> Option<&'static str> {
                Some(#type_name)
            }
        }
    })
}

fn add_trait_bounds(
    where_clause: Option<&syn::WhereClause>,
    generics: &syn::Generics,
) -> proc_macro2::TokenStream {
    let mut predicates = where_clause
        .map(|w| w.predicates.iter().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    
    // Add ToEureSchema bound for each type parameter
    for param in &generics.params {
        if let syn::GenericParam::Type(type_param) = param {
            let ident = &type_param.ident;
            predicates.push(syn::parse_quote!(#ident: ::eure_schema::ToEureSchema));
        }
    }
    
    if predicates.is_empty() {
        quote! {}
    } else {
        quote! { where #(#predicates),* }
    }
}

fn generate_struct_schema(
    data_struct: &syn::DataStruct,
    attrs: &[Attribute],
) -> syn::Result<proc_macro2::TokenStream> {
    let serde_opts = extract_serde_options(attrs)?;
    let rename_all_rule = extract_rename_all_rule(attrs)?;
    let container_has_default = has_container_serde_default(attrs);
    let is_transparent = has_serde_transparent(attrs);
    
    // Handle transparent structs
    if is_transparent {
        match &data_struct.fields {
            Fields::Named(fields) if fields.named.len() == 1 => {
                let field = fields.named.first().unwrap();
                let ty = &field.ty;
                let field_eure_opts = extract_field_eure_options(field)?;
                
                // For transparent structs, we want the actual schema, not a field schema
                // This preserves constraints while keeping the inner type's structure
                return Ok(quote! {
                    {
                        let mut schema = <#ty as ::eure_schema::ToEureSchema>::eure_schema();
                        #field_eure_opts
                        schema
                    }
                });
            }
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let field = fields.unnamed.first().unwrap();
                let ty = &field.ty;
                let field_eure_opts = extract_field_eure_options(field)?;
                
                // For transparent structs, we want the actual schema, not a field schema
                // This preserves constraints while keeping the inner type's structure
                return Ok(quote! {
                    {
                        let mut schema = <#ty as ::eure_schema::ToEureSchema>::eure_schema();
                        #field_eure_opts
                        schema
                    }
                });
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    data_struct.struct_token,
                    "#[serde(transparent)] requires exactly one field",
                ));
            }
        }
    }
    
    let fields_tokens = match &data_struct.fields {
        Fields::Named(fields) => {
            let mut regular_fields = Vec::new();
            let mut flattened_types = Vec::new();
            
            for field in &fields.named {
                // Skip fields with #[serde(skip)]
                if has_serde_skip(field) {
                    continue;
                }
                
                // Handle flattened fields separately
                if has_serde_flatten(field) {
                    let ty = &field.ty;
                    flattened_types.push(ty);
                    continue;
                }
                
                let field_name = field.ident.as_ref().unwrap();
                let field_schema = generate_field_schema(field, &serde_opts, container_has_default)?;
                let field_name_str = apply_rename_rule(
                    &field_name.to_string(),
                    &extract_field_rename(field)?,
                    &rename_all_rule,
                );
                regular_fields.push(quote! {
                    fields.insert(::eure_schema::KeyCmpValue::String(#field_name_str.to_string()), #field_schema);
                });
            }
            
            let flattened_tokens = if !flattened_types.is_empty() {
                quote! {
                    // Merge fields from flattened types
                    #(
                        if let ::eure_schema::Type::Object(flattened_schema) = <#flattened_types as ::eure_schema::ToEureSchema>::eure_schema().type_expr {
                            for (key, value) in flattened_schema.fields {
                                fields.insert(key, value);
                            }
                        }
                    )*
                }
            } else {
                quote! {}
            };
            
            let field_schemas = regular_fields;
            
            quote! {
                {
                    let mut fields = ::indexmap::IndexMap::new();
                    #(#field_schemas)*
                    #flattened_tokens
                    ::eure_schema::Type::Object(::eure_schema::ObjectSchema {
                        fields,
                        additional_properties: None, // deny_unknown_fields is the default behavior
                    })
                }
            }
        }
        Fields::Unnamed(fields) => {
            if fields.unnamed.len() == 1 {
                // Newtype struct - delegate to inner type
                let field = fields.unnamed.first().unwrap();
                let ty = &field.ty;
                quote! {
                    <#ty as ::eure_schema::ToEureSchema>::eure_field_schema().type_expr
                }
            } else {
                // Tuple struct - represent as array
                quote! {
                    ::eure_schema::Type::Array(Box::new(::eure_schema::Type::Any))
                }
            }
        }
        Fields::Unit => {
            // Unit struct
            quote! {
                ::eure_schema::Type::Null
            }
        }
    };
    
    let result = quote! {
        ::eure_schema::FieldSchema {
            type_expr: #fields_tokens,
            optional: false,
            constraints: Default::default(),
            preferences: Default::default(),
            serde: #serde_opts,
            span: None,
            default_value: None,
            description: None,
        }
    };
    
    Ok(result)
}

fn generate_enum_schema(
    data_enum: &syn::DataEnum,
    attrs: &[Attribute],
) -> syn::Result<proc_macro2::TokenStream> {
    let serde_opts = extract_serde_options(attrs)?;
    let rename_all_rule = extract_rename_all_rule(attrs)?;
    let rename_all_fields_rule = extract_rename_all_fields_rule(attrs)?;
    let variant_repr = extract_variant_representation(attrs)?;
    
    let variants: Vec<_> = data_enum.variants.iter()
        .map(|variant| {
            let variant_name = apply_rename_rule(
                &variant.ident.to_string(),
                &extract_variant_rename(variant)?,
                &rename_all_rule,
            );
            
            let variant_schema = match &variant.fields {
                Fields::Named(fields) => {
                    // Get variant-level rename_all, fall back to enum-level rename_all_fields
                    let variant_field_rename_rule = extract_variant_rename_all(variant)?
                        .or(rename_all_fields_rule.clone());
                    
                    let field_schemas: Vec<_> = fields.named.iter()
                        .map(|field| {
                            let field_name = field.ident.as_ref().unwrap();
                            let field_schema = generate_field_schema(field, &serde_opts, false)?;
                            let field_name_str = apply_rename_rule(
                                &field_name.to_string(),
                                &extract_field_rename(field)?,
                                &variant_field_rename_rule,  // Use variant-specific or enum-level rename rule
                            );
                            Ok(quote! {
                                variant_fields.insert(::eure_schema::KeyCmpValue::String(#field_name_str.to_string()), #field_schema);
                            })
                        })
                        .collect::<syn::Result<Vec<_>>>()?;
                    
                    quote! {
                        {
                            let mut variant_fields = ::indexmap::IndexMap::new();
                            #(#field_schemas)*
                            ::eure_schema::ObjectSchema {
                                fields: variant_fields,
                                additional_properties: None,
                            }
                        }
                    }
                }
                Fields::Unnamed(fields) => {
                    if fields.unnamed.len() == 1 {
                        // Single field variant
                        let field = fields.unnamed.first().unwrap();
                        let ty = &field.ty;
                        quote! {
                            {
                                let mut variant_fields = ::indexmap::IndexMap::new();
                                variant_fields.insert(::eure_schema::KeyCmpValue::U64(0), <#ty as ::eure_schema::ToEureSchema>::eure_field_schema());
                                ::eure_schema::ObjectSchema {
                                    fields: variant_fields,
                                    additional_properties: None,
                                }
                            }
                        }
                    } else {
                        // Multiple unnamed fields - enumerate them with numeric indices
                        let field_insertions = fields.unnamed.iter().enumerate().map(|(idx, field)| {
                            let idx = idx as u64;
                            let ty = &field.ty;
                            quote! {
                                variant_fields.insert(::eure_schema::KeyCmpValue::U64(#idx), <#ty as ::eure_schema::ToEureSchema>::eure_field_schema());
                            }
                        });
                        
                        quote! {
                            {
                                let mut variant_fields = ::indexmap::IndexMap::new();
                                #(#field_insertions)*
                                ::eure_schema::ObjectSchema {
                                    fields: variant_fields,
                                    additional_properties: None,
                                }
                            }
                        }
                    }
                }
                Fields::Unit => {
                    quote! {
                        ::eure_schema::ObjectSchema {
                            fields: ::indexmap::IndexMap::new(),
                            additional_properties: None,
                        }
                    }
                }
            };
            
            Ok(quote! {
                variants.insert(::eure_schema::KeyCmpValue::String(#variant_name.to_string()), #variant_schema);
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;
    
    Ok(quote! {
        {
            let mut variants = ::indexmap::IndexMap::new();
            #(#variants)*
            
            ::eure_schema::FieldSchema {
                type_expr: ::eure_schema::Type::Variants(::eure_schema::VariantSchema {
                    variants,
                    representation: #variant_repr,
                }),
                optional: false,
                constraints: Default::default(),
                preferences: Default::default(),
                serde: #serde_opts,
                span: None,
                default_value: None,
                description: None,
            }
        }
    })
}

fn generate_field_schema(
    field: &syn::Field,
    _parent_serde_opts: &proc_macro2::TokenStream,
    container_has_default: bool,
) -> syn::Result<proc_macro2::TokenStream> {
    let ty = &field.ty;
    let field_serde_opts = extract_field_serde_options(field)?;
    let field_eure_opts = extract_field_eure_options(field)?;
    
    // Check if field has serde(default) or if container has default
    let has_default = has_serde_default(field) || container_has_default;
    
    // Check if field is optional and extract inner type
    if let Some(inner_ty) = extract_option_inner_type(ty) {
        // It's an Option<T>, use T's schema and mark as optional
        Ok(quote! {
            {
                let mut schema = <#inner_ty as ::eure_schema::ToEureSchema>::eure_field_schema();
                schema.optional = true;
                #field_serde_opts
                #field_eure_opts
                schema
            }
        })
    } else if has_default {
        // Field has default, so it's optional in EURE schema
        Ok(quote! {
            {
                let mut schema = <#ty as ::eure_schema::ToEureSchema>::eure_field_schema();
                schema.optional = true;
                #field_serde_opts
                #field_eure_opts
                schema
            }
        })
    } else {
        // Not an Option and no default, use the type directly
        Ok(quote! {
            {
                let mut schema = <#ty as ::eure_schema::ToEureSchema>::eure_field_schema();
                #field_serde_opts
                #field_eure_opts
                schema
            }
        })
    }
}


#[allow(dead_code)]
fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        // Check if it's core::option::Option or std::option::Option
        let is_full_path = type_path.path.segments.len() >= 3
            && type_path.path.segments[type_path.path.segments.len() - 3].ident == "core"
            || type_path.path.segments[type_path.path.segments.len() - 3].ident == "std"
            && type_path.path.segments[type_path.path.segments.len() - 2].ident == "option"
            && type_path.path.segments.last().unwrap().ident == "Option";
            
        // Check if it's just Option
        let is_simple = type_path.path.segments.len() == 1
            && type_path.path.segments[0].ident == "Option";
            
        // Check if it's option::Option
        let is_module = type_path.path.segments.len() == 2
            && type_path.path.segments[0].ident == "option"
            && type_path.path.segments[1].ident == "Option";
            
        return is_full_path || is_simple || is_module;
    }
    false
}

/// Extract the inner type from Option<T>
fn extract_option_inner_type(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
            && segment.ident == "Option"
                && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
                    && let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner);
                    }
    None
}

fn extract_serde_options(attrs: &[Attribute]) -> syn::Result<proc_macro2::TokenStream> {
    let mut rename_all = None;
    
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename_all") {
                let value: Lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = value {
                    rename_all = Some(lit_str.value());
                }
            }
            Ok(())
        })?;
    }
    
    let rename_all_token = if let Some(rule) = rename_all {
        
        match rule.as_str() {
            "camelCase" => quote! { Some(::eure_schema::RenameRule::CamelCase) },
            "snake_case" => quote! { Some(::eure_schema::RenameRule::SnakeCase) },
            "kebab-case" => quote! { Some(::eure_schema::RenameRule::KebabCase) },
            "PascalCase" => quote! { Some(::eure_schema::RenameRule::PascalCase) },
            "lowercase" => quote! { Some(::eure_schema::RenameRule::Lowercase) },
            "UPPERCASE" => quote! { Some(::eure_schema::RenameRule::Uppercase) },
            _ => quote! { None },
        }
    } else {
        quote! { None }
    };
    
    Ok(quote! {
        ::eure_schema::SerdeOptions {
            rename: None,
            rename_all: #rename_all_token,
        }
    })
}

fn extract_field_serde_options(field: &syn::Field) -> syn::Result<proc_macro2::TokenStream> {
    let rename = extract_field_rename(field)?;
    
    if let Some(rename) = rename {
        Ok(quote! {
            schema.serde.rename = Some(#rename.to_string());
        })
    } else {
        Ok(quote! {})
    }
}

fn extract_field_rename(field: &syn::Field) -> syn::Result<Option<String>> {
    for attr in &field.attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        
        let mut rename = None;
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let value: Lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = value {
                    rename = Some(lit_str.value());
                }
            }
            Ok(())
        })?;
        
        if rename.is_some() {
            return Ok(rename);
        }
    }
    
    Ok(None)
}

fn extract_variant_rename(variant: &syn::Variant) -> syn::Result<Option<String>> {
    for attr in &variant.attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        
        let mut rename = None;
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let value: Lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = value {
                    rename = Some(lit_str.value());
                }
            }
            Ok(())
        })?;
        
        if rename.is_some() {
            return Ok(rename);
        }
    }
    
    Ok(None)
}

fn extract_variant_rename_all(variant: &syn::Variant) -> syn::Result<Option<String>> {
    for attr in &variant.attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        
        let mut rename_all = None;
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename_all") {
                let value: Lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = value {
                    rename_all = Some(lit_str.value());
                }
            }
            Ok(())
        })?;
        
        if rename_all.is_some() {
            return Ok(rename_all);
        }
    }
    
    Ok(None)
}

fn extract_rename_all_fields_rule(attrs: &[Attribute]) -> syn::Result<Option<String>> {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        
        let mut rename_all_fields = None;
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename_all_fields") {
                // Handle both direct value and serialize/deserialize variants
                if meta.input.peek(syn::Token![=]) {
                    // Direct: #[serde(rename_all_fields = "camelCase")]
                    let value: Lit = meta.value()?.parse()?;
                    if let Lit::Str(lit_str) = value {
                        rename_all_fields = Some(lit_str.value());
                    }
                } else {
                    // With serialize/deserialize: #[serde(rename_all_fields(serialize = "...", deserialize = "..."))]
                    // For EURE schema, we'll use the serialize variant
                    let content;
                    syn::parenthesized!(content in meta.input);
                    while !content.is_empty() {
                        let ident: syn::Ident = content.parse()?;
                        content.parse::<syn::Token![=]>()?;
                        let value: Lit = content.parse()?;
                        
                        if ident == "serialize"
                            && let Lit::Str(lit_str) = value {
                                rename_all_fields = Some(lit_str.value());
                            }
                        
                        if !content.is_empty() {
                            content.parse::<syn::Token![,]>()?;
                        }
                    }
                }
            }
            Ok(())
        })?;
        
        if rename_all_fields.is_some() {
            return Ok(rename_all_fields);
        }
    }
    
    Ok(None)
}

fn extract_field_eure_options(field: &syn::Field) -> syn::Result<proc_macro2::TokenStream> {
    let mut tokens = Vec::new();
    
    for attr in &field.attrs {
        if !attr.path().is_ident("eure") {
            continue;
        }
        
        attr.parse_nested_meta(|meta| {
            // Handle length(min = X, max = Y)
            if meta.path.is_ident("length") {
                let content;
                syn::parenthesized!(content in meta.input);
                let mut min = None;
                let mut max = None;
                
                while !content.is_empty() {
                    let ident: syn::Ident = content.parse()?;
                    content.parse::<syn::Token![=]>()?;
                    let value: syn::LitInt = content.parse()?;
                    
                    if ident == "min" {
                        min = Some(value);
                    } else if ident == "max" {
                        max = Some(value);
                    }
                    
                    if !content.is_empty() {
                        content.parse::<syn::Token![,]>()?;
                    }
                }
                
                let min_tok = min.map(|v| quote! { Some(#v) }).unwrap_or(quote! { None });
                let max_tok = max.map(|v| quote! { Some(#v) }).unwrap_or(quote! { None });
                tokens.push(quote! {
                    schema.constraints.length = Some((#min_tok, #max_tok));
                });
            }
            
            // Handle pattern = "regex"
            if meta.path.is_ident("pattern") {
                let value: Lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = value {
                    let pattern = lit_str.value();
                    tokens.push(quote! {
                        schema.constraints.pattern = Some(#pattern.to_string());
                    });
                }
            }
            
            // Handle range(min = X, max = Y)
            if meta.path.is_ident("range") {
                let content;
                syn::parenthesized!(content in meta.input);
                let mut min = None;
                let mut max = None;
                
                while !content.is_empty() {
                    let ident: syn::Ident = content.parse()?;
                    content.parse::<syn::Token![=]>()?;
                    let value: syn::LitFloat = content.parse()?;
                    
                    if ident == "min" {
                        min = Some(value);
                    } else if ident == "max" {
                        max = Some(value);
                    }
                    
                    if !content.is_empty() {
                        content.parse::<syn::Token![,]>()?;
                    }
                }
                
                let min_tok = min.map(|v| quote! { Some(#v) }).unwrap_or(quote! { None });
                let max_tok = max.map(|v| quote! { Some(#v) }).unwrap_or(quote! { None });
                tokens.push(quote! {
                    schema.constraints.range = Some((#min_tok, #max_tok));
                });
            }
            
            // min_items and max_items have been removed per language designer
            
            // Handle unique = true/false
            if meta.path.is_ident("unique") {
                let value: syn::LitBool = meta.value()?.parse()?;
                tokens.push(quote! {
                    schema.constraints.unique = Some(#value);
                });
            }
            
            // Handle prefer_section = true/false
            if meta.path.is_ident("prefer_section") {
                let value: syn::LitBool = meta.value()?.parse()?;
                tokens.push(quote! {
                    schema.preferences.section = Some(#value);
                });
            }
            
            // Handle description = "..."
            if meta.path.is_ident("description") {
                let value: Lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = value {
                    let description = lit_str.value();
                    tokens.push(quote! {
                        schema.description = Some(#description.to_string());
                    });
                }
            }
            
            Ok(())
        })?;
    }
    
    Ok(quote! {
        #(#tokens)*
    })
}

fn extract_variant_representation(attrs: &[Attribute]) -> syn::Result<proc_macro2::TokenStream> {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        
        let mut untagged = false;
        let mut tag = None;
        let mut content = None;
        
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("untagged") {
                untagged = true;
            } else if meta.path.is_ident("tag") {
                let value: Lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = value {
                    tag = Some(lit_str.value());
                }
            } else if meta.path.is_ident("content") {
                let value: Lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = value {
                    content = Some(lit_str.value());
                }
            }
            Ok(())
        })?;
        
        if untagged {
            return Ok(quote! { ::eure_schema::VariantRepr::Untagged });
        } else if let Some(tag) = tag {
            if let Some(content) = content {
                return Ok(quote! { 
                    ::eure_schema::VariantRepr::AdjacentlyTagged { 
                        tag: #tag.to_string(), 
                        content: #content.to_string() 
                    } 
                });
            } else {
                return Ok(quote! { 
                    ::eure_schema::VariantRepr::InternallyTagged { 
                        tag: #tag.to_string() 
                    } 
                });
            }
        }
    }
    
    Ok(quote! { ::eure_schema::VariantRepr::Tagged })
}

fn has_serde_skip(field: &syn::Field) -> bool {
    for attr in &field.attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        
        let mut has_skip = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                has_skip = true;
            }
            Ok(())
        });
        
        if has_skip {
            return true;
        }
    }
    false
}

fn has_serde_flatten(field: &syn::Field) -> bool {
    for attr in &field.attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        
        let mut has_flatten = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("flatten") {
                has_flatten = true;
            }
            Ok(())
        });
        
        if has_flatten {
            return true;
        }
    }
    false
}

fn has_serde_default(field: &syn::Field) -> bool {
    for attr in &field.attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        
        let mut has_default = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("default") {
                has_default = true;
            }
            Ok(())
        });
        
        if has_default {
            return true;
        }
    }
    false
}

fn has_container_serde_default(attrs: &[Attribute]) -> bool {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        
        let mut has_default = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("default") {
                has_default = true;
            }
            Ok(())
        });
        
        if has_default {
            return true;
        }
    }
    false
}

fn has_serde_transparent(attrs: &[Attribute]) -> bool {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        
        let mut has_transparent = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("transparent") {
                has_transparent = true;
            }
            Ok(())
        });
        
        if has_transparent {
            return true;
        }
    }
    false
}


fn extract_rename_all_rule(attrs: &[Attribute]) -> syn::Result<Option<String>> {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        
        let mut rename_all = None;
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename_all") {
                let value: Lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = value {
                    rename_all = Some(lit_str.value());
                }
            }
            Ok(())
        })?;
        
        if rename_all.is_some() {
            return Ok(rename_all);
        }
    }
    
    Ok(None)
}


fn apply_rename_rule(
    name: &str,
    explicit_rename: &Option<String>,
    rename_all: &Option<String>,
) -> String {
    if let Some(rename) = explicit_rename {
        return rename.clone();
    }
    
    if let Some(rule) = rename_all {
        match rule.as_str() {
            "camelCase" => name.to_case(Case::Camel),
            "snake_case" => name.to_case(Case::Snake),
            "kebab-case" => name.to_case(Case::Kebab),
            "PascalCase" => name.to_case(Case::Pascal),
            "lowercase" => name.to_lowercase(),
            "UPPERCASE" => name.to_uppercase(),
            _ => name.to_string(),
        }
    } else {
        name.to_string()
    }
}

// Case conversion is now handled by the convert_case crate

