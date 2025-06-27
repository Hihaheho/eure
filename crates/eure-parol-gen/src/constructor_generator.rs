use std::path::PathBuf;

use parol::generators::export_node_types::{
    ChildrenType, NodeName, NodeTypesInfo, NonTerminalInfo, TerminalInfo,
};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::format_snake_case;

pub struct ConstructorGenerator {
    output_path: PathBuf,
}

impl ConstructorGenerator {
    pub fn new(output_path: PathBuf) -> Self {
        Self { output_path }
    }

    pub fn generate(&self, node_info: &NodeTypesInfo) {
        let imports = self.generate_imports();
        let branded_types = self.generate_branded_types(node_info);
        let constructors = self.generate_constructors(node_info);

        let content = quote! {
            #imports

            #branded_types

            #constructors
        };

        let content_str = content.to_string();
        let syn_file = match syn::parse_file(&content_str) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("Parse error: {e}");
                eprintln!("Generated code:\n{content_str}");
                panic!("Failed to parse generated code");
            }
        };
        std::fs::write(&self.output_path, prettyplease::unparse(&syn_file)).unwrap();
    }

    fn generate_imports(&self) -> TokenStream {
        quote! {
            use crate::builder::{CstBuilder, BuilderNodeId};
            use crate::node_kind::{NonTerminalKind, TerminalKind};
        }
    }

    fn generate_constructors(&self, node_info: &NodeTypesInfo) -> TokenStream {
        let non_terminal_constructors: Vec<_> = node_info
            .non_terminals
            .iter()
            .map(|nt| self.generate_non_terminal_constructor(node_info, nt))
            .collect();

        let terminal_constructors = self.generate_terminal_constructors(node_info);

        quote! {
            #(#non_terminal_constructors)*
            #terminal_constructors
        }
    }

    fn generate_non_terminal_constructor(
        &self,
        node_info: &NodeTypesInfo,
        nt: &NonTerminalInfo,
    ) -> TokenStream {
        // Generate constructor based on the ChildrenType, following grammar structure exactly
        match nt.kind {
            ChildrenType::Sequence => self.generate_sequence_constructor(node_info, nt),
            ChildrenType::OneOf => self.generate_one_of_constructor(node_info, nt),
            ChildrenType::Recursion => self.generate_recursion_constructor(node_info, nt),
            ChildrenType::Option => self.generate_option_constructor(node_info, nt),
        }
    }

    fn generate_sequence_constructor(
        &self,
        node_info: &NodeTypesInfo,
        nt: &NonTerminalInfo,
    ) -> TokenStream {
        let name = format_ident!("{}Constructor", nt.name);
        let node_type = format_ident!("{}Node", nt.name);
        let variant_name = format_ident!("{}", nt.variant);

        // Generate fields for each child
        let fields: Vec<_> = nt
            .children
            .iter()
            .enumerate()
            .map(|(idx, child)| {
                let field_name = self.derive_field_name(node_info, nt, idx);
                let field_ident = format_ident!("{}", field_name);

                let field_type = match &child.name {
                    NodeName::Terminal(term) => {
                        let term_info = self.get_terminal_by_name(node_info, &term.0);
                        format_ident!("{}Token", term_info.name)
                    }
                    NodeName::NonTerminal(nt_name) => {
                        let nt_info = self.get_non_terminal_by_name(node_info, &nt_name.0);
                        format_ident!("{}Node", nt_info.name)
                    }
                };

                (field_ident, field_type)
            })
            .collect();

        let field_defs = fields.iter().map(|(name, ty)| quote!(#name: #ty));
        let embed_children = fields
            .iter()
            .map(|(name, _)| quote!(let #name = builder.embed(self.#name.builder);));
        let child_ids = fields.iter().map(|(name, _)| quote!(#name));

        quote! {
            #[derive(bon::Builder)]
            pub struct #name {
                #(#field_defs),*
            }

            impl #name {
                pub fn build(self) -> #node_type {
                    let mut builder = CstBuilder::new();

                    // Embed children's builders
                    #(#embed_children)*

                    // Create the non-terminal node
                    let node_id = builder.non_terminal(
                        NonTerminalKind::#variant_name,
                        vec![#(#child_ids),*]
                    );

                    #node_type { node_id, builder }
                }
            }
        }
    }

    fn generate_one_of_constructor(
        &self,
        node_info: &NodeTypesInfo,
        nt: &NonTerminalInfo,
    ) -> TokenStream {
        let enum_name = format_ident!("{}Constructor", nt.name);
        let node_type = format_ident!("{}Node", nt.name);
        let variant_name = format_ident!("{}", nt.variant);

        let variants: Vec<_> = nt
            .children
            .iter()
            .map(|child| match &child.name {
                NodeName::Terminal(term) => {
                    let term_info = self.get_terminal_by_name(node_info, &term.0);
                    let term_type = format_ident!("{}Token", term_info.name);
                    let variant = format_ident!("{}", term_info.variant);
                    (variant, term_type)
                }
                NodeName::NonTerminal(nt_name) => {
                    let nt_info = self.get_non_terminal_by_name(node_info, &nt_name.0);
                    let nt_node = format_ident!("{}Node", nt_info.name);
                    let variant = format_ident!("{}", nt_info.variant);
                    (variant, nt_node)
                }
            })
            .collect();

        let variant_defs = variants.iter().map(|(variant, ty)| quote!(#variant(#ty)));
        let variant_matches = variants
            .iter()
            .map(|(variant, _)| quote!(Self::#variant(node) => builder.embed(node.builder)));

        quote! {
            pub enum #enum_name {
                #(#variant_defs),*
            }

            impl #enum_name {
                pub fn build(self) -> #node_type {
                    let mut builder = CstBuilder::new();

                    let child_id = match self {
                        #(#variant_matches),*
                    };

                    let node_id = builder.non_terminal(
                        NonTerminalKind::#variant_name,
                        vec![child_id]
                    );

                    #node_type { node_id, builder }
                }
            }
        }
    }

    fn generate_recursion_constructor(
        &self,
        node_info: &NodeTypesInfo,
        nt: &NonTerminalInfo,
    ) -> TokenStream {
        let name = format_ident!("{}Constructor", nt.name);
        let node_type = format_ident!("{}Node", nt.name);
        let variant_name = format_ident!("{}", nt.variant);

        // Recursion nodes have two cases:
        // 1. Empty (base case) - no children
        // 2. Push (recursive case) - with specific children including recursive reference

        // For the Push case, we need fields for all children
        let fields: Vec<_> = nt
            .children
            .iter()
            .enumerate()
            .map(|(idx, child)| {
                let field_name = self.derive_field_name(node_info, nt, idx);
                let field_ident = format_ident!("{}", field_name);

                let field_type = match &child.name {
                    NodeName::Terminal(term) => {
                        let term_info = self.get_terminal_by_name(node_info, &term.0);
                        format_ident!("{}Token", term_info.name)
                    }
                    NodeName::NonTerminal(nt_name) => {
                        let nt_info = self.get_non_terminal_by_name(node_info, &nt_name.0);
                        format_ident!("{}Node", nt_info.name)
                    }
                };

                (field_ident, field_type)
            })
            .collect();

        // Generate the builder struct for Push variant
        let field_defs = fields.iter().map(|(name, ty)| quote!(#name: #ty));
        let embed_children = fields
            .iter()
            .map(|(name, _)| quote!(let #name = builder.embed(self.#name.builder);));
        let child_ids = fields.iter().map(|(name, _)| quote!(#name));

        quote! {
            #[derive(bon::Builder)]
            pub struct #name {
                #(#field_defs),*
            }

            impl #name {
                /// Create an empty node (base case for recursion)
                pub fn empty() -> #node_type {
                    let mut builder = CstBuilder::new();
                    let node_id = builder.non_terminal(
                        NonTerminalKind::#variant_name,
                        Vec::<BuilderNodeId>::new()
                    );
                    #node_type { node_id, builder }
                }

                /// Create a node with children (recursive case)
                pub fn build(self) -> #node_type {
                    let mut builder = CstBuilder::new();

                    // Embed children's builders
                    #(#embed_children)*

                    let node_id = builder.non_terminal(
                        NonTerminalKind::#variant_name,
                        vec![#(#child_ids),*]
                    );

                    #node_type { node_id, builder }
                }
            }
        }
    }

    fn generate_option_constructor(
        &self,
        node_info: &NodeTypesInfo,
        nt: &NonTerminalInfo,
    ) -> TokenStream {
        let name = format_ident!("{}Constructor", nt.name);
        let node_type = format_ident!("{}Node", nt.name);
        let variant_name = format_ident!("{}", nt.variant);

        if nt.children.is_empty() {
            // Empty option - just None
            quote! {
                #[derive(bon::Builder)]
                pub struct #name {}

                impl #name {
                    pub fn build(self) -> #node_type {
                        let mut builder = CstBuilder::new();
                        let node_id = builder.non_terminal(
                            NonTerminalKind::#variant_name,
                            Vec::<BuilderNodeId>::new()
                        );
                        #node_type { node_id, builder }
                    }
                }
            }
        } else if nt.children.len() == 1 {
            // Single child option - Some(child) or None
            let child = &nt.children[0];
            let (child_type, field_name) = match &child.name {
                NodeName::Terminal(term) => {
                    let term_info = self.get_terminal_by_name(node_info, &term.0);
                    let term_type = format_ident!("{}Token", term_info.name);
                    let field = format_snake_case(&term.0);
                    (term_type, field)
                }
                NodeName::NonTerminal(nt_name) => {
                    let nt_info = self.get_non_terminal_by_name(node_info, &nt_name.0);
                    let nt_node = format_ident!("{}Node", nt_info.name);
                    let field = format_snake_case(&nt_name.0);
                    (nt_node, field)
                }
            };

            quote! {
                #[derive(bon::Builder)]
                pub struct #name {
                    #field_name: Option<#child_type>,
                }

                impl #name {
                    pub fn build(self) -> #node_type {
                        let mut builder = CstBuilder::new();

                        let children = if let Some(child) = self.#field_name {
                            vec![builder.embed(child.builder)]
                        } else {
                            Vec::<BuilderNodeId>::new()
                        };

                        let node_id = builder.non_terminal(
                            NonTerminalKind::#variant_name,
                            children
                        );

                        #node_type { node_id, builder }
                    }
                }
            }
        } else {
            // Multiple children in option - treat as sequence when Some
            self.generate_sequence_constructor(node_info, nt)
        }
    }

    fn derive_field_name(
        &self,
        _node_info: &NodeTypesInfo,
        nt: &NonTerminalInfo,
        idx: usize,
    ) -> String {
        let child = &nt.children[idx];

        // Basic field name from the child
        let base_name = match &child.name {
            NodeName::Terminal(term) => format_snake_case(&term.0).to_string(),
            NodeName::NonTerminal(nt_name) => format_snake_case(&nt_name.0).to_string(),
        };

        // Check for naming conflicts and add suffix if needed
        let mut count = 0;
        for (i, other_child) in nt.children.iter().enumerate() {
            if i >= idx {
                break;
            }
            let other_name = match &other_child.name {
                NodeName::Terminal(term) => format_snake_case(&term.0).to_string(),
                NodeName::NonTerminal(nt_name) => format_snake_case(&nt_name.0).to_string(),
            };
            if other_name == base_name {
                count += 1;
            }
        }

        if count > 0 {
            format!("{}{}", base_name, count + 1)
        } else {
            base_name
        }
    }

    fn get_terminal_by_name<'a>(&self, info: &'a NodeTypesInfo, name: &str) -> &'a TerminalInfo {
        info.terminals
            .iter()
            .find(|t| t.name == name)
            .unwrap_or_else(|| panic!("Terminal {name} not found"))
    }

    fn get_non_terminal_by_name<'a>(
        &self,
        info: &'a NodeTypesInfo,
        name: &str,
    ) -> &'a NonTerminalInfo {
        info.non_terminals
            .iter()
            .find(|nt| nt.name == name)
            .unwrap_or_else(|| panic!("Non-terminal {name} not found"))
    }

    fn generate_terminal_constructors(&self, node_info: &NodeTypesInfo) -> TokenStream {
        let constructors: Vec<_> = node_info
            .terminals
            .iter()
            .filter(|t| {
                !t.name.starts_with("Newline")
                    && !t.name.starts_with("Whitespace")
                    && !t.name.starts_with("LineComment")
                    && !t.name.starts_with("BlockComment")
                    && t.name != "NewLine"
            })
            .map(|t| self.generate_terminal_constructor(t))
            .collect();

        quote! {
            pub mod terminals {
                use super::*;

                #(#constructors)*
            }
        }
    }

    fn generate_terminal_constructor(&self, terminal: &TerminalInfo) -> TokenStream {
        let fn_name = format_snake_case(&terminal.name);
        let token_type = format_ident!("{}Token", terminal.name);
        let variant = format_ident!("{}", terminal.variant);

        // Handle special terminal names that conflict with keywords
        let fn_name = match terminal.name.as_str() {
            "True" => format_ident!("r#true"),
            "False" => format_ident!("r#false"),
            "Continue" => format_ident!("r#continue"),
            _ => fn_name,
        };

        // Determine parameter type and default value based on terminal
        let (param_type, param_name, default_value) = match terminal.name.as_str() {
            // Terminals with values
            "Integer" => (quote!(&str), quote!(value), quote!(value)),
            "Str" | "Text" | "Code" | "CodeBlock" | "NamedCode" => {
                (quote!(&str), quote!(value), quote!(value))
            }
            "Ident" => (quote!(&str), quote!(name), quote!(name)),

            // Terminals with fixed values
            "True" => (quote!(), quote!(), quote!("true")),
            "False" => (quote!(), quote!(), quote!("false")),
            "Null" => (quote!(), quote!(), quote!("null")),
            "Bind" => (quote!(), quote!(), quote!("=")),
            "Comma" => (quote!(), quote!(), quote!(",")),
            "Dot" => (quote!(), quote!(), quote!(".")),
            "At" => (quote!(), quote!(), quote!("@")),
            "Dollar" => (quote!(), quote!(), quote!("$")),
            "LBrace" => (quote!(), quote!(), quote!("{")),
            "RBrace" => (quote!(), quote!(), quote!("}")),
            "LBracket" => (quote!(), quote!(), quote!("[")),
            "RBracket" => (quote!(), quote!(), quote!("]")),
            "Continue" => (quote!(), quote!(), quote!("\\")),
            "Esc" => (quote!(), quote!(), quote!("\\\\")),
            "Hole" => (quote!(), quote!(), quote!("!")),

            // Special terminals that might need content
            "Ws" => (quote!(), quote!(), quote!(" ")),
            "GrammarNewline" => (quote!(), quote!(), quote!("\n")),
            "TextStart" => (quote!(), quote!(), quote!("")),
            "Ext" => (quote!(), quote!(), quote!("$")),

            // Default: empty
            _ => (quote!(), quote!(), quote!("")),
        };

        let params = if param_type.is_empty() {
            quote!()
        } else {
            quote!(#param_name: #param_type)
        };

        let terminal_value = if param_type.is_empty() {
            default_value
        } else {
            param_name
        };

        quote! {
            pub fn #fn_name(#params) -> #token_type {
                let mut builder = CstBuilder::new();
                let node_id = builder.terminal(
                    TerminalKind::#variant,
                    #terminal_value
                );
                #token_type { node_id, builder }
            }
        }
    }

    fn generate_branded_types(&self, node_info: &NodeTypesInfo) -> TokenStream {
        let terminal_types = self.generate_terminal_branded_types(node_info);
        let non_terminal_types = self.generate_non_terminal_branded_types(node_info);

        quote! {
            #terminal_types
            #non_terminal_types
        }
    }

    fn generate_terminal_branded_types(&self, node_info: &NodeTypesInfo) -> TokenStream {
        let types: Vec<_> = node_info
            .terminals
            .iter()
            .filter(|t| {
                !t.name.starts_with("Newline")
                    && !t.name.starts_with("Whitespace")
                    && !t.name.starts_with("LineComment")
                    && !t.name.starts_with("BlockComment")
                    && t.name != "NewLine"
            })
            .map(|t| {
                let type_name = format_ident!("{}Token", t.name);
                let doc = format!("Branded type for {} terminal", t.name);

                quote! {
                    #[doc = #doc]
                    #[derive(Debug, Clone)]
                    pub struct #type_name {
                        pub(super) node_id: BuilderNodeId,
                        pub(super) builder: CstBuilder,
                    }

                    impl #type_name {
                        /// Consume this token and return its builder
                        pub fn into_builder(self) -> CstBuilder {
                            self.builder
                        }
                    }

                    impl From<#type_name> for BuilderNodeId {
                        fn from(token: #type_name) -> Self {
                            token.node_id
                        }
                    }
                }
            })
            .collect();

        quote! {
            #(#types)*
        }
    }

    fn generate_non_terminal_branded_types(&self, node_info: &NodeTypesInfo) -> TokenStream {
        let types: Vec<_> = node_info
            .non_terminals
            .iter()
            .map(|nt| {
                let type_name = format_ident!("{}Node", nt.name);
                let doc = format!("Branded type for {} non-terminal", nt.name);

                quote! {
                    #[doc = #doc]
                    #[derive(Debug, Clone)]
                    pub struct #type_name {
                        pub(super) node_id: BuilderNodeId,
                        pub(super) builder: CstBuilder,
                    }

                    impl #type_name {
                        /// Consume this node and return its builder
                        pub fn into_builder(self) -> CstBuilder {
                            self.builder
                        }
                    }

                    impl From<#type_name> for BuilderNodeId {
                        fn from(node: #type_name) -> Self {
                            node.node_id
                        }
                    }
                }
            })
            .collect();

        quote! {
            #(#types)*
        }
    }
}
