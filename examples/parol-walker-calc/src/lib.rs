pub mod grammar;
pub mod tree;

#[allow(clippy::enum_variant_names)]
#[allow(clippy::large_enum_variant)]
#[allow(clippy::upper_case_acronyms)]
pub mod grammar_trait {
    include!(concat!(env!("OUT_DIR"), "/grammar_trait.rs"));
}

pub mod parser {
    include!(concat!(env!("OUT_DIR"), "/parser.rs"));
}

pub mod node_kind {
    include!(concat!(env!("OUT_DIR"), "/node_kind.rs"));
}

pub mod nodes {
    include!(concat!(env!("OUT_DIR"), "/nodes.rs"));
}

pub mod visitor {
    include!(concat!(env!("OUT_DIR"), "/visitor.rs"));
}

use tree::{Cst, CstBuilder};

/// Parse arithmetic expression and return CST
pub fn parse(input: &str) -> Result<Cst, parol_runtime::ParolError> {
    let mut actions = grammar::Grammar::new();
    let mut tree_builder = CstBuilder::new();
    parser::parse_into(input, &mut tree_builder, "calc.input", &mut actions)?;
    Ok(tree_builder.build_tree())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_number() {
        let input = "42";
        let tree = parse(input);
        assert!(tree.is_ok());
    }

    #[test]
    fn test_parse_addition() {
        let input = "1 + 2";
        let tree = parse(input);
        assert!(tree.is_ok());
    }

    #[test]
    fn test_parse_complex_expression() {
        let input = "(2 + 3) * 4 - 1";
        let tree = parse(input);
        assert!(tree.is_ok());
    }
}
