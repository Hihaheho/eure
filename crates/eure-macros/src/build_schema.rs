//! BuildSchema derive macro implementation

mod emit_ir;

use proc_macro2::TokenStream;

use crate::codegen_ir_adapter::{self, DeriveIrArtifacts};
use crate::context::MacroContext;

pub fn derive(context: MacroContext) -> TokenStream {
    derive_inner(&context).unwrap_or_else(syn::Error::into_compile_error)
}

fn derive_inner(context: &MacroContext) -> syn::Result<TokenStream> {
    let artifacts = codegen_ir_adapter::derive_input_to_ir_artifacts(&context.input)?;
    derive_ir(&artifacts)
}

pub(crate) fn derive_ir(artifacts: &DeriveIrArtifacts) -> syn::Result<TokenStream> {
    emit_ir::derive(&artifacts.module, &artifacts.spans)
}
