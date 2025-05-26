use eure_parol::parol_runtime::ParolError;
use eure_parol::tree::CstBuilder;
use eure_parol::{TreeConstruct, parser};
use eure_tree::Cst;

pub enum ParseResult {
    Ok(Cst),
    ErrWithCst { cst: Cst, error: ParolError },
}

/// Parse a document and return a CST
pub fn parse_document(text: &str) -> ParseResult {
    let mut actions = eure_parol::grammar::Grammar::new();
    let mut tree_builder = CstBuilder::new();

    // Parse the document and capture any error
    let parse_result = parser::parse_into(text, &mut tree_builder, "document.eure", &mut actions);

    let cst = tree_builder.build().expect("TreeConstruction never fails");
    // Handle the result
    match parse_result {
        Ok(()) => ParseResult::Ok(cst),
        Err(err) => ParseResult::ErrWithCst { cst, error: err },
    }
}
