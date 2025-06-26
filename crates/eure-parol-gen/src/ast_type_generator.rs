use std::{collections::BTreeMap, path::PathBuf};

use parol::generators::export_node_types::{
    ChildrenType, NodeName, NodeTypesInfo, NonTerminalInfo, TerminalInfo,
};
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::parse_quote;

use crate::format_snake_case;

pub struct AstTypeGenerator {
    path: PathBuf,
}

impl AstTypeGenerator {
    pub fn new(path: PathBuf) -> Self {
        println!("path: {path:?}");
        Self { path }
    }

    pub fn generate(&mut self, node_info: &NodeTypesInfo) {
        let imports = self.generate_imports();
        let node_handles = self.generate_node_handles(node_info);
        let terminals = self.generate_terminals(node_info);
        let syn_file = syn::parse_file(
            &quote! {
                #imports
                #node_handles
                #terminals
            }
            .to_string(),
        )
        .unwrap();
        std::fs::write(&self.path, prettyplease::unparse(&syn_file)).unwrap();
    }

    pub fn generate_terminals(&mut self, node_info: &NodeTypesInfo) -> proc_macro2::TokenStream {
        let terminals = node_info
            .terminals
            .iter()
            .map(|t| self.generate_terminal(t));
        quote::quote!(#(#terminals)*)
    }

    pub fn generate_terminal(&mut self, terminal: &TerminalInfo) -> proc_macro2::TokenStream {
        let struct_name = format_ident!("{}", terminal.name);
        let variant_name = format_ident!("{}", terminal.variant);

        quote! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
            pub struct #struct_name(pub(crate) super::tree::CstNodeId);

            impl TerminalHandle for #struct_name {
                fn node_id(&self) -> CstNodeId {
                    self.0
                }
                fn kind(&self) -> TerminalKind {
                    TerminalKind::#variant_name
                }
            }
        }
    }

    pub fn generate_node_handles(&mut self, node_info: &NodeTypesInfo) -> proc_macro2::TokenStream {
        let handles = node_info
            .non_terminals
            .iter()
            .map(|nt| self.generate_node_handle(node_info, nt));
        quote::quote!(#(#handles)*)
    }

    pub fn generate_imports(&self) -> proc_macro2::TokenStream {
        quote! {
            #![allow(unused_variables)]
            use super::tree::{TerminalHandle, NonTerminalHandle, RecursiveView, CstNodeId, ViewConstructionError, CstFacade};
            use super::visitor::BuiltinTerminalVisitor;
            use crate::CstConstructError;
            use super::node_kind::{TerminalKind, NonTerminalKind, NodeKind};
        }
    }

    pub fn generate_node_handle(
        &self,
        info: &NodeTypesInfo,
        nt: &NonTerminalInfo,
    ) -> proc_macro2::TokenStream {
        match nt.kind {
            ChildrenType::Sequence => self
                .generate_non_terminal_sequence(info, nt)
                .to_token_stream(),
            ChildrenType::OneOf => self.generate_one_of_handle(info, nt).to_token_stream(),
            ChildrenType::Recursion => self.generate_recursion_handle(info, nt).to_token_stream(),
            ChildrenType::Option => self.generate_option_handle(info, nt).to_token_stream(),
        }
    }

    pub fn get_terminal_by_name<'a>(
        &self,
        info: &'a NodeTypesInfo,
        name: &str,
    ) -> &'a TerminalInfo {
        info.terminals
            .iter()
            .find(|t| t.name == name)
            .unwrap_or_else(|| panic!("Terminal {name} not found"))
    }

    pub fn get_non_terminal_by_name<'a>(
        &self,
        info: &'a NodeTypesInfo,
        name: &str,
    ) -> &'a NonTerminalInfo {
        info.non_terminals
            .iter()
            .find(|nt| nt.name == name)
            .unwrap_or_else(|| panic!("Non-terminal {name} not found"))
    }

    fn generate_non_terminal_sequence(
        &self,
        info: &NodeTypesInfo,
        nt: &NonTerminalInfo,
    ) -> NonTerminalSequence {
        let handle_name = format_ident!("{}Handle", nt.name);
        let view_name = format_ident!("{}View", nt.name);
        let variant_name = format_ident!("{}", nt.variant);
        let fields = self.fields(info, nt);
        let field_names = fields.iter().map(|f| &f.field_name).collect::<Vec<_>>();
        let field_types = fields.iter().map(|f| &f.field_type).collect::<Vec<_>>();
        let node_kinds = fields.iter().map(|f| &f.node_kind).collect::<Vec<_>>();
        let item_struct = parse_quote! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct #handle_name(pub(crate) super::tree::CstNodeId);
        };
        let new_method = self.generate_handle_new_method(
            quote!(NodeKind::NonTerminal(NonTerminalKind::#variant_name)),
        );
        let kind_method = self.generate_handle_kind_method(quote!(NonTerminalKind::#variant_name));
        let item_impl = parse_quote! {
            impl NonTerminalHandle for #handle_name {
                type View = #view_name;
                fn node_id(&self) -> CstNodeId {
                    self.0
                }
                #new_method
                #kind_method
                fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
                    &self,
                    tree: &F,
                    mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
                    visit_ignored: &'v mut V,
                ) -> Result<O, CstConstructError<E>> {
                    tree.collect_nodes(self.0, [#(#node_kinds),*], |[#(#field_names),*], visit_ignored| Ok(visit(#view_name {
                        #(#field_names: #field_types(#field_names),)*
                    }, visit_ignored)), visit_ignored)
                }
            }
        };
        let view_struct = parse_quote! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub struct #view_name {
                #(pub #field_names: #field_types),*
            }
        };
        let view_impl = parse_quote! {
            impl #view_name {
            }
        };
        NonTerminalSequence {
            handle: item_struct,
            handle_impl: item_impl,
            view: view_struct,
            view_impl,
        }
    }

    fn fields(&self, info: &NodeTypesInfo, nt: &NonTerminalInfo) -> Vec<Field> {
        let mut fields = nt
            .children
            .iter()
            .map(|c| match &c.name {
                NodeName::Terminal(name) => {
                    let field_name = format_snake_case(&name.0);
                    let field_type = format_ident!("{}", name.0);
                    let terminal = self.get_terminal_by_name(info, &name.0);
                    let variant_name = format_ident!("{}", terminal.variant);
                    let node_kind = quote!(NodeKind::Terminal(TerminalKind::#variant_name));
                    Field {
                        field_name,
                        field_type,
                        node_kind,
                    }
                }
                NodeName::NonTerminal(name) => {
                    let field_name = format_snake_case(&name.0);
                    let field_type = format_ident!("{}Handle", name.0);
                    let non_terminal = self.get_non_terminal_by_name(info, &name.0);
                    let variant_name = format_ident!("{}", non_terminal.variant);
                    let node_kind = quote!(NodeKind::NonTerminal(NonTerminalKind::#variant_name));
                    Field {
                        field_name,
                        field_type,
                        node_kind,
                    }
                }
            })
            .collect::<Vec<_>>();
        let mut existing_fields = BTreeMap::new();
        for field in &mut fields {
            let existing_count = existing_fields
                .entry(field.field_name.to_string())
                .or_insert(0);
            if *existing_count > 0 {
                field.field_name =
                    format_ident!("{}{}", field.field_name, (*existing_count + 1).to_string());
            }
            *existing_count += 1;
        }
        fields
    }

    fn generate_one_of_handle(
        &self,
        info: &NodeTypesInfo,
        nt: &NonTerminalInfo,
    ) -> NonTerminalOneOf {
        let handle_name = format_ident!("{}Handle", nt.name);
        let view_name = format_ident!("{}View", nt.name);
        let nt_variant_name = format_ident!("{}", nt.variant);

        struct VariantInfo {
            name: syn::Ident,
            ty: proc_macro2::TokenStream,
            expected_node_kind: proc_macro2::TokenStream,
            view_constructor: proc_macro2::TokenStream,
        }

        let variants = nt.children.iter().map(|child_info| {
            match &child_info.name {
                NodeName::Terminal(name) => {
                    let terminal = self.get_terminal_by_name(info, &name.0);
                    let variant_ident = format_ident!("{}", terminal.name);
                    let type_ident = format_ident!("{}", name.0);
                    let terminal_kind_variant = format_ident!("{}", terminal.variant);

                    VariantInfo {
                        name: variant_ident,
                        ty: quote!(#type_ident),
                        expected_node_kind: quote!(NodeKind::Terminal(TerminalKind::#terminal_kind_variant)),
                        view_constructor: quote!(#type_ident(child)),
                    }
                }
                NodeName::NonTerminal(name) => {
                    let non_terminal = self.get_non_terminal_by_name(info, &name.0);
                    let variant_ident = format_ident!("{}", non_terminal.name);
                    let handle_ident = format_ident!("{}Handle", name.0);
                    let non_terminal_kind_variant = format_ident!("{}", non_terminal.variant);

                    VariantInfo {
                        name: variant_ident,
                        ty: quote!(#handle_ident),
                        expected_node_kind: quote!(NodeKind::NonTerminal(NonTerminalKind::#non_terminal_kind_variant)),
                        view_constructor: quote!(#handle_ident(child)),
                    }
                }
            }
        }).collect::<Vec<_>>();

        let view_enum_variants = variants.iter().map(|v_info| {
            let name = &v_info.name;
            let ty = &v_info.ty;
            quote!(#name(#ty))
        });

        let get_view_match_arms = variants.iter().map(|v_info| {
            let variant_name = &v_info.name;
            let expected_kind = &v_info.expected_node_kind;
            let constructor = &v_info.view_constructor;
            quote! {
                #expected_kind => #view_name::#variant_name(#constructor),
            }
        });

        let item_struct: syn::ItemStruct = parse_quote! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct #handle_name(pub(crate) super::tree::CstNodeId);
        };

        let new_method = self.generate_handle_new_method(
            quote!(NodeKind::NonTerminal(NonTerminalKind::#nt_variant_name)),
        );
        let kind_method =
            self.generate_handle_kind_method(quote!(NonTerminalKind::#nt_variant_name));
        let item_impl: syn::ItemImpl = parse_quote! {
            impl NonTerminalHandle for #handle_name {
                type View = #view_name;
                fn node_id(&self) -> CstNodeId {
                    self.0
                }
                #new_method
                #kind_method
                fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
                    &self,
                    tree: &F,
                    mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
                    visit_ignored: &'v mut V,
                ) -> Result<O, CstConstructError<E>> {
                    let mut children = tree.children(self.0);
                    let Some(child) = children.next() else {
                        return Err(ViewConstructionError::UnexpectedEndOfChildren { parent: self.0 });
                    };
                    let Some(child_data) = tree.node_data(child) else {
                        return Err(ViewConstructionError::NodeIdNotFound { node: child });
                    };

                    let variant = match child_data.node_kind() {
                        #(#get_view_match_arms)*
                        _ => {
                            return Err(ViewConstructionError::UnexpectedNode {
                                node: child,
                                data: child_data,
                                expected_kind: child_data.node_kind(),
                            });
                        }
                    };
                    let (result, _visit) = visit(variant, visit_ignored);
                    if let Some(child) = children.next() {
                        return Err(ViewConstructionError::UnexpectedExtraNode { node: child });
                    }
                    Ok(result)
                }
            }
        };

        let view_enum: syn::ItemEnum = parse_quote! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub enum #view_name {
                #(#view_enum_variants),*
            }
        };

        let view_impl: syn::ItemImpl = parse_quote! {
            impl #view_name {}
        };

        NonTerminalOneOf {
            handle: item_struct,
            handle_impl: item_impl,
            view: view_enum,
            view_impl,
        }
    }

    fn generate_recursion_handle(
        &self,
        info: &NodeTypesInfo,
        nt: &NonTerminalInfo,
    ) -> NonTerminalRecursion {
        let handle_name = format_ident!("{}Handle", nt.name);
        let view_name = format_ident!("{}View", nt.name);
        let mut item_name = format_ident!("{}Item", nt.name);
        let variant_name = format_ident!("{}", nt.variant);

        let fields = self.fields(info, nt);
        let mut field_names = fields.iter().map(|f| &f.field_name).collect::<Vec<_>>();
        let mut field_types = fields.iter().map(|f| &f.field_type).collect::<Vec<_>>();
        let node_kinds = fields.iter().map(|f| &f.node_kind).collect::<Vec<_>>();

        let handle_struct: syn::ItemStruct = parse_quote! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct #handle_name(pub(crate) super::tree::CstNodeId);
        };
        let new_method = self.generate_handle_new_method(
            quote!(NodeKind::NonTerminal(NonTerminalKind::#variant_name)),
        );
        let kind_method = self.generate_handle_kind_method(quote!(NonTerminalKind::#variant_name));
        let item_impl: syn::ItemImpl = parse_quote! {
            impl NonTerminalHandle for #handle_name {
                type View = Option<#view_name>;
                fn node_id(&self) -> CstNodeId {
                    self.0
                }
                #new_method
                #kind_method
                fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
                    &self,
                    tree: &F,
                    mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
                    visit_ignored: &'v mut V,
                ) -> Result<O, CstConstructError<E>> {
                    if tree.has_no_children(self.0) {
                        return Ok(visit(None, visit_ignored).0);
                    }
                    tree.collect_nodes(self.0, [#(#node_kinds),*], |[#(#field_names),*], visit_ignored| Ok(visit(Some(#view_name {
                        #(#field_names: #field_types(#field_names),)*
                    }), visit_ignored)), visit_ignored)
                }
            }
        };

        let view_struct: syn::ItemStruct = parse_quote! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub struct #view_name {
                #(pub #field_names: #field_types),*
            }
        };
        let last_name = field_names.pop().unwrap();
        field_types.pop();
        let push;
        let item_struct = if field_names.len() != 1 {
            push = quote!(items.push(#item_name { #(#field_names),* }));
            Some(parse_quote! {
                #[derive(Debug, Clone, Copy, PartialEq, Eq)]
                pub struct #item_name {
                    #(pub #field_names: #field_types),*
                }
            })
        } else {
            let field_name = field_names.first().unwrap();
            item_name = (*field_types.first().unwrap()).clone();
            push = quote!(items.push(#field_name));
            None
        };
        let view_impl: syn::ItemImpl = parse_quote! {
            impl<F: CstFacade> RecursiveView<F> for #view_name {
                type Item = #item_name;
                fn get_all_with_visit<E>(&self, tree: &F, visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>) -> Result<Vec<Self::Item>, CstConstructError<E>> {
                    let mut items = Vec::new();
                    let mut current_view = Some(*self);
                    while let Some(item) = current_view {
                        let Self { #(#field_names),*, .. } = item;
                        #push;
                        item.#last_name.get_view_with_visit(
                            tree,
                            |view, visit_ignored| {
                                current_view = view;
                                ((), visit_ignored)
                            },
                            visit_ignored,
                        )?;
                    }
                    Ok(items)
                }
            }
        };

        NonTerminalRecursion {
            handle: handle_struct,
            handle_impl: item_impl,
            item_struct,
            view: view_struct,
            view_impl,
        }
    }

    fn generate_option_handle(
        &self,
        info: &NodeTypesInfo,
        nt: &NonTerminalInfo,
    ) -> NonTerminalOption {
        let handle_name = format_ident!("{}Handle", nt.name);
        let variant_name = format_ident!("{}", nt.variant);

        // If there are multiple children, treat it as a sequence
        if nt.children.len() > 1 {
            let view_name = format_ident!("{}View", nt.name);
            let fields = self.fields(info, nt);
            let field_names = fields.iter().map(|f| &f.field_name).collect::<Vec<_>>();
            let field_types = fields.iter().map(|f| &f.field_type).collect::<Vec<_>>();
            let node_kinds = fields.iter().map(|f| &f.node_kind).collect::<Vec<_>>();

            let item_struct: syn::ItemStruct = parse_quote! {
                #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
                pub struct #handle_name(pub(crate) super::tree::CstNodeId);
            };

            let view_struct: syn::ItemStruct = parse_quote! {
                #[derive(Debug, Clone, Copy, PartialEq, Eq)]
                pub struct #view_name {
                    #(pub #field_names: #field_types),*
                }
            };

            let new_method = self.generate_handle_new_method(
                quote!(NodeKind::NonTerminal(NonTerminalKind::#variant_name)),
            );
            let kind_method =
                self.generate_handle_kind_method(quote!(NonTerminalKind::#variant_name));

            let item_impl: syn::ItemImpl = parse_quote! {
                impl NonTerminalHandle for #handle_name {
                    type View = Option<#view_name>;
                    fn node_id(&self) -> CstNodeId {
                        self.0
                    }
                    #new_method
                    #kind_method
                    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
                        &self,
                        tree: &F,
                        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
                        visit_ignored: &'v mut V,
                    ) -> Result<O, CstConstructError<E>> {
                        if tree.has_no_children(self.0) {
                            return Ok(visit(None, visit_ignored).0);
                        }
                        tree.collect_nodes(self.0, [#(#node_kinds),*], |[#(#field_names),*], visit_ignored| Ok(visit(Some(#view_name {
                            #(#field_names: #field_types(#field_names),)*
                        }), visit_ignored)), visit_ignored)
                    }
                }
            };

            return NonTerminalOption {
                handle: item_struct,
                handle_impl: item_impl,
                view_struct: Some(view_struct),
            };
        }

        // Handle single child case (existing logic)
        if nt.children.len() != 1 {
            panic!(
                "Option non-terminal {} should have exactly one child, found {}",
                nt.name,
                nt.children.len()
            );
        }
        let child_info = &nt.children[0];

        let child_handle_name = match &child_info.name {
            NodeName::Terminal(name) => {
                let terminal = self.get_terminal_by_name(info, &name.0);
                let name = format_ident!("{}", terminal.name);
                quote!(#name)
            }
            NodeName::NonTerminal(name) => {
                let non_terminal = self.get_non_terminal_by_name(info, &name.0);
                let handle_name = format_ident!("{}Handle", non_terminal.name);
                quote!(#handle_name)
            }
        };

        let item_struct: syn::ItemStruct = parse_quote! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct #handle_name(pub(crate) super::tree::CstNodeId);
        };
        let new_method = self.generate_handle_new_method(
            quote!(NodeKind::NonTerminal(NonTerminalKind::#variant_name)),
        );
        let kind_method = self.generate_handle_kind_method(quote!(NonTerminalKind::#variant_name));

        let item_impl: syn::ItemImpl = parse_quote! {
            impl NonTerminalHandle for #handle_name {
                type View = Option<#child_handle_name>;
                fn node_id(&self) -> CstNodeId {
                    self.0
                }
                #new_method
                #kind_method
                fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
                    &self,
                    tree: &F,
                    mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
                    visit_ignored: &'v mut V,
                ) -> Result<O, CstConstructError<E>> {
                    if tree.has_no_children(self.0) {
                        return Ok(visit(None, visit_ignored).0);
                    }
                    Ok(visit(
                        Some(#child_handle_name::new_with_visit(
                            self.0,
                            tree,
                            visit_ignored,
                        )?),
                        visit_ignored,
                    )
                    .0)
                }
            }
        };

        NonTerminalOption {
            handle: item_struct,
            handle_impl: item_impl,
            view_struct: None,
        }
    }

    fn generate_handle_new_method(&self, node_kind: TokenStream) -> proc_macro2::TokenStream {
        parse_quote! {
            fn new_with_visit<F: CstFacade, E>(index: CstNodeId, tree: &F, visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>) -> Result<Self, CstConstructError<E>> {
                tree.collect_nodes(index, [#node_kind], |[index], visit| Ok((Self(index), visit)), visit_ignored)
            }
        }
    }

    fn generate_handle_kind_method(&self, node_kind: TokenStream) -> proc_macro2::TokenStream {
        parse_quote! {
            fn kind(&self) -> NonTerminalKind {
                #node_kind
            }
        }
    }
}

struct Field {
    field_name: syn::Ident,
    field_type: syn::Ident,
    node_kind: TokenStream,
}

struct NonTerminalSequence {
    handle: syn::ItemStruct,
    handle_impl: syn::ItemImpl,
    view: syn::ItemStruct,
    view_impl: syn::ItemImpl,
}

impl ToTokens for NonTerminalSequence {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.handle.to_tokens(tokens);
        self.handle_impl.to_tokens(tokens);
        self.view.to_tokens(tokens);
        self.view_impl.to_tokens(tokens);
    }
}

struct NonTerminalOneOf {
    handle: syn::ItemStruct,
    handle_impl: syn::ItemImpl,
    view: syn::ItemEnum,
    view_impl: syn::ItemImpl,
}

impl ToTokens for NonTerminalOneOf {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.handle.to_tokens(tokens);
        self.handle_impl.to_tokens(tokens);
        self.view.to_tokens(tokens);
        self.view_impl.to_tokens(tokens);
    }
}

struct NonTerminalRecursion {
    handle: syn::ItemStruct,
    handle_impl: syn::ItemImpl,
    view: syn::ItemStruct,
    view_impl: syn::ItemImpl,
    /// None if the item is a single child
    item_struct: Option<syn::ItemStruct>,
}

impl ToTokens for NonTerminalRecursion {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.handle.to_tokens(tokens);
        self.handle_impl.to_tokens(tokens);
        self.view.to_tokens(tokens);
        self.view_impl.to_tokens(tokens);
        self.item_struct.to_tokens(tokens);
    }
}

struct NonTerminalOption {
    handle: syn::ItemStruct,
    handle_impl: syn::ItemImpl,
    view_struct: Option<syn::ItemStruct>,
}

impl ToTokens for NonTerminalOption {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.handle.to_tokens(tokens);
        self.handle_impl.to_tokens(tokens);
        self.view_struct.to_tokens(tokens);
    }
}
