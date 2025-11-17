mod value_visitor;

use eros::Union as _;
use eure_parol::parol_runtime::ParolError;
pub use eure_value::document::*;

use crate::document::value_visitor::{DocumentConstructionError, ValueVisitor};
use eure_tree::prelude::*;

pub fn parse_to_document(
    input: &str,
) -> eros::UResult<EureDocument, (ParolError, DocumentConstructionError)> {
    let tree = eure_parol::parse(input).union()?;
    let document = cst_to_document(input, &tree).union()?;
    Ok(document)
}

pub fn cst_to_document(input: &str, cst: &Cst) -> Result<EureDocument, DocumentConstructionError> {
    let mut visitor = ValueVisitor::new(input);
    visitor.visit_root_handle(cst.root_handle(), cst)?;
    Ok(visitor.into_document())
}
