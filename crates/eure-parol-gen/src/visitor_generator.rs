use std::collections::BTreeMap;
use std::path::PathBuf;

use parol::generators::export_node_types::{
    ChildrenType, NodeName, NodeTypesInfo, NonTerminalInfo, TerminalInfo,
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
        .unwrap_or_else(|| panic!("Terminal {} not found", name))
}

pub struct VisitorGenerator {
    path: PathBuf,
}

impl VisitorGenerator {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
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
        quote! {
            use crate::{
                Cst, CstConstructError, NodeKind, CstNode,
                nodes::*,
                node_kind::{TerminalKind, NonTerminalKind},
                tree::{TerminalHandle as _, NonTerminalHandle as _, TerminalData, NonTerminalData, CstNodeId, CstFacade},
            };
        }
    }

    fn get_view_param_type(&self, nt_info: &NonTerminalInfo) -> TokenStream {
        let view_name_ident = format_ident!("{}View", nt_info.name);
        match nt_info.kind {
            ChildrenType::Sequence | ChildrenType::OneOf => {
                quote!(#view_name_ident)
            }
            ChildrenType::Recursion => {
                quote!(#view_name_ident)
            }
            ChildrenType::Option => {
                let child_node_name = &nt_info.children[0].name;
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
            pub trait CstVisitor<F: CstFacade>: CstVisitorSuper<F, Self::Error> {
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
                fn then_construct_error(&mut self, node_data: Option<CstNode>, parent: CstNodeId, kind: NodeKind, error: CstConstructError, tree: &F) -> Result<(), Self::Error> {
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
                pub trait Sealed<F> {}
            }
            pub trait CstVisitorSuper<F: CstFacade, E>: private::Sealed<F> {
                #(#methods)*
                #(#terminal_methods)*
                fn visit_non_terminal_super(&mut self, id: CstNodeId, kind: NonTerminalKind, data: NonTerminalData, tree: &F) -> Result<(), E>;
                fn visit_non_terminal_close_super(&mut self, id: CstNodeId, kind: NonTerminalKind, data: NonTerminalData, tree: &F) -> Result<(), E>;
                fn visit_terminal_super(&mut self, id: CstNodeId, kind: TerminalKind, data: TerminalData, tree: &F) -> Result<(), E>;
                fn visit_any(&mut self, id: CstNodeId, node: CstNode, tree: &F) -> Result<(), E>;
                /// Recover from a construct error. This eagerly visits the children of the node.
                fn recover_error(&mut self, node_data: Option<CstNode>, id: CstNodeId, kind: NodeKind, tree: &F) -> Result<(), E>;
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
            impl<V: CstVisitor<F>, F: CstFacade> private::Sealed<F> for V {}
            impl<V: CstVisitor<F>, F: CstFacade> CstVisitorSuper<F, V::Error> for V {
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
                fn recover_error(&mut self, node_data: Option<CstNode>, id: CstNodeId, kind: NodeKind, tree: &F) -> Result<(), V::Error> {
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

        let on_view = match nt_info.kind {
            ChildrenType::Sequence | ChildrenType::OneOf => {
                quote! {
                    visit.#visitor_method_name(handle, view, tree)
                }
            }
            ChildrenType::Option | ChildrenType::Recursion => {
                quote! {
                    if let Some(view) = view {
                        visit.#visitor_method_name(handle, view, tree)
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
                            error,
                            tree,
                        );
                    }
                };
                self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
                let result = match handle.get_view_with_visit(tree, |view, visit: &mut Self| (#on_view, visit), self).map_err(|e| e.extract_error()) {
                    Ok(Ok(())) => Ok(()),
                    Ok(Err(e)) => Err(e),
                    Err(Ok(e)) => Err(e),
                    Err(Err(e)) => self.then_construct_error(Some(CstNode::new_non_terminal(handle.kind(), nt_data)), handle.node_id(), NodeKind::NonTerminal(handle.kind()), e, tree),
                };
                self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
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

    fn get_fields_for_view(&self, nt_info: &NonTerminalInfo) -> Vec<GenField> {
        let mut gen_fields = nt_info
            .children
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

        let body = match nt_info.kind {
            ChildrenType::Sequence | ChildrenType::Recursion => {
                let view_fields = self.get_fields_for_view(nt_info);
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
                                    Err(error) => return self.then_construct_error(None, #child_handle_field_name.0, NodeKind::Terminal(#child_handle_field_name.kind()), error, tree),
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
            ChildrenType::OneOf => {
                let variants_handling = nt_info.children.iter().map(|child_prod_info| {
                    let (child_name_str, is_child_nt) = match &child_prod_info.name {
                        NodeName::Terminal(name) => (name.0.as_str(), false),
                        NodeName::NonTerminal(name) => (name.0.as_str(), true),
                    };
                    let variant_name_ident = format_ident!("{}", child_name_str);

                    if is_child_nt {
                        let visit_child_handle_method =
                            crate::format_snake_case(&format!("visit_{}_handle", child_name_str));
                        quote! {
                            #actual_view_type_name::#variant_name_ident(item) => {
                                self.#visit_child_handle_method(item, tree)?;
                            }
                        }
                    } else {
                        let visit_terminal_method =
                            crate::format_snake_case(&format!("visit_{}_terminal", child_name_str));
                        quote! {
                            #actual_view_type_name::#variant_name_ident(item) => {
                                self.#visit_terminal_method(item, tree)?;
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
            ChildrenType::Option => {
                let child_info = &nt_info.children[0];
                let visit_call = match &child_info.name {
                    NodeName::NonTerminal(name) => {
                        // name is NonTerminalName(String)
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
                            self.#visit_terminal_method(#view_ident, tree)?;
                        }
                    }
                };
                quote! {
                    #visit_call
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
        quote! {
            pub trait BuiltinTerminalVisitor<E, F: CstFacade> {
                fn visit_builtin_new_line_terminal(
                    &mut self,
                    terminal: NewLine,
                    data: TerminalData,
                    tree: &F,
                ) -> Result<(), E>;
                fn visit_builtin_whitespace_terminal(
                    &mut self,
                    terminal: Whitespace,
                    data: TerminalData,
                    tree: &F,
                ) -> Result<(), E>;
                fn visit_builtin_line_comment_terminal(
                    &mut self,
                    terminal: LineComment,
                    data: TerminalData,
                    tree: &F,
                ) -> Result<(), E>;
                fn visit_builtin_block_comment_terminal(
                    &mut self,
                    terminal: BlockComment,
                    data: TerminalData,
                    tree: &F,
                ) -> Result<(), E>;
            }

            impl<V: CstVisitor<F>, F: CstFacade> BuiltinTerminalVisitor<V::Error, F> for V {
                fn visit_builtin_new_line_terminal(
                    &mut self,
                    terminal: NewLine,
                    data: TerminalData,
                    tree: &F,
                ) -> Result<(), V::Error> {
                    self.visit_new_line_terminal(terminal, data, tree)
                }

                fn visit_builtin_whitespace_terminal(
                    &mut self,
                    terminal: Whitespace,
                    data: TerminalData,
                    tree: &F,
                ) -> Result<(), V::Error> {
                    self.visit_whitespace_terminal(terminal, data, tree)
                }

                fn visit_builtin_line_comment_terminal(
                    &mut self,
                    terminal: LineComment,
                    data: TerminalData,
                    tree: &F,
                ) -> Result<(), V::Error> {
                    self.visit_line_comment_terminal(terminal, data, tree)
                }

                fn visit_builtin_block_comment_terminal(
                    &mut self,
                    terminal: BlockComment,
                    data: TerminalData,
                    tree: &F,
                ) -> Result<(), V::Error> {
                    self.visit_block_comment_terminal(terminal, data, tree)
                }
            }
        }
    }
}
