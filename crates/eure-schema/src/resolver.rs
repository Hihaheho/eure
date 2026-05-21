//! Schema import graph types.
//!
//! Query-layer code is responsible for loading files and building a
//! [`LoadedSchemaSet`]. The converter only consumes that already-loaded graph.

use std::path::PathBuf;

use eure_document::document::EureDocument;
use eure_document::identifier::Identifier;
use indexmap::IndexMap;
use thiserror::Error;

/// Stable identity of a schema document, used for cycle detection and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResolvedSchemaUri {
    /// A lexically normalized local path. This is intentionally not
    /// canonicalized; query code must not require filesystem access.
    Local(PathBuf),
    /// An opaque inline name (used for tests and bundled schemas).
    Inline(String),
}

impl std::fmt::Display for ResolvedSchemaUri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local(p) => write!(f, "{}", p.display()),
            Self::Inline(s) => write!(f, "<inline:{}>", s),
        }
    }
}

/// Errors raised while resolving a `$import` entry.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ResolverError {
    #[error("imported path {resolved} escapes workspace root {workspace_root}")]
    EscapesWorkspaceRoot {
        resolved: PathBuf,
        workspace_root: PathBuf,
    },

    #[error("absolute schema import paths are not supported in v1: {raw_path}")]
    AbsolutePathUnsupported { raw_path: String },

    #[error("cannot import {scheme} URLs (not yet supported)")]
    UnsupportedScheme { scheme: String },

    #[error("invalid schema import URL {raw_url}: {reason}")]
    InvalidUrl { raw_url: String, reason: String },

    #[error("schema import `{raw_path}` has no local filesystem base: {base}")]
    NonLocalBase {
        raw_path: String,
        base: ResolvedSchemaUri,
    },
}

/// A complete set of schema documents that has already been loaded by the
/// caller. Import edges are keyed by the importing document URI and alias.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedSchemaSet {
    pub root_uri: ResolvedSchemaUri,
    pub documents: IndexMap<ResolvedSchemaUri, EureDocument>,
    pub imports: IndexMap<ResolvedSchemaUri, IndexMap<Identifier, ResolvedSchemaUri>>,
}

impl LoadedSchemaSet {
    pub fn new(root_uri: ResolvedSchemaUri, root_doc: EureDocument) -> Self {
        let mut documents = IndexMap::new();
        documents.insert(root_uri.clone(), root_doc);
        Self {
            root_uri,
            documents,
            imports: IndexMap::new(),
        }
    }

    pub fn insert_document(&mut self, uri: ResolvedSchemaUri, doc: EureDocument) {
        self.documents.insert(uri, doc);
    }

    pub fn insert_import(
        &mut self,
        base: ResolvedSchemaUri,
        alias: Identifier,
        target: ResolvedSchemaUri,
    ) {
        self.imports.entry(base).or_default().insert(alias, target);
    }

    pub fn document(&self, uri: &ResolvedSchemaUri) -> Option<&EureDocument> {
        self.documents.get(uri)
    }

    pub fn import_target(
        &self,
        base: &ResolvedSchemaUri,
        alias: &Identifier,
    ) -> Option<&ResolvedSchemaUri> {
        self.imports.get(base)?.get(alias)
    }
}
