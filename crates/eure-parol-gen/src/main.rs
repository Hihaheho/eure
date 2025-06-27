mod ast_type_generator;
mod constructor_generator;
mod visitor_generator;

use std::{path::Path, process::Command};

use ast_type_generator::AstTypeGenerator;
use constructor_generator::ConstructorGenerator;
use convert_case::{Case, Casing as _};
use parol::generators::export_node_types::{NodeName, NodeTypesInfo};
use quote::format_ident;
use syn::parse_quote;
use visitor_generator::VisitorGenerator;

fn main() {
    let mut node_info = parol::build::Builder::with_explicit_output_dir("crates/eure-parol")
        .grammar_file("crates/eure-parol/eure.par")
        .parser_output_file("src/parser.rs")
        .actions_output_file("src/grammar_trait.rs")
        .node_kind_enums()
        .node_kind_enums_output_file("../eure-tree/src/node_kind.rs")
        .expanded_grammar_output_file("eure-expanded.par")
        .user_type_name("Grammar")
        .user_trait_module_name("grammar")
        .range()
        .minimize_boxed_types()
        // 2 for trailing comma
        .max_lookahead(1)
        .unwrap()
        .generate_parser_and_export_node_infos()
        .unwrap();

    format_node_info(&node_info);
    rename_non_terminal_names(&mut node_info);
    let mut ast_type_generator =
        AstTypeGenerator::new(Path::new("crates/eure-tree/src/nodes.rs").into());
    ast_type_generator.generate(&node_info);
    let visitor_generator =
        VisitorGenerator::new(Path::new("crates/eure-tree/src/visitor.rs").into());
    visitor_generator.generate(&node_info);
    let constructor_generator =
        ConstructorGenerator::new(Path::new("crates/eure-tree/src/generated/constructors.rs").into());
    constructor_generator.generate(&node_info);
    generate_node_kind("crates/eure-tree/src/node_kind.rs");

    Command::new("cargo")
        .args(["fmt", "-p", "crates/eure-tree"])
        .output()
        .unwrap();
}

fn format_node_info(node_info: &NodeTypesInfo) {
    println!("Terminals:");
    for terminal in &node_info.terminals {
        println!("  {} {}", terminal.name, terminal.variant);
    }
    println!("Non-terminals:");
    for non_terminal in &node_info.non_terminals {
        println!(
            "  {} {} {:?}",
            non_terminal.name, non_terminal.variant, non_terminal.kind
        );
        for child in &non_terminal.children {
            println!("    {:?} {:?}", child.kind, child.name);
        }
    }
}

fn rename_non_terminal_name(name: &mut String) {
    match name.as_str() {
        "EureList" => *name = "EureBindings".to_string(),
        "EureList0" => *name = "EureSections".to_string(),
        _ => {}
    }
}

fn format_snake_case(name: &str) -> syn::Ident {
    let name = name.from_case(Case::Camel).to_case(Case::Snake);
    match name.as_str() {
        "true" => format_ident!("r#true"),
        "false" => format_ident!("r#false"),
        "continue" => format_ident!("r#continue"),
        _ => format_ident!("{}", name),
    }
}

fn rename_non_terminal_names(info: &mut NodeTypesInfo) {
    for nt in &mut info.non_terminals {
        rename_non_terminal_name(&mut nt.name);
        for child in &mut nt.children {
            if let NodeName::NonTerminal(name) = &mut child.name {
                rename_non_terminal_name(&mut name.0);
            }
        }
    }
}

fn generate_node_kind(path: &str) {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new().append(true).open(path).unwrap();
    let content: syn::File = parse_quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum NodeKind<T, Nt> {
            Terminal(T),
            NonTerminal(Nt),
        }
    };

    write!(file, "{}", prettyplease::unparse(&content)).unwrap();
}
