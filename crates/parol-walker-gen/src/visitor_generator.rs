use std::collections::BTreeMap;
use std::path::PathBuf;

use parol::generators::export_node_types::{
    Child, NodeName, NodeTypesInfo, NonTerminalInfo, NonTerminalStructure, TerminalInfo,
};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::format_snake_case;

struct GenField {
    field_name_ident: syn::Ident,
    is_non_terminal: bool,
    original_name: String,
}

#[allow(dead_code)]
fn get_terminal_by_name<'a>(info: &'a NodeTypesInfo, name: &str) -> &'a TerminalInfo {
    info.terminals
        .iter()
        .find(|t| t.name == name)
        .unwrap_or_else(|| panic!("Terminal {name} not found"))
}

pub struct VisitorGenerator {
    path: PathBuf,
    #[allow(dead_code)]
    config: crate::WalkerConfig,
}

impl VisitorGenerator {
    pub fn new(path: PathBuf, config: crate::WalkerConfig) -> Self {
        Self { path, config }
    }

    pub fn generate(&self, node_info: &NodeTypesInfo) {
        let imports = self.generate_imports();
        let cst_visitor_trait = self.generate_cst_visitor_trait(node_info);
        let cst_handle_super_trait = self.generate_cst_handle_super_trait(node_info);
        let cst_handle_super_impl = self.generate_cst_handle_super_impl(node_info);
        let node_visitor = self.generate_node_visitor();
        let builtin_terminal_visitor = self.generate_builtin_terminal_visitor();

        let generated_code = quote! {
            #imports
            #cst_visitor_trait
            #cst_handle_super_trait
            #cst_handle_super_impl
            #node_visitor
            #builtin_terminal_visitor
        };

        let syn_file = syn::parse_file(&generated_code.to_string())
            .expect("Failed to parse generated visitor code");
        std::fs::write(&self.path, prettyplease::unparse(&syn_file))
            .expect("Failed to write generated visitor file");
    }

    fn generate_imports(&self) -> TokenStream {
        let header = crate::generate_header_comment();
        let node_kind_module = &self.config.imports.node_kind_module;
        let nodes_module = &self.config.imports.nodes_module;

        // Parse module paths to create proper use statements
        let runtime_use = syn::parse_str::<syn::Path>(&self.config.imports.runtime_crate).unwrap();
        let node_kind_use = syn::parse_str::<syn::Path>(node_kind_module).unwrap();
        let nodes_use = syn::parse_str::<syn::Path>(nodes_module).unwrap();

        // Import from runtime - use parol_walker::CstFacade, and import BuiltinTerminalVisitor for adapter
        quote! {
            #header
            #[allow(unused_imports)]
            use #runtime_use::{
                CstNodeId, TerminalData, NonTerminalData,
                TerminalHandle as _, NonTerminalHandle as _,
                CstConstructError, NodeKind,
                CstNodeData, CstFacade,
                BuiltinTerminalVisitor as PwBuiltinTerminalVisitor,
            };
            use #node_kind_use::{TerminalKind, NonTerminalKind};
            use #nodes_use::*;

            /// Type alias for CstNodeData with grammar-specific types
            pub type CstNode = CstNodeData<TerminalKind, NonTerminalKind>;
            /// Type alias for the CST
            pub type Cst = super::tree::Cst;
        }
    }

    fn get_view_param_type(&self, nt_info: &NonTerminalInfo) -> TokenStream {
        let view_name_ident = format_ident!("{}View", nt_info.name);
        match &nt_info.structure {
            NonTerminalStructure::Sequence(_) | NonTerminalStructure::OneOf(_) => {
                quote!(#view_name_ident)
            }
            NonTerminalStructure::Recursion(_) => {
                quote!(#view_name_ident)
            }
            NonTerminalStructure::Option(children) => {
                if children.len() > 1 {
                    quote!(#view_name_ident)
                } else {
                    let child_node_name = &children[0].name;
                    match child_node_name {
                        NodeName::Terminal(name) => {
                            let terminal_ident = format_ident!("{}", name.0);
                            quote!(#terminal_ident)
                        }
                        NodeName::NonTerminal(name) => {
                            let child_handle_ident = format_ident!("{}Handle", name.0);
                            quote!(#child_handle_ident)
                        }
                    }
                }
            }
        }
    }

    fn generate_cst_visitor_trait(&self, node_info: &NodeTypesInfo) -> TokenStream {
        let nt_visit_methods = node_info.non_terminals.iter().map(|nt| {
            let fn_name_ident = crate::format_snake_case(&format!("visit_{}", nt.name));
            let fn_name_super_ident = crate::format_snake_case(&format!("visit_{}_super", nt.name));
            let handle_type_ident = format_ident!("{}Handle", nt.name);
            let view_param_type = self.get_view_param_type(nt);

            quote! {
                fn #fn_name_ident(
                    &mut self,
                    handle: #handle_type_ident,
                    view: #view_param_type,
                    tree: &F,
                ) -> Result<(), Self::Error> {
                    self.#fn_name_super_ident(handle, view, tree)
                }
            }
        });
        let terminal_visit_methods = node_info.terminals.iter().map(|t| {
            let fn_name_ident = crate::format_snake_case(&format!("visit_{}_terminal", t.name));
            let fn_name_super_ident =
                crate::format_snake_case(&format!("visit_{}_terminal_super", t.name));
            let terminal_ident = format_ident!("{}", t.name);
            quote! {
                fn #fn_name_ident(
                    &mut self,
                    terminal: #terminal_ident,
                    data: TerminalData,
                    tree: &F,
                ) -> Result<(), Self::Error> {
                    self.#fn_name_super_ident(terminal, data, tree)
                }
            }
        });

        quote! {
            pub trait CstVisitor<F: CstFacade<TerminalKind, NonTerminalKind>>: CstVisitorSuper<F, Self::Error> {
                type Error;
                #(#nt_visit_methods)*
                #(#terminal_visit_methods)*
                fn visit_non_terminal(&mut self, id: CstNodeId, kind: NonTerminalKind, data: NonTerminalData, tree: &F) -> Result<(), Self::Error> {
                    self.visit_non_terminal_super(id, kind, data, tree)
                }
                fn visit_non_terminal_close(&mut self, id: CstNodeId, kind: NonTerminalKind, data: NonTerminalData, tree: &F) -> Result<(), Self::Error> {
                    self.visit_non_terminal_close_super(id, kind, data, tree)
                }
                fn visit_terminal(&mut self, id: CstNodeId, kind: TerminalKind, data: TerminalData, tree: &F) -> Result<(), Self::Error> {
                    self.visit_terminal_super(id, kind, data, tree)
                }
                /// This method is called when a construct view fails.
                /// If you return Ok(()), the error is not propagated.
                fn then_construct_error(&mut self, node_data: Option<CstNode>, parent: CstNodeId, kind: NodeKind<TerminalKind, NonTerminalKind>, error: CstConstructError<TerminalKind, NonTerminalKind, Self::Error>, tree: &F) -> Result<(), Self::Error> {
                    let _error = error;
                    self.recover_error(node_data, parent, kind, tree)
                }
            }
        }
    }

    fn generate_cst_handle_super_trait(&self, node_info: &NodeTypesInfo) -> TokenStream {
        let methods = node_info
            .non_terminals
            .iter()
            .flat_map(|nt| {
                let visit_handle_fn_name =
                    crate::format_snake_case(&format!("visit_{}_handle", nt.name));
                let visit_super_fn_name =
                    crate::format_snake_case(&format!("visit_{}_super", nt.name));
                let handle_type_ident = format_ident!("{}Handle", nt.name);
                let view_param_type = self.get_view_param_type(nt);

                let visit_handle_method = quote! {
                    fn #visit_handle_fn_name(
                        &mut self,
                        handle: #handle_type_ident,
                        tree: &F,
                    ) -> Result<(), E>;
                };

                let visit_super_method = quote! {
                    fn #visit_super_fn_name(
                        &mut self,
                        handle: #handle_type_ident,
                        view: #view_param_type,
                        tree: &F,
                    ) -> Result<(), E>;
                };
                vec![visit_handle_method, visit_super_method]
            })
            .collect::<Vec<TokenStream>>();

        let terminal_methods = node_info.terminals.iter().map(|t| {
            let fn_name_super_ident =
                crate::format_snake_case(&format!("visit_{}_terminal_super", t.name));
            let terminal_ident = format_ident!("{}", t.name);
            quote! {
                fn #fn_name_super_ident(
                    &mut self,
                    terminal: #terminal_ident,
                    data: TerminalData,
                    tree: &F,
                ) -> Result<(), E>;
            }
        });

        quote! {
            mod private {
                #[allow(unused_imports)]
                use super::*;
                pub trait Sealed<F> {}
            }
            pub trait CstVisitorSuper<F: CstFacade<TerminalKind, NonTerminalKind>, E>: private::Sealed<F> {
                #(#methods)*
                #(#terminal_methods)*
                fn visit_non_terminal_super(&mut self, id: CstNodeId, kind: NonTerminalKind, data: NonTerminalData, tree: &F) -> Result<(), E>;
                fn visit_non_terminal_close_super(&mut self, id: CstNodeId, kind: NonTerminalKind, data: NonTerminalData, tree: &F) -> Result<(), E>;
                fn visit_terminal_super(&mut self, id: CstNodeId, kind: TerminalKind, data: TerminalData, tree: &F) -> Result<(), E>;
                fn visit_any(&mut self, id: CstNodeId, node: CstNode, tree: &F) -> Result<(), E>;
                /// Recover from a construct error. This eagerly visits the children of the node.
                fn recover_error(&mut self, node_data: Option<CstNode>, id: CstNodeId, kind: NodeKind<TerminalKind, NonTerminalKind>, tree: &F) -> Result<(), E>;
            }
        }
    }

    fn generate_cst_handle_super_impl(&self, node_info: &NodeTypesInfo) -> TokenStream {
        let visit_handle_impls = node_info
            .non_terminals
            .iter()
            .map(|nt| self.generate_visit_handle_impl_method(nt));

        let visit_super_impls = node_info
            .non_terminals
            .iter()
            .map(|nt| self.generate_visit_super_impl_method(nt));

        let terminal_visit_super_impls = node_info
            .terminals
            .iter()
            .map(|t| self.generate_terminal_visit_super_impl_method(t));
        let visit_any_impl = self.generate_visit_any(node_info);

        quote! {
            impl<V: CstVisitor<F>, F: CstFacade<TerminalKind, NonTerminalKind>> private::Sealed<F> for V {}
            impl<V: CstVisitor<F> + BuiltinTerminalVisitor<V::Error, F>, F: CstFacade<TerminalKind, NonTerminalKind>> CstVisitorSuper<F, V::Error> for V {
                #(#visit_handle_impls)*
                #(#visit_super_impls)*
                #(#terminal_visit_super_impls)*
                fn visit_non_terminal_super(&mut self, _id: CstNodeId, _kind: NonTerminalKind, _data: NonTerminalData, _tree: &F) -> Result<(), V::Error> {
                    Ok(())
                }
                fn visit_non_terminal_close_super(&mut self, _id: CstNodeId, _kind: NonTerminalKind, _data: NonTerminalData, _tree: &F) -> Result<(), V::Error> {
                    Ok(())
                }
                fn visit_terminal_super(&mut self, _id: CstNodeId, _kind: TerminalKind, _data: TerminalData, _tree: &F) -> Result<(), V::Error> {
                    Ok(())
                }
                fn recover_error(&mut self, node_data: Option<CstNode>, id: CstNodeId, kind: NodeKind<TerminalKind, NonTerminalKind>, tree: &F) -> Result<(), V::Error> {
                    let Some(node_data) = node_data else {
                        return Ok(());
                    };
                    if node_data.node_kind() == kind {
                        for child in tree.children(id) {
                            if let Some(node_data) = tree.node_data(child) {
                                self.visit_any(child, node_data, tree)?;
                            }
                        }
                    } else {
                        self.visit_any(id, node_data, tree)?;
                    }
                    Ok(())
                }
                #visit_any_impl
            }
        }
    }

    fn generate_visit_handle_impl_method(&self, nt_info: &NonTerminalInfo) -> TokenStream {
        let fn_name_handle_ident =
            crate::format_snake_case(&format!("visit_{}_handle", nt_info.name));
        let visitor_method_name = crate::format_snake_case(&format!("visit_{}", nt_info.name));
        let handle_type_ident = format_ident!("{}Handle", nt_info.name);

        let on_view = match &nt_info.structure {
            NonTerminalStructure::Sequence(_) | NonTerminalStructure::OneOf(_) => {
                quote! {
                    visit.0.#visitor_method_name(handle, view, tree)
                }
            }
            NonTerminalStructure::Option(_) | NonTerminalStructure::Recursion(_) => {
                quote! {
                    if let Some(view) = view {
                        visit.0.#visitor_method_name(handle, view, tree)
                    } else {
                        Ok(())
                    }
                }
            }
        };

        quote! {
            fn #fn_name_handle_ident(
                &mut self,
                handle: #handle_type_ident,
                tree: &F,
            ) -> Result<(), V::Error> {
                let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
                    Ok(nt_data) => nt_data,
                    Err(error) => {
                        return self.then_construct_error(
                            None,
                            handle.node_id(),
                            NodeKind::NonTerminal(handle.kind()),
                            CstConstructError::ViewConstruction(error),
                            tree,
                        );
                    }
                };
                self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
                let mut adapter = BuiltinVisitorAdapter(self);
                let result = match handle.get_view_with_visit(tree, |view, visit: &mut BuiltinVisitorAdapter<'_, Self>| (#on_view, visit), &mut adapter) {
                    Ok(Ok(())) => Ok(()),
                    Ok(Err(e)) => Err(e),
                    Err(e) => adapter.0.then_construct_error(Some(CstNode::NonTerminal { kind: handle.kind(), data: nt_data }), handle.node_id(), NodeKind::NonTerminal(handle.kind()), e, tree),
                };
                adapter.0.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
                result
            }
        }
    }

    fn generate_terminal_visit_super_impl_method(&self, t_info: &TerminalInfo) -> TokenStream {
        let fn_name_super_ident =
            crate::format_snake_case(&format!("visit_{}_terminal_super", t_info.name));
        let terminal_ident = format_ident!("{}", t_info.name);
        quote! {
            fn #fn_name_super_ident(
                &mut self,
                terminal: #terminal_ident,
                data: TerminalData,
                tree: &F,
            ) -> Result<(), V::Error> {
                self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
                Ok(())
            }
        }
    }

    fn get_fields_for_children(&self, children: &[Child]) -> Vec<GenField> {
        let mut gen_fields = children
            .iter()
            .map(|child_prod_info| {
                let (name_str_ref, is_nt) = match &child_prod_info.name {
                    NodeName::Terminal(name) => (&name.0, false),
                    NodeName::NonTerminal(name) => (&name.0, true),
                };
                GenField {
                    field_name_ident: crate::format_snake_case(name_str_ref),
                    is_non_terminal: is_nt,
                    original_name: name_str_ref.to_string(),
                }
            })
            .collect::<Vec<_>>();

        let mut existing_names = BTreeMap::new();
        for field in &mut gen_fields {
            let base_name = field.field_name_ident.to_string();
            let count = existing_names.entry(base_name.clone()).or_insert(0u32);
            if *count > 0 {
                field.field_name_ident = format_ident!("{}{}", base_name, (*count + 1));
            }
            *count += 1;
        }
        gen_fields
    }

    fn generate_visit_super_impl_method(&self, nt_info: &NonTerminalInfo) -> TokenStream {
        let fn_name_super_ident =
            crate::format_snake_case(&format!("visit_{}_super", nt_info.name));
        let view_param_type = self.get_view_param_type(nt_info);
        let view_ident = format_ident!("view_param");

        let actual_view_type_name = format_ident!("{}View", nt_info.name);
        let handle_type_ident = format_ident!("{}Handle", nt_info.name);

        let body = match &nt_info.structure {
            NonTerminalStructure::Option(children) if children.len() == 1 => {
                let child_info = &children[0];
                let visit_call = match &child_info.name {
                    NodeName::NonTerminal(name) => {
                        let visit_child_handle_method =
                            crate::format_snake_case(&format!("visit_{}_handle", name.0));
                        quote! {
                            self.#visit_child_handle_method(#view_ident, tree)?;
                        }
                    }
                    NodeName::Terminal(name) => {
                        let visit_terminal_method =
                            crate::format_snake_case(&format!("visit_{}_terminal", name.0));
                        quote! {
                            self.#visit_terminal_method(#view_ident, data, tree)?;
                        }
                    }
                };
                quote! {
                    #visit_call
                    Ok(())
                }
            }
            NonTerminalStructure::Sequence(children)
            | NonTerminalStructure::Recursion(children)
            | NonTerminalStructure::Option(children) => {
                let view_fields = self.get_fields_for_children(children);
                let (field_names, visit_calls) = view_fields
                    .iter()
                    .map(|field_info| {
                        let child_handle_field_name = &field_info.field_name_ident;
                        let visit_call = if field_info.is_non_terminal {
                            let visit_child_handle_method = format_snake_case(&format!(
                                "visit_{}_handle",
                                field_info.original_name
                            ));
                            quote! {
                                self.#visit_child_handle_method(#child_handle_field_name, tree)?;
                            }
                        } else {
                            let visit_terminal_method = format_snake_case(&format!(
                                "visit_{}_terminal",
                                field_info.original_name
                            ));
                            quote! {
                                let data = match #child_handle_field_name.get_data(tree) {
                                    Ok(data) => data,
                                    Err(error) => return self.then_construct_error(None, #child_handle_field_name.0, NodeKind::Terminal(#child_handle_field_name.kind()), CstConstructError::ViewConstruction(error), tree),
                                };
                                self.#visit_terminal_method(#child_handle_field_name, data, tree)?;
                            }
                        };
                        (child_handle_field_name, visit_call)
                    })
                    .collect::<(Vec<_>, Vec<_>)>();
                quote! {
                    let #actual_view_type_name { #(#field_names),* } = #view_ident;
                    #(#visit_calls)*
                    Ok(())
                }
            }
            NonTerminalStructure::OneOf(alts) => {
                let variants_handling = alts.iter().enumerate().map(|(idx, alt_children)| {
                    if alt_children.len() == 1 {
                        // Single-element alternative: Variant(item)
                        let child_info = &alt_children[0];
                        let (child_name_str, is_child_nt) = match &child_info.name {
                            NodeName::Terminal(name) => (name.0.as_str(), false),
                            NodeName::NonTerminal(name) => (name.0.as_str(), true),
                        };
                        let variant_name_ident = format_ident!("{}", child_name_str);

                        if is_child_nt {
                            let visit_child_handle_method =
                                crate::format_snake_case(&format!("visit_{child_name_str}_handle"));
                            quote! {
                                #actual_view_type_name::#variant_name_ident(item) => {
                                    self.#visit_child_handle_method(item, tree)?;
                                }
                            }
                        } else {
                            let visit_terminal_method =
                                crate::format_snake_case(&format!("visit_{child_name_str}_terminal"));
                            quote! {
                                #actual_view_type_name::#variant_name_ident(item) => {
                                    let data = match item.get_data(tree) {
                                        Ok(data) => data,
                                        Err(error) => return self.then_construct_error(None, item.0, NodeKind::Terminal(item.kind()), CstConstructError::ViewConstruction(error), tree),
                                    };
                                    self.#visit_terminal_method(item, data, tree)?;
                                }
                            }
                        }
                    } else {
                        // Multi-element alternative: AltN(struct)
                        let variant_name_ident = format_ident!("Alt{}", idx);
                        let view_fields = self.get_fields_for_children(alt_children);
                        let (field_names, visit_calls): (Vec<_>, Vec<_>) = view_fields
                            .iter()
                            .map(|field_info| {
                                let child_handle_field_name = &field_info.field_name_ident;
                                let visit_call = if field_info.is_non_terminal {
                                    let visit_child_handle_method = format_snake_case(&format!(
                                        "visit_{}_handle",
                                        field_info.original_name
                                    ));
                                    quote! {
                                        self.#visit_child_handle_method(#child_handle_field_name, tree)?;
                                    }
                                } else {
                                    let visit_terminal_method = format_snake_case(&format!(
                                        "visit_{}_terminal",
                                        field_info.original_name
                                    ));
                                    quote! {
                                        let data = match #child_handle_field_name.get_data(tree) {
                                            Ok(data) => data,
                                            Err(error) => return self.then_construct_error(None, #child_handle_field_name.0, NodeKind::Terminal(#child_handle_field_name.kind()), CstConstructError::ViewConstruction(error), tree),
                                        };
                                        self.#visit_terminal_method(#child_handle_field_name, data, tree)?;
                                    }
                                };
                                (child_handle_field_name, visit_call)
                            })
                            .unzip();

                        let alt_struct_name = format_ident!("{}Alt{}", nt_info.name, idx);
                        quote! {
                            #actual_view_type_name::#variant_name_ident(alt_struct) => {
                                let #alt_struct_name { #(#field_names),* } = alt_struct;
                                #(#visit_calls)*
                            }
                        }
                    }
                });
                quote! {
                    match #view_ident {
                        #(#variants_handling)*
                    }
                    Ok(())
                }
            }
        };

        quote! {
            fn #fn_name_super_ident(
                &mut self,
                handle: #handle_type_ident,
                #view_ident: #view_param_type,
                tree: &F,
            ) -> Result<(), V::Error> {
                let _handle = handle;
                #body
            }
        }
    }

    fn generate_visit_any(&self, node_info: &NodeTypesInfo) -> TokenStream {
        let non_terminal_arms = node_info.non_terminals.iter().map(|nt| {
            let nt_kind_variant = format_ident!("{}", nt.variant);
            let nt_handle_type = format_ident!("{}Handle", nt.name);
            let visit_handle_method =
                crate::format_snake_case(&format!("visit_{}_handle", nt.name));
            quote! {
                NonTerminalKind::#nt_kind_variant => {
                    let handle = #nt_handle_type(id);
                    self.#visit_handle_method(handle, tree)?;
                }
            }
        });

        let terminal_arms = node_info.terminals.iter().map(|t| {
            let t_kind_variant = format_ident!("{}", t.variant);
            let t_type = format_ident!("{}", t.name);
            let visit_terminal_method =
                crate::format_snake_case(&format!("visit_{}_terminal", t.name));
            quote! {
                TerminalKind::#t_kind_variant => {
                    let terminal = #t_type(id);
                    self.#visit_terminal_method(terminal, data, tree)?;
                }
            }
        });

        quote! {
            fn visit_any(&mut self, id: CstNodeId, node: CstNode, tree: &F) -> Result<(), V::Error> {
                match node {
                    CstNode::NonTerminal { kind, .. } => {
                        match kind {
                            #(#non_terminal_arms)*
                        }
                    }
                    CstNode::Terminal { kind, data } => {
                        match kind {
                            #(#terminal_arms)*
                        }
                    }
                }
                Ok(())
            }
        }
    }

    fn generate_node_visitor(&self) -> TokenStream {
        quote! {
            mod private2 {
                pub trait Sealed {}
            }

            pub trait NodeVisitor: NodeVisitorSuper<Self::Error> {
                type Error;
                fn visit_node(&mut self, id: CstNodeId, node: CstNode, tree: &Cst) -> Result<(), Self::Error>;
            }

            pub trait NodeVisitorSuper<E>: private2::Sealed {
                fn visit_node_id(&mut self, id: CstNodeId, tree: &Cst) -> Result<(), E>;
                fn visit_node_super(&mut self, id: CstNodeId, node: CstNode, tree: &Cst) -> Result<(), E>;
            }

            impl<V: NodeVisitor> private2::Sealed for V {}

            impl<V: NodeVisitor> NodeVisitorSuper<V::Error> for V {
                fn visit_node_id(
                    &mut self,
                    id: CstNodeId,
                    tree: &Cst,
                ) -> Result<(), V::Error> {
                    if let Some(node) = tree.node_data(id) {
                        self.visit_node(id, node, tree)
                    } else {
                        Ok(())
                    }
                }
                fn visit_node_super(
                    &mut self,
                    id: CstNodeId,
                    _node: CstNode,
                    tree: &Cst,
                ) -> Result<(), V::Error> {
                    for child in tree.children(id) {
                        if let Some(child_node) = tree.node_data(child) {
                            self.visit_node(child, child_node, tree)?;
                        }
                    }
                    Ok(())
                }
            }
        }
    }

    fn generate_builtin_terminal_visitor(&self) -> TokenStream {
        // Generate local BuiltinTerminalVisitor trait and an adapter wrapper.
        // The adapter bridges local CstVisitor implementations to parol_walker::BuiltinTerminalVisitor.
        quote! {
            /// Trait for visiting builtin terminal tokens (whitespace, newline, comments).
            /// This is a grammar-specific version to avoid orphan rule issues.
            pub trait BuiltinTerminalVisitor<E, F: CstFacade<TerminalKind, NonTerminalKind>> {
                fn visit_builtin_whitespace_terminal(
                    &mut self,
                    node: CstNodeId,
                    data: TerminalData,
                    tree: &F,
                ) -> Result<(), E>;

                fn visit_builtin_new_line_terminal(
                    &mut self,
                    node: CstNodeId,
                    data: TerminalData,
                    tree: &F,
                ) -> Result<(), E>;

                fn visit_builtin_line_comment_terminal(
                    &mut self,
                    node: CstNodeId,
                    data: TerminalData,
                    tree: &F,
                ) -> Result<(), E>;

                fn visit_builtin_block_comment_terminal(
                    &mut self,
                    node: CstNodeId,
                    data: TerminalData,
                    tree: &F,
                ) -> Result<(), E>;
            }

            // Blanket impl that delegates BuiltinTerminalVisitor methods to CstVisitor methods.
            impl<V: CstVisitor<F>, F: CstFacade<TerminalKind, NonTerminalKind>> BuiltinTerminalVisitor<V::Error, F> for V {
                fn visit_builtin_whitespace_terminal(
                    &mut self,
                    node: CstNodeId,
                    data: TerminalData,
                    tree: &F,
                ) -> Result<(), V::Error> {
                    self.visit_whitespace_terminal(Whitespace(node), data, tree)
                }

                fn visit_builtin_new_line_terminal(
                    &mut self,
                    node: CstNodeId,
                    data: TerminalData,
                    tree: &F,
                ) -> Result<(), V::Error> {
                    self.visit_new_line_terminal(NewLine(node), data, tree)
                }

                fn visit_builtin_line_comment_terminal(
                    &mut self,
                    node: CstNodeId,
                    data: TerminalData,
                    tree: &F,
                ) -> Result<(), V::Error> {
                    self.visit_line_comment_terminal(LineComment(node), data, tree)
                }

                fn visit_builtin_block_comment_terminal(
                    &mut self,
                    node: CstNodeId,
                    data: TerminalData,
                    tree: &F,
                ) -> Result<(), V::Error> {
                    self.visit_block_comment_terminal(BlockComment(node), data, tree)
                }
            }

            /// Wrapper adapter that bridges local CstVisitor to parol_walker::BuiltinTerminalVisitor.
            /// This is needed because we can't directly impl the foreign trait for generic V due to orphan rules.
            #[repr(transparent)]
            pub struct BuiltinVisitorAdapter<'a, V>(pub &'a mut V);

            impl<'a, V: CstVisitor<F>, F: CstFacade<TerminalKind, NonTerminalKind>>
                PwBuiltinTerminalVisitor<TerminalKind, NonTerminalKind, V::Error, F>
                for BuiltinVisitorAdapter<'a, V>
            {
                fn visit_builtin_whitespace_terminal(
                    &mut self,
                    node: CstNodeId,
                    data: TerminalData,
                    tree: &F,
                ) -> Result<(), V::Error> {
                    self.0.visit_whitespace_terminal(Whitespace(node), data, tree)
                }

                fn visit_builtin_new_line_terminal(
                    &mut self,
                    node: CstNodeId,
                    data: TerminalData,
                    tree: &F,
                ) -> Result<(), V::Error> {
                    self.0.visit_new_line_terminal(NewLine(node), data, tree)
                }

                fn visit_builtin_line_comment_terminal(
                    &mut self,
                    node: CstNodeId,
                    data: TerminalData,
                    tree: &F,
                ) -> Result<(), V::Error> {
                    self.0.visit_line_comment_terminal(LineComment(node), data, tree)
                }

                fn visit_builtin_block_comment_terminal(
                    &mut self,
                    node: CstNodeId,
                    data: TerminalData,
                    tree: &F,
                ) -> Result<(), V::Error> {
                    self.0.visit_block_comment_terminal(BlockComment(node), data, tree)
                }
            }
        }
    }
}
