//! BuildSchema derive implementation for structs (records)

use darling::FromField;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DataStruct, Fields};

use crate::attrs::FieldAttrs;
use crate::context::MacroContext;

pub fn generate_record_schema(context: &MacroContext, input: &DataStruct) -> TokenStream {
    match &input.fields {
        Fields::Named(fields) => generate_named_struct(context, &fields.named),
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            generate_newtype_struct(context, &fields.unnamed[0].ty)
        }
        Fields::Unnamed(fields) => generate_tuple_struct(context, &fields.unnamed),
        Fields::Unit => generate_unit_struct(context),
    }
}

fn generate_named_struct(
    context: &MacroContext,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> TokenStream {
    let schema_crate = context.schema_crate();

    let field_schemas: Vec<_> = fields
        .iter()
        .enumerate()
        .map(|(idx, f)| {
            let field_name = f.ident.as_ref().expect("named fields must have names");
            let field_ty = &f.ty;
            let attrs = FieldAttrs::from_field(f).expect("failed to parse field attributes");

            // Skip flatten fields for now - they need special handling
            if attrs.flatten || attrs.flatten_ext {
                panic!("flatten is not yet supported in BuildSchema derive");
            }

            let field_name_str = attrs
                .rename
                .clone()
                .unwrap_or_else(|| context.apply_rename(&field_name.to_string()));

            let schema_var = format_ident!("field_{}_schema", idx);

            // Check if the field is Option<T> to mark as optional
            let is_optional = is_option_type(field_ty);

            (field_name_str, schema_var, field_ty.clone(), is_optional)
        })
        .collect();

    // Generate schema building for each field
    let field_builds: Vec<_> = field_schemas
        .iter()
        .map(|(_, schema_var, field_ty, _)| {
            quote! {
                let #schema_var = ctx.build::<#field_ty>();
            }
        })
        .collect();

    // Generate the properties HashMap entries
    let properties_entries: Vec<_> = field_schemas
        .iter()
        .map(|(name, schema_var, _, is_optional)| {
            quote! {
                (
                    #name.to_string(),
                    #schema_crate::RecordFieldSchema {
                        schema: #schema_var,
                        optional: #is_optional,
                        binding_style: None,
                    }
                )
            }
        })
        .collect();

    // Determine unknown fields policy
    let unknown_fields_policy = if context.config.allow_unknown_fields {
        quote! { #schema_crate::UnknownFieldsPolicy::Allow }
    } else {
        quote! { #schema_crate::UnknownFieldsPolicy::Deny }
    };

    let content = quote! {
        #(#field_builds)*

        #schema_crate::SchemaNodeContent::Record(#schema_crate::RecordSchema {
            properties: [
                #(#properties_entries),*
            ].into_iter().collect(),
            unknown_fields: #unknown_fields_policy,
        })
    };

    context.impl_build_schema(content)
}

fn generate_unit_struct(context: &MacroContext) -> TokenStream {
    let schema_crate = context.schema_crate();
    let content = quote! {
        #schema_crate::SchemaNodeContent::Null
    };
    context.impl_build_schema(content)
}

fn generate_tuple_struct(
    context: &MacroContext,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> TokenStream {
    let schema_crate = context.schema_crate();

    let field_builds: Vec<_> = fields
        .iter()
        .enumerate()
        .map(|(idx, f)| {
            let field_ty = &f.ty;
            let schema_var = format_ident!("field_{}_schema", idx);
            quote! {
                let #schema_var = ctx.build::<#field_ty>();
            }
        })
        .collect();

    let schema_vars: Vec<_> = (0..fields.len())
        .map(|idx| format_ident!("field_{}_schema", idx))
        .collect();

    let content = quote! {
        #(#field_builds)*

        #schema_crate::SchemaNodeContent::Tuple(#schema_crate::TupleSchema {
            elements: vec![#(#schema_vars),*],
            binding_style: None,
        })
    };

    context.impl_build_schema(content)
}

fn generate_newtype_struct(context: &MacroContext, field_ty: &syn::Type) -> TokenStream {
    // Newtype just delegates to the inner type
    let content = quote! {
        <#field_ty as BuildSchema>::build_schema(ctx)
    };
    context.impl_build_schema(content)
}

/// Check if a type is Option<T>
fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        return segment.ident == "Option";
    }
    false
}
