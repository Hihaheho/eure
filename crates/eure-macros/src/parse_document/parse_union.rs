use proc_macro2::TokenStream;
use quote::quote;
use syn::{DataEnum, Field, Generics, Ident, Variant};

use crate::{config::MacroConfig, context::MacroContext};

pub fn generate_union_parser(context: &MacroContext, input: &DataEnum) -> TokenStream {
    let ident = context.ident();
    let MacroConfig { document_crate, .. } = &context.config;
    let DataEnum {
        enum_token,
        brace_token,
        variants,
    } = input;
    let impl_generics = context.impl_generics();
    let for_generics = context.for_generics();
    let variant_repr = variant_repr(&context);
    let variants = variants
        .iter()
        .map(|variant| generate_variant(&context, variant));
    quote! {
        impl<'doc, #(#impl_generics),*> #document_crate::parse::ParseDocument<'doc> for #ident<#(#for_generics),*> {
            type Error = #document_crate::parse::ParseError;

            fn parse(ctx: &#document_crate::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
                ctx.parse_union(#variant_repr)
                #(#variants)*
            }
        }
    }
}

fn variant_repr(context: &MacroContext) -> TokenStream {
    todo!()
}

fn generate_variant(context: &MacroContext, field: &Variant) -> TokenStream {
    quote! {
        .variant()
    }
}
