//! Editor-independent navigation from data fields to their schema sources.

use eure_document::path::PathSegment;
use eure_schema::navigate::{SchemaNavigator, hint_at};
use eure_schema::{SchemaNodeContent, SchemaNodeId};
use eure_tree::tree::InputSpan;
use query_flow::{Db, QueryError};

use super::completion::AnchorKind;
use super::completion::site::find_navigation_site;
use super::{DocumentToSchemaQuery, ParseCst, ResolveSchema, TextFile, ValidatedSchema};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Definition {
    pub file: TextFile,
    pub origin: InputSpan,
    pub selection: InputSpan,
    pub range: InputSpan,
}

#[derive(Debug, thiserror::Error)]
enum DefinitionError {
    #[error("Schema source information is missing for node {0:?}")]
    MissingSource(SchemaNodeId),
}

/// Resolve the cursor using the same tolerant syntax walk as hover/completion.
/// Cursor results are deliberately not memoized.
pub fn get_definition(
    db: &impl Db,
    file: &TextFile,
    offset: u32,
) -> Result<Vec<Definition>, QueryError> {
    let cst = db.query(ParseCst::new(file.clone()))?;
    let text = db.asset(file.clone())?;
    let Some(site) = find_navigation_site(text.get(), &cst.cst, offset) else {
        return Ok(vec![]);
    };
    let Some(anchor) = site.anchor else {
        return Ok(vec![]);
    };
    if anchor.kind == AnchorKind::Value
        && matches!(anchor.path.0.as_slice(), [PathSegment::Extension(ext)] if ext.as_ref() == "schema")
    {
        let resolved = db.query(ResolveSchema::new(file.clone()))?;
        return match resolved.as_ref() {
            Some(resolved) => {
                // Load the exact source now, including suspension for remote files.
                db.asset(resolved.file.clone())?;
                Ok(vec![Definition {
                    file: resolved.file.clone(),
                    origin: anchor.span,
                    selection: InputSpan { start: 0, end: 0 },
                    range: InputSpan { start: 0, end: 0 },
                }])
            }
            None => Ok(vec![]),
        };
    }

    if anchor.kind == AnchorKind::Value
        && let [PathSegment::Extension(ext), alias] = anchor.path.0.as_slice()
        && ext.as_ref() == "import"
    {
        let parsed = db.query(super::ParseDocument::new(file.clone()))?;
        let imports = eure_schema::parse::parse_root_imports(
            &parsed.doc.parse_context(parsed.doc.get_root_id()),
        )?;
        let alias = match alias {
            PathSegment::Ident(name) => Some(name.as_ref()),
            PathSegment::Value(eure_document::value::ObjectKey::String(name)) => {
                Some(name.as_str())
            }
            _ => None,
        };
        if let Some(entry) = alias.and_then(|alias| {
            imports
                .entries
                .iter()
                .find(|(key, _)| key.as_ref() == alias)
                .map(|(_, entry)| entry)
        }) {
            let target = super::schema::resolve_schema_import_text_file(
                file,
                &entry.raw_path,
                super::schema::import_boundary(db, file)?.as_deref(),
            )?;
            db.asset(target.clone())?;
            return Ok(vec![Definition {
                file: target,
                origin: anchor.span,
                selection: InputSpan { start: 0, end: 0 },
                range: InputSpan { start: 0, end: 0 },
            }]);
        }
        return Ok(vec![]);
    }

    // A schema's own type references must be interpreted against that schema,
    // not against the meta-schema which governs the schema document.
    let token = anchor.span.as_str(text.get());
    if anchor.kind == AnchorKind::Value
        && (token.starts_with("`$types.") || token.starts_with("eure-path`$types."))
    {
        let schema = db.query(DocumentToSchemaQuery::new(file.clone()))?;
        for (id, source) in schema.source_map.iter() {
            if schema.source_files.get(&source.uri) != Some(file) {
                continue;
            }
            let parsed = schema
                .parsed_by_uri
                .get(&source.uri)
                .ok_or(DefinitionError::MissingSource(*id))?;
            if parsed
                .origins
                .get_value_span(source.node_id, &cst.cst)
                .is_some_and(|span| span.start <= offset && offset <= span.end)
                && let SchemaNodeContent::Reference(reference) = &schema.schema.node(*id).content
                && let Some(target) = schema.schema.resolve_reference(reference)
            {
                return definitions(db, &schema, [target], anchor.span);
            }
        }
        return Ok(vec![]);
    }

    let resolved = db.query(ResolveSchema::new(file.clone()))?;
    let Some(resolved) = resolved.as_ref() else {
        return Ok(vec![]);
    };
    let schema = db.query(DocumentToSchemaQuery::new(resolved.file.clone()))?;
    let nav = SchemaNavigator::new(&schema.schema);
    let targets = match anchor.path.0.split_last() {
        None => vec![schema.schema.root],
        Some((PathSegment::Extension(ext), parent)) if ext.as_ref() == "variant" => {
            let unions = nav.resolve_to_union(parent, &site.hints);
            if anchor.kind == AnchorKind::Value {
                match hint_at(&site.hints, parent.len()) {
                    Some(selected) => unions
                        .into_iter()
                        .flat_map(|union| nav.descend_variants(union, selected))
                        .collect(),
                    None => vec![],
                }
            } else {
                unions
            }
        }
        Some((last, parent)) => nav
            .resolve(parent, &site.hints)
            .into_iter()
            .filter_map(|parent| nav.step(parent, last))
            .collect(),
    };
    definitions(db, &schema, targets, anchor.span)
}

fn definitions(
    db: &impl Db,
    schema: &ValidatedSchema,
    targets: impl IntoIterator<Item = SchemaNodeId>,
    origin: InputSpan,
) -> Result<Vec<Definition>, QueryError> {
    let mut result = Vec::new();
    for id in targets {
        let source = schema
            .source_map
            .get(&id)
            .ok_or(DefinitionError::MissingSource(id))?;
        let file = schema
            .source_files
            .get(&source.uri)
            .ok_or(DefinitionError::MissingSource(id))?;
        let parsed = schema
            .parsed_by_uri
            .get(&source.uri)
            .ok_or(DefinitionError::MissingSource(id))?;
        let cst = db.query(ParseCst::new(file.clone()))?;
        let value = parsed.origins.get_value_span(source.node_id, &cst.cst);
        let Some(selection) = parsed
            .origins
            .get_definition_span(source.node_id, &cst.cst)
            .or(value)
        else {
            // Implicit nodes have no token to select.
            continue;
        };
        let range = match value {
            Some(value) => InputSpan {
                start: selection.start.min(value.start),
                end: selection.end.max(value.end),
            },
            None => selection,
        };
        let target = Definition {
            file: file.clone(),
            origin,
            selection,
            range,
        };
        if !result.contains(&target) {
            result.push(target);
        }
    }
    Ok(result)
}
