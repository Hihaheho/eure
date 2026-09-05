//! Schema-based completion.
//!
//! Pipeline (all pieces are LSP-agnostic so the CLI, the web editor and the
//! test-suite can share them):
//!
//! 1. [`site::find_site`] walks the tolerant CST and reports *where* the
//!    cursor is: the document path and whether a key or a value is typed.
//! 2. The document's schema is resolved and loaded like validation does.
//! 3. [`items::completion_items`] navigates the schema to that path and lists
//!    the fields, variants or values allowed there.

pub mod items;
pub mod site;

use std::sync::Arc;

use eure_schema::SchemaDocument;
use query_flow::{Db, QueryError, query};

pub use items::{CompletionItem, CompletionKind, completion_items};
pub use site::{CompletionSite, SiteKind, ValueStyle, find_site};

use super::assets::TextFile;
use super::parse::ParseCst;
use super::schema::{DocumentToSchemaQuery, ResolveSchema};

/// Completion items for `file` at byte offset `offset`.
///
/// Works on documents that do not parse. Returns an empty list when the
/// cursor is not at a completable position, or when the document has no
/// usable schema and the position is a key.
#[query(debug = "{Self}({file}, {offset})")]
pub fn get_completions(
    db: &impl Db,
    file: TextFile,
    offset: u32,
) -> Result<Vec<CompletionItem>, QueryError> {
    let parsed = db.query(ParseCst::new(file.clone()))?;
    let source = db.asset(file.clone())?;

    let Some(site) = find_site(source.get(), &parsed.cst, offset) else {
        return Ok(Vec::new());
    };

    let schema = load_schema(db, &file)?;
    Ok(completion_items(&site, schema.as_deref()))
}

/// Load the schema that governs `file`, if any.
///
/// A schema that cannot be resolved or converted yields `None`: the problem
/// is reported through diagnostics, and completion degrades to
/// schema-less behavior instead of failing the request.
fn load_schema(db: &impl Db, file: &TextFile) -> Result<Option<Arc<SchemaDocument>>, QueryError> {
    let resolved = match db.query(ResolveSchema::new(file.clone())) {
        Ok(resolved) => resolved.as_ref().clone(),
        Err(QueryError::UserError(_)) => return Ok(None),
        Err(error) => return Err(error),
    };
    let Some(resolved) = resolved else {
        return Ok(None);
    };
    match db.query(DocumentToSchemaQuery::new(resolved.file)) {
        Ok(validated) => Ok(Some(validated.schema.clone())),
        Err(QueryError::UserError(_)) => Ok(None),
        Err(error) => Err(error),
    }
}
