use parol_walker_gen::{ImportPaths, NamingConfig, WalkerConfig};
use std::path::Path;

fn main() {
    // Tell Cargo to rerun if the grammar changes
    println!("cargo:rerun-if-changed=calc.par");

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir);

    // Generate parser using parol
    let node_info = parol::build::Builder::with_explicit_output_dir(out_path)
        .grammar_file("calc.par")
        .parser_output_file("parser.rs")
        .actions_output_file("grammar_trait.rs")
        .node_kind_enums()
        .node_kind_enums_output_file("node_kind.rs")
        .user_type_name("Grammar")
        .user_trait_module_name("grammar")
        .range()
        .minimize_boxed_types()
        .max_lookahead(1)
        .unwrap()
        .generate_parser_and_export_node_infos()
        .unwrap();

    // Post-process grammar_trait.rs to fix inner attributes for include! compatibility
    let grammar_trait_path = out_path.join("grammar_trait.rs");
    let content = std::fs::read_to_string(&grammar_trait_path).unwrap();
    let fixed_content = content
        .replace(
            "#![allow(clippy::enum_variant_names)]",
            "#[allow(clippy::enum_variant_names)]",
        )
        .replace(
            "#![allow(clippy::large_enum_variant)]",
            "#[allow(clippy::large_enum_variant)]",
        )
        .replace(
            "#![allow(clippy::upper_case_acronyms)]",
            "#[allow(clippy::upper_case_acronyms)]",
        );
    std::fs::write(&grammar_trait_path, fixed_content).unwrap();

    // Configure parol-walker code generation
    // Use parol_walker crate for runtime types
    let config = WalkerConfig {
        naming: NamingConfig::default(),
        imports: ImportPaths {
            runtime_crate: "parol_walker".into(),
            node_kind_module: "crate::node_kind".into(),
            nodes_module: "crate::nodes".into(),
        },
    };

    // Generate visitor and node types
    parol_walker_gen::generate_visitor(&node_info, &config, &out_path.join("visitor.rs")).unwrap();
    parol_walker_gen::generate_nodes(&node_info, &config, &out_path.join("nodes.rs")).unwrap();
}
