use std::{collections::BTreeMap, path::PathBuf};

use parol::generators::export_node_types::{
    Child, NodeName, NodeTypesInfo, NonTerminalInfo, NonTerminalStructure, TerminalInfo,
};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::parse_quote;

use crate::format_snake_case;

pub struct AstTypeGenerator {
    path: PathBuf,
    #[allow(dead_code)]
    config: crate::WalkerConfig,
}

impl AstTypeGenerator {
    pub fn new(path: PathBuf, config: crate::WalkerConfig) -> Self {
        println!("path: {path:?}");
        Self { path, config }
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
            pub struct #struct_name(pub(crate) CstNodeId);

            impl TerminalHandle<TerminalKind> for #struct_name {
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
        let header = crate::generate_header_comment();
        let node_kind_module = &self.config.imports.node_kind_module;

        // Parse module paths to create proper use statements
        let runtime_use = syn::parse_str::<syn::Path>(&self.config.imports.runtime_crate).unwrap();
        let node_kind_use = syn::parse_str::<syn::Path>(node_kind_module).unwrap();

        // BuiltinTerminalVisitor is generated in visitor module, not in runtime
        // Derive visitor module path from nodes_module (they're siblings)
        quote! {
            #header
            #[allow(unused_imports)]
            use #runtime_use::{
                TerminalHandle, NonTerminalHandle, RecursiveView, CstNodeId,
                CstFacade, CstConstructError, ViewConstructionError,
                NodeKind, BuiltinTerminalVisitor, BuiltinTerminalKind,
            };
            use #node_kind_use::{TerminalKind, NonTerminalKind};
        }
    }

    pub fn generate_node_handle(
        &self,
        info: &NodeTypesInfo,
        nt: &NonTerminalInfo,
    ) -> proc_macro2::TokenStream {
        match &nt.structure {
            NonTerminalStructure::Sequence(_) => self
                .generate_non_terminal_sequence(info, nt)
                .to_token_stream(),
            NonTerminalStructure::OneOf(_) => {
                self.generate_one_of_handle(info, nt).to_token_stream()
            }
            NonTerminalStructure::Recursion(_) => {
                self.generate_recursion_handle(info, nt).to_token_stream()
            }
            NonTerminalStructure::Option(_) => {
                self.generate_option_handle(info, nt).to_token_stream()
            }
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
        let children = match &nt.structure {
            NonTerminalStructure::Sequence(children) => children,
            _ => panic!("Expected Sequence structure"),
        };
        let fields = self.fields(info, children);
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
            impl NonTerminalHandle<TerminalKind, NonTerminalKind> for #handle_name {
                type View = #view_name;
                fn node_id(&self) -> CstNodeId {
                    self.0
                }
                #new_method
                #kind_method
                fn get_view_with_visit<'v, F: CstFacade<TerminalKind, NonTerminalKind>, V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>, O, E>(
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

    fn field(&self, info: &NodeTypesInfo, child: &Child) -> Field {
        match &child.name {
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
        }
    }

    fn fields(&self, info: &NodeTypesInfo, children: &[Child]) -> Vec<Field> {
        let mut fields: Vec<_> = children.iter().map(|c| self.field(info, c)).collect();
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

    fn build_alt_info(
        &self,
        info: &NodeTypesInfo,
        nt_name: &str,
        idx: usize,
        alt_children: &[Child],
    ) -> AltInfo {
        if alt_children.len() == 1 {
            // Single element alternative - use the element name as variant name
            let f = self.field(info, &alt_children[0]);
            let variant_ty = {
                let ty = &f.field_type;
                quote!(#ty)
            };
            // Derive variant_name from field_type (strip "Handle" suffix for non-terminals)
            let field_type_str = f.field_type.to_string();
            let variant_name = if field_type_str.ends_with("Handle") {
                format_ident!("{}", &field_type_str[..field_type_str.len() - 6])
            } else {
                f.field_type.clone()
            };
            AltInfo {
                variant_name,
                variant_ty,
                first_child_kind: f.node_kind,
                fields: vec![],
                node_kinds: vec![],
            }
        } else {
            // Multi-element alternative - create Alt{idx} variant with struct
            let variant_name = format_ident!("Alt{}", idx);
            let struct_name = format_ident!("{}Alt{}", nt_name, idx);

            let fields = self.fields(info, alt_children);
            let node_kinds: Vec<_> = fields.iter().map(|f| f.node_kind.clone()).collect();
            let first_kind = node_kinds
                .first()
                .cloned()
                .unwrap_or(quote!(NodeKind::Root));

            AltInfo {
                variant_name,
                variant_ty: quote!(#struct_name),
                first_child_kind: first_kind,
                fields,
                node_kinds,
            }
        }
    }

    fn generate_alt_structs(&self, alt_infos: &[AltInfo]) -> Vec<syn::ItemStruct> {
        alt_infos
            .iter()
            .filter(|a| a.is_multi_element())
            .map(|a| {
                let struct_name = &a.variant_ty;
                let field_names: Vec<_> = a.fields.iter().map(|f| &f.field_name).collect();
                let field_types: Vec<_> = a.fields.iter().map(|f| &f.field_type).collect();
                parse_quote! {
                    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
                    pub struct #struct_name {
                        #(pub #field_names: #field_types),*
                    }
                }
            })
            .collect()
    }

    fn generate_one_of_match_arms(
        &self,
        alt_infos: &[AltInfo],
        view_name: &syn::Ident,
        all_multi_element: bool,
    ) -> Vec<proc_macro2::TokenStream> {
        alt_infos
            .iter()
            .map(|a| {
                let variant_name = &a.variant_name;
                let expected_kind = &a.first_child_kind;

                if a.is_multi_element() {
                    // Multi-element: use collect_nodes
                    let struct_ty = &a.variant_ty;
                    let field_names: Vec<_> = a.fields.iter().map(|f| &f.field_name).collect();
                    let field_types: Vec<_> = a.fields.iter().map(|f| &f.field_type).collect();
                    let node_kinds: Vec<_> = a.node_kinds.iter().collect();

                    let collect_nodes_call = quote! {
                        tree.collect_nodes(
                            self.0,
                            [#(#node_kinds),*],
                            |[#(#field_names),*], visit_ignored| {
                                Ok(visit(#view_name::#variant_name(#struct_ty {
                                    #(#field_names: #field_types(#field_names)),*
                                }), visit_ignored))
                            },
                            visit_ignored,
                        )
                    };

                    if all_multi_element {
                        quote! { #expected_kind => { #collect_nodes_call } }
                    } else {
                        quote! { #expected_kind => { return #collect_nodes_call; } }
                    }
                } else {
                    // Single element: simple match
                    let ty = &a.variant_ty;
                    quote! {
                        #expected_kind => #view_name::#variant_name(#ty(child)),
                    }
                }
            })
            .collect()
    }

    fn generate_one_of_handle(
        &self,
        info: &NodeTypesInfo,
        nt: &NonTerminalInfo,
    ) -> NonTerminalOneOf {
        let handle_name = format_ident!("{}Handle", nt.name);
        let view_name = format_ident!("{}View", nt.name);
        let nt_variant_name = format_ident!("{}", nt.variant);

        // Get all alternatives from the structure
        let NonTerminalStructure::OneOf(alternatives) = &nt.structure else {
            panic!("Expected OneOf structure")
        };

        let alt_infos: Vec<AltInfo> = alternatives
            .iter()
            .enumerate()
            .map(|(idx, alt_children)| self.build_alt_info(info, &nt.name, idx, alt_children))
            .collect();

        let alt_structs = self.generate_alt_structs(&alt_infos);

        // Generate view enum variants
        let view_enum_variants = alt_infos.iter().map(|a| {
            let name = &a.variant_name;
            let ty = &a.variant_ty;
            quote!(#name(#ty))
        });

        // Check if all alternatives are multi-element
        let all_multi_element = alt_infos.iter().all(|a| a.is_multi_element());

        let get_view_match_arms =
            self.generate_one_of_match_arms(&alt_infos, &view_name, all_multi_element);

        let item_struct: syn::ItemStruct = parse_quote! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct #handle_name(pub(crate) super::tree::CstNodeId);
        };

        let new_method = self.generate_handle_new_method(
            quote!(NodeKind::NonTerminal(NonTerminalKind::#nt_variant_name)),
        );
        let kind_method =
            self.generate_handle_kind_method(quote!(NonTerminalKind::#nt_variant_name));

        // Common code snippets for get_view_with_visit body
        let children_preamble = quote! {
            let mut children = tree.children(self.0);
            let Some(child) = children.next() else {
                return Err(ViewConstructionError::UnexpectedEndOfChildren { parent: self.0 }.into());
            };
            let Some(child_data) = tree.node_data(child) else {
                return Err(ViewConstructionError::NodeIdNotFound { node: child }.into());
            };
        };
        let unexpected_node_error = quote! {
            Err(ViewConstructionError::UnexpectedNode { node: child }.into())
        };

        // Generate get_view_with_visit body
        let get_view_body = if all_multi_element {
            // All multi-element alternatives: every match arm returns via collect_nodes
            quote! {
                #children_preamble
                match child_data.node_kind() {
                    #(#get_view_match_arms)*
                    _ => #unexpected_node_error
                }
            }
        } else {
            // Mixed or all single-element: multi-element arms early return,
            // single-element arms assign to variant
            quote! {
                #children_preamble
                let variant = match child_data.node_kind() {
                    #(#get_view_match_arms)*
                    _ => { return #unexpected_node_error; }
                };
                let (result, _visit) = visit(variant, visit_ignored);
                if let Some(extra_child) = children.next() {
                    return Err(ViewConstructionError::UnexpectedExtraNode { node: extra_child }.into());
                }
                Ok(result)
            }
        };

        let item_impl: syn::ItemImpl = parse_quote! {
            impl NonTerminalHandle<TerminalKind, NonTerminalKind> for #handle_name {
                type View = #view_name;
                fn node_id(&self) -> CstNodeId {
                    self.0
                }
                #new_method
                #kind_method
                fn get_view_with_visit<'v, F: CstFacade<TerminalKind, NonTerminalKind>, V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>, O, E>(
                    &self,
                    tree: &F,
                    mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
                    visit_ignored: &'v mut V,
                ) -> Result<O, CstConstructError<E>> {
                    #get_view_body
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
            alt_structs,
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

        let children = match &nt.structure {
            NonTerminalStructure::Recursion(children) => children,
            _ => panic!("Expected Recursion structure"),
        };
        let fields = self.fields(info, children);
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
            impl NonTerminalHandle<TerminalKind, NonTerminalKind> for #handle_name {
                type View = Option<#view_name>;
                fn node_id(&self) -> CstNodeId {
                    self.0
                }
                #new_method
                #kind_method
                fn get_view_with_visit<'v, F: CstFacade<TerminalKind, NonTerminalKind>, V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>, O, E>(
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
            impl<F: CstFacade<TerminalKind, NonTerminalKind>> RecursiveView<TerminalKind, NonTerminalKind, F> for #view_name {
                type Item = #item_name;
                fn get_all_with_visit<E>(&self, tree: &F, visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>) -> Result<Vec<Self::Item>, CstConstructError<E>> {
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
        let children = match &nt.structure {
            NonTerminalStructure::Option(children) => children,
            _ => panic!("Expected Option structure"),
        };

        // If there are multiple children, treat it as a sequence
        if children.len() > 1 {
            let view_name = format_ident!("{}View", nt.name);
            let fields = self.fields(info, children);
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
                impl NonTerminalHandle<TerminalKind, NonTerminalKind> for #handle_name {
                    type View = Option<#view_name>;
                    fn node_id(&self) -> CstNodeId {
                        self.0
                    }
                    #new_method
                    #kind_method
                    fn get_view_with_visit<'v, F: CstFacade<TerminalKind, NonTerminalKind>, V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>, O, E>(
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
        if children.len() != 1 {
            panic!(
                "Option non-terminal {} should have exactly one child, found {}",
                nt.name,
                children.len()
            );
        }
        let child_info = &children[0];

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
            impl NonTerminalHandle<TerminalKind, NonTerminalKind> for #handle_name {
                type View = Option<#child_handle_name>;
                fn node_id(&self) -> CstNodeId {
                    self.0
                }
                #new_method
                #kind_method
                fn get_view_with_visit<'v, F: CstFacade<TerminalKind, NonTerminalKind>, V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>, O, E>(
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
            fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(index: CstNodeId, tree: &F, visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>) -> Result<Self, CstConstructError<E>> {
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

struct AltInfo {
    variant_name: syn::Ident,
    variant_ty: proc_macro2::TokenStream,
    first_child_kind: proc_macro2::TokenStream,
    fields: Vec<Field>,
    node_kinds: Vec<proc_macro2::TokenStream>,
}

impl AltInfo {
    fn is_multi_element(&self) -> bool {
        !self.fields.is_empty()
    }
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
    /// Additional structs for multi-element alternatives
    alt_structs: Vec<syn::ItemStruct>,
}

impl ToTokens for NonTerminalOneOf {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.handle.to_tokens(tokens);
        self.handle_impl.to_tokens(tokens);
        self.view.to_tokens(tokens);
        self.view_impl.to_tokens(tokens);
        for alt_struct in &self.alt_structs {
            alt_struct.to_tokens(tokens);
        }
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
