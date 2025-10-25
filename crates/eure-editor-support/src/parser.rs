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

    // Build the CST
    // According to the implementation, tree_builder.build() should never fail
    // because it always constructs a valid CST from the parsed tokens.
    // We use unwrap_or_else to handle the theoretically impossible error case
    // with a clear panic message for debugging if it ever occurs.
    let cst = tree_builder.build().unwrap_or_else(|e| {
        panic!(
            "CST construction unexpectedly failed. This indicates a bug in the parser.\n\
             Error: {:?}\n\
             Please report this issue.",
            e
        )
    });

    // Handle the parse result
    match parse_result {
        Ok(()) => ParseResult::Ok(cst),
        Err(err) => ParseResult::ErrWithCst { cst, error: err },
    }
}
