use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Write as _};
use std::path::Path;
use std::str::FromStr;

use eros::ErrorUnion;
use num_bigint::BigInt;
use thiserror::Error;

use eure_document::document::node::NodeValue;
use eure_document::document::{EureDocument, NodeId};
use eure_document::identifier::{Identifier, IdentifierError};
use eure_document::path::{ArrayIndexKind, PathSegment};
use eure_document::plan::traverse;
use eure_document::plan::{ArrayForm, Form, LayoutPlan, PlanBuilder, PlanError};
use eure_document::source::{
    BindSource, BindingSource, Comment, EureSource, SectionBody, SourceDocument, SourceKey,
    SourcePath, StringStyle, Trivia,
};
use eure_document::text::{Text, TextParseError};
use eure_document::value::{ObjectKey, Tuple};
use eure_fmt::format_source_document;

use crate::document::{DocumentConstructionError, parse_to_document, parse_to_source_document};
use crate::parol::EureParseError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditPath(pub Vec<EditPathSegment>);

impl EditPath {
    pub fn root() -> Self {
        Self(Vec::new())
    }

    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditPathSegment {
    pub key: EditKey,
    pub array: Option<EditArraySelector>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditKey {
    Ident(Identifier),
    Extension(Identifier),
    String(String, StringStyle),
    TupleIndex(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditArraySelector {
    Append,
    Index(usize),
    Tail,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EditPathParseError {
    #[error("unexpected end of path")]
    UnexpectedEof,
    #[error("expected path segment at byte {offset}")]
    ExpectedSegment { offset: usize },
    #[error("invalid identifier at byte {offset}: {error}")]
    InvalidIdentifier {
        offset: usize,
        error: IdentifierError,
    },
    #[error("invalid tuple index `{value}` at byte {offset}")]
    InvalidTupleIndex { offset: usize, value: String },
    #[error("invalid array index `{value}` at byte {offset}")]
    InvalidArrayIndex { offset: usize, value: String },
    #[error("unterminated quoted string at byte {offset}")]
    UnterminatedQuotedString { offset: usize },
    #[error("invalid quoted string at byte {offset}: {error}")]
    InvalidQuotedString {
        offset: usize,
        error: TextParseError,
    },
    #[error("unexpected character `{ch}` at byte {offset}")]
    UnexpectedCharacter { offset: usize, ch: char },
    #[error("array selector must follow a key segment at byte {offset}")]
    ArrayWithoutKey { offset: usize },
}

impl FromStr for EditPath {
    type Err = EditPathParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        if input.is_empty() {
            return Ok(Self::root());
        }

        let bytes = input.as_bytes();
        let mut i = 0usize;
        let mut segments = Vec::new();

        while i < bytes.len() {
            let (key, next) = parse_edit_key(input, i)?;
            i = next;

            let mut array = None;
            if i < bytes.len() && bytes[i] == b'[' {
                let (selector, next) = parse_array_selector(input, i)?;
                array = Some(selector);
                i = next;
            }

            segments.push(EditPathSegment { key, array });

            if i == bytes.len() {
                break;
            }
            if bytes[i] != b'.' {
                return Err(EditPathParseError::UnexpectedCharacter {
                    offset: i,
                    ch: input[i..].chars().next().unwrap_or('\0'),
                });
            }
            i += 1;
            if i == bytes.len() {
                return Err(EditPathParseError::UnexpectedEof);
            }
        }

        Ok(Self(segments))
    }
}

fn parse_edit_key(input: &str, offset: usize) -> Result<(EditKey, usize), EditPathParseError> {
    let bytes = input.as_bytes();
    let ch = input[offset..]
        .chars()
        .next()
        .ok_or(EditPathParseError::UnexpectedEof)?;
    match ch {
        '$' => {
            let (raw, next) = parse_ident_like(input, offset + 1);
            let ident = raw
                .parse()
                .map_err(|error| EditPathParseError::InvalidIdentifier { offset, error })?;
            Ok((EditKey::Extension(ident), next))
        }
        '#' => {
            let start = offset + 1;
            let mut end = start;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            if start == end {
                return Err(EditPathParseError::InvalidTupleIndex {
                    offset,
                    value: String::new(),
                });
            }
            let raw = &input[start..end];
            let index = raw
                .parse()
                .map_err(|_| EditPathParseError::InvalidTupleIndex {
                    offset,
                    value: raw.to_string(),
                })?;
            Ok((EditKey::TupleIndex(index), end))
        }
        '"' => {
            let mut end = offset + 1;
            let mut escaped = false;
            while end < bytes.len() {
                let b = bytes[end];
                if escaped {
                    escaped = false;
                    end += 1;
                    continue;
                }
                if b == b'\\' {
                    escaped = true;
                    end += 1;
                    continue;
                }
                if b == b'"' {
                    let content = &input[offset + 1..end];
                    let text = Text::parse_quoted_string(content).map_err(|error| {
                        EditPathParseError::InvalidQuotedString { offset, error }
                    })?;
                    return Ok((EditKey::String(text.content, StringStyle::Quoted), end + 1));
                }
                end += 1;
            }
            Err(EditPathParseError::UnterminatedQuotedString { offset })
        }
        _ => {
            let (raw, next) = parse_ident_like(input, offset);
            if raw.is_empty() {
                return Err(EditPathParseError::ExpectedSegment { offset });
            }
            let ident = raw
                .parse()
                .map_err(|error| EditPathParseError::InvalidIdentifier { offset, error })?;
            Ok((EditKey::Ident(ident), next))
        }
    }
}

fn parse_ident_like(input: &str, offset: usize) -> (&str, usize) {
    let bytes = input.as_bytes();
    let mut end = offset;
    while end < bytes.len() {
        match bytes[end] {
            b'.' | b'[' => break,
            _ => end += 1,
        }
    }
    (&input[offset..end], end)
}

fn parse_array_selector(
    input: &str,
    offset: usize,
) -> Result<(EditArraySelector, usize), EditPathParseError> {
    let bytes = input.as_bytes();
    if bytes.get(offset) != Some(&b'[') {
        return Err(EditPathParseError::ArrayWithoutKey { offset });
    }
    let start = offset + 1;
    let mut end = start;
    while end < bytes.len() && bytes[end] != b']' {
        end += 1;
    }
    if end >= bytes.len() {
        return Err(EditPathParseError::UnexpectedEof);
    }
    let raw = &input[start..end];
    let selector = if raw.is_empty() {
        EditArraySelector::Append
    } else if raw == "^" {
        EditArraySelector::Tail
    } else {
        let index = raw
            .parse()
            .map_err(|_| EditPathParseError::InvalidArrayIndex {
                offset,
                value: raw.to_string(),
            })?;
        EditArraySelector::Index(index)
    };
    Ok((selector, end + 1))
}

impl Display for EditPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            return write!(f, "(root)");
        }
        for (index, segment) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_char('.')?;
            }
            match &segment.key {
                EditKey::Ident(ident) => write!(f, "{ident}")?,
                EditKey::Extension(ident) => write!(f, "${ident}")?,
                EditKey::String(value, style) => match style {
                    StringStyle::Quoted => write!(f, "\"{}\"", escape_quoted(value))?,
                    StringStyle::Literal => write!(f, "'{value}'")?,
                    StringStyle::DelimitedLitStr(level) => {
                        let open = "<".repeat(*level as usize);
                        let close = ">".repeat(*level as usize);
                        write!(f, "{open}'{value}'{close}")?;
                    }
                    StringStyle::DelimitedCode(level) => {
                        let open = "<".repeat(*level as usize);
                        let close = ">".repeat(*level as usize);
                        write!(f, "{open}`{value}`{close}")?;
                    }
                },
                EditKey::TupleIndex(tuple_index) => write!(f, "#{tuple_index}")?,
            }
            if let Some(array) = segment.array {
                match array {
                    EditArraySelector::Append => f.write_str("[]")?,
                    EditArraySelector::Tail => f.write_str("[^]")?,
                    EditArraySelector::Index(array_index) => write!(f, "[{array_index}]")?,
                }
            }
        }
        Ok(())
    }
}

fn escape_quoted(input: &str) -> String {
    input
        .chars()
        .flat_map(|ch| match ch {
            '\\' => ['\\', '\\'].into_iter().collect::<Vec<_>>(),
            '"' => ['\\', '"'].into_iter().collect::<Vec<_>>(),
            '\n' => ['\\', 'n'].into_iter().collect::<Vec<_>>(),
            '\r' => ['\\', 'r'].into_iter().collect::<Vec<_>>(),
            '\t' => ['\\', 't'].into_iter().collect::<Vec<_>>(),
            other => [other].into_iter().collect::<Vec<_>>(),
        })
        .collect()
}

#[derive(Debug, Clone)]
pub enum EditValue {
    Document(EureDocument),
    Source(SourceDocument),
}

impl EditValue {
    fn document(&self) -> &EureDocument {
        match self {
            EditValue::Document(doc) => doc,
            EditValue::Source(source) => &source.document,
        }
    }

    fn clone_document(&self) -> EureDocument {
        self.document().clone()
    }
}

impl From<EureDocument> for EditValue {
    fn from(value: EureDocument) -> Self {
        Self::Document(value)
    }
}

impl From<SourceDocument> for EditValue {
    fn from(value: SourceDocument) -> Self {
        Self::Source(value)
    }
}

#[derive(Debug, Clone)]
pub enum EditCommand {
    Set { path: EditPath, value: EditValue },
    Insert { path: EditPath, value: EditValue },
    Delete { path: EditPath },
}

#[derive(Debug, Error)]
pub enum EditError {
    #[error(transparent)]
    PathParse(#[from] EditPathParseError),
    #[error("failed to parse Eure source: {0}")]
    ParseSyntax(#[from] EureParseError),
    #[error("failed to construct Eure document: {0}")]
    ParseDocument(#[from] DocumentConstructionError),
    #[error("failed to build layout plan: {0}")]
    Plan(#[from] PlanError),
    #[error("root delete is not supported")]
    DeleteRoot,
    #[error("delete path {path} cannot use []")]
    DeleteAppendSelector { path: EditPath },
    #[error("insert path {path} must end at a numeric array selector like a[0]")]
    InsertRequiresNumericSelector { path: EditPath },
    #[error("insert path {path} cannot use [] or [^]")]
    InsertInvalidSelector { path: EditPath },
    #[error("insert path {path} with index {index} equals array length {len}; use [] instead")]
    InsertAtLenUseAppend {
        path: EditPath,
        index: usize,
        len: usize,
    },
    #[error("array tail selector [^] is invalid on empty array at {path}")]
    TailOnEmptyArray { path: EditPath },
    #[error("array index {index} is out of bounds for length {len} at {path}")]
    ArrayIndexOutOfBounds {
        path: EditPath,
        index: usize,
        len: usize,
    },
    #[error("missing path {path}")]
    MissingPath { path: EditPath },
    #[error("expected map-like container at {path}")]
    ExpectedMap { path: EditPath },
    #[error("expected array at {path}")]
    ExpectedArray { path: EditPath },
    #[error("expected tuple at {path}")]
    ExpectedTuple { path: EditPath },
    #[error("failed to parse edit value `{input}`: {message}")]
    ValueParse { input: String, message: String },
}

pub struct EditableDocument {
    source: SourceDocument,
}

impl EditableDocument {
    pub fn parse(input: &str, name: impl AsRef<Path>) -> Result<Self, EditError> {
        let source = parse_to_source_document(input, name).map_err(map_parse_union_error)?;
        Ok(Self { source })
    }

    pub fn document(&self) -> &EureDocument {
        &self.source.document
    }

    pub fn source_document(&self) -> &SourceDocument {
        &self.source
    }

    pub fn render(&self) -> String {
        format_source_document(&self.source)
    }

    pub fn apply(&mut self, command: EditCommand) -> Result<(), EditError> {
        match command {
            EditCommand::Set { path, value } => self.apply_set(path, value),
            EditCommand::Insert { path, value } => self.apply_insert(path, value),
            EditCommand::Delete { path } => self.apply_delete(path),
        }
    }

    pub fn apply_all(
        &mut self,
        commands: impl IntoIterator<Item = EditCommand>,
    ) -> Result<(), EditError> {
        for command in commands {
            self.apply(command)?;
        }
        Ok(())
    }

    pub fn set(&mut self, path: impl AsRef<str>, value_eure: &str) -> Result<(), EditError> {
        let path = EditPath::from_str(path.as_ref())?;
        let value = EditValue::Document(parse_edit_value_document(value_eure)?);
        self.apply(EditCommand::Set { path, value })
    }

    pub fn insert(&mut self, path: impl AsRef<str>, value_eure: &str) -> Result<(), EditError> {
        let path = EditPath::from_str(path.as_ref())?;
        let value = EditValue::Document(parse_edit_value_document(value_eure)?);
        self.apply(EditCommand::Insert { path, value })
    }

    pub fn delete(&mut self, path: impl AsRef<str>) -> Result<(), EditError> {
        let path = EditPath::from_str(path.as_ref())?;
        self.apply(EditCommand::Delete { path })
    }

    fn apply_set(&mut self, path: EditPath, value: EditValue) -> Result<(), EditError> {
        let old_source = self.source.clone();
        let replacement = value.clone_document();
        apply_set_to_document(&mut self.source.document, &path, &replacement)?;
        self.source = rebuild_source_document(&old_source, self.source.document.clone())?;
        Ok(())
    }

    fn apply_insert(&mut self, path: EditPath, value: EditValue) -> Result<(), EditError> {
        let old_source = self.source.clone();
        let replacement = value.clone_document();
        apply_insert_to_document(&mut self.source.document, &path, &replacement)?;
        self.source = rebuild_source_document(&old_source, self.source.document.clone())?;
        Ok(())
    }

    fn apply_delete(&mut self, path: EditPath) -> Result<(), EditError> {
        let old_source = self.source.clone();
        apply_delete_to_document(&mut self.source.document, &path)?;
        self.source = rebuild_source_document(&old_source, self.source.document.clone())?;
        Ok(())
    }
}

fn parse_edit_value_document(input: &str) -> Result<EureDocument, EditError> {
    match parse_to_document(input, "<input>") {
        Ok(doc) => Ok(doc),
        Err(first_error) => {
            let wrapped = format!("= {input}");
            parse_to_document(&wrapped, "<input>").map_err(|_| EditError::ValueParse {
                input: input.to_string(),
                message: format!("{}", map_parse_union_error(first_error)),
            })
        }
    }
}

fn map_parse_union_error(
    error: ErrorUnion<(EureParseError, DocumentConstructionError)>,
) -> EditError {
    match error.narrow::<EureParseError, _>() {
        Ok(parse_error) => EditError::ParseSyntax(parse_error),
        Err(error) => EditError::ParseDocument(error.take()),
    }
}

fn apply_set_to_document(
    document: &mut EureDocument,
    path: &EditPath,
    replacement: &EureDocument,
) -> Result<(), EditError> {
    if path.is_root() {
        let root_id = document.get_root_id();
        document.overwrite_subtree_from(root_id, replacement, replacement.get_root_id());
        return Ok(());
    }

    let root_id = document.get_root_id();
    set_into_node(
        document,
        root_id,
        &path.0,
        path,
        replacement,
        replacement.get_root_id(),
    )
}

fn set_into_node(
    document: &mut EureDocument,
    current: NodeId,
    segments: &[EditPathSegment],
    full_path: &EditPath,
    replacement: &EureDocument,
    replacement_root: NodeId,
) -> Result<(), EditError> {
    let (segment, rest) = segments
        .split_first()
        .expect("set_into_node only called with non-empty path");
    let is_last = rest.is_empty();

    let child = ensure_key_child(document, current, &segment.key, full_path)?;
    match segment.array {
        None => {
            if is_last {
                document.overwrite_subtree_from(child, replacement, replacement_root);
                Ok(())
            } else {
                set_into_node(
                    document,
                    child,
                    rest,
                    full_path,
                    replacement,
                    replacement_root,
                )
            }
        }
        Some(EditArraySelector::Append) => {
            let array_path = full_path.clone();
            ensure_array_node(document, child, &array_path)?;
            let element = document
                .add_array_element(None, child)
                .map_err(|_| EditError::ExpectedArray {
                    path: array_path.clone(),
                })?
                .node_id;
            if is_last {
                document.overwrite_subtree_from(element, replacement, replacement_root);
                Ok(())
            } else {
                set_into_node(
                    document,
                    element,
                    rest,
                    full_path,
                    replacement,
                    replacement_root,
                )
            }
        }
        Some(EditArraySelector::Index(index)) => {
            let array_path = full_path.clone();
            let element = get_existing_array_element(document, child, index, &array_path)?;
            if is_last {
                document.overwrite_subtree_from(element, replacement, replacement_root);
                Ok(())
            } else {
                set_into_node(
                    document,
                    element,
                    rest,
                    full_path,
                    replacement,
                    replacement_root,
                )
            }
        }
        Some(EditArraySelector::Tail) => {
            let array_path = full_path.clone();
            let element = get_tail_array_element(document, child, &array_path)?;
            if is_last {
                document.overwrite_subtree_from(element, replacement, replacement_root);
                Ok(())
            } else {
                set_into_node(
                    document,
                    element,
                    rest,
                    full_path,
                    replacement,
                    replacement_root,
                )
            }
        }
    }
}

fn apply_insert_to_document(
    document: &mut EureDocument,
    path: &EditPath,
    replacement: &EureDocument,
) -> Result<(), EditError> {
    let Some((segment_index, selector)) = last_array_selector(path) else {
        return Err(EditError::InsertRequiresNumericSelector { path: path.clone() });
    };

    let insert_index = match selector {
        EditArraySelector::Index(index) => index,
        EditArraySelector::Append | EditArraySelector::Tail => {
            return Err(EditError::InsertInvalidSelector { path: path.clone() });
        }
    };

    let target_segment = &path.0[segment_index];
    if !matches!(target_segment.array, Some(EditArraySelector::Index(_))) {
        return Err(EditError::InsertInvalidSelector { path: path.clone() });
    }

    let mut current = document.get_root_id();
    for (i, segment) in path.0.iter().enumerate() {
        let is_target = i == segment_index;
        let key_path = prefix_path(path, i);
        let child = get_existing_key_child(document, current, &segment.key, &key_path)?;

        if is_target {
            let array_path = prefix_path(path, i);
            let len = ensure_array_len(document, child, &array_path)?;
            if insert_index > len {
                return Err(EditError::ArrayIndexOutOfBounds {
                    path: array_path,
                    index: insert_index,
                    len,
                });
            }
            if insert_index == len {
                return Err(EditError::InsertAtLenUseAppend {
                    path: path.clone(),
                    index: insert_index,
                    len,
                });
            }
            let inserted = document
                .insert_array_element(insert_index, child)
                .map_err(|_| EditError::ExpectedArray {
                    path: prefix_path(path, i),
                })?
                .node_id;
            if i + 1 == path.0.len() {
                document.overwrite_subtree_from(inserted, replacement, replacement.get_root_id());
                return Ok(());
            }
            return set_into_node(
                document,
                inserted,
                &path.0[i + 1..],
                path,
                replacement,
                replacement.get_root_id(),
            );
        }

        current = match segment.array {
            None => child,
            Some(EditArraySelector::Index(index)) => {
                get_existing_array_element(document, child, index, &key_path)?
            }
            Some(EditArraySelector::Tail) => get_tail_array_element(document, child, &key_path)?,
            Some(EditArraySelector::Append) => {
                return Err(EditError::InsertInvalidSelector { path: path.clone() });
            }
        };
    }

    Err(EditError::InsertRequiresNumericSelector { path: path.clone() })
}

fn apply_delete_to_document(document: &mut EureDocument, path: &EditPath) -> Result<(), EditError> {
    if path.is_root() {
        return Err(EditError::DeleteRoot);
    }

    let mut current = document.get_root_id();
    for (i, segment) in path.0.iter().enumerate() {
        let is_last = i + 1 == path.0.len();
        let key_path = prefix_path(path, i);
        let child = get_existing_key_child(document, current, &segment.key, &key_path)?;
        match segment.array {
            None => {
                if is_last {
                    remove_key_child(document, current, &segment.key, path)?;
                    return Ok(());
                }
                current = child;
            }
            Some(EditArraySelector::Append) => {
                return Err(EditError::DeleteAppendSelector { path: path.clone() });
            }
            Some(EditArraySelector::Index(index)) => {
                if is_last {
                    let len = ensure_array_len(document, child, &key_path)?;
                    if index >= len {
                        return Err(EditError::ArrayIndexOutOfBounds {
                            path: key_path,
                            index,
                            len,
                        });
                    }
                    document.remove_array_element(index, child).map_err(|_| {
                        EditError::ExpectedArray {
                            path: prefix_path(path, i),
                        }
                    })?;
                    return Ok(());
                }
                current = get_existing_array_element(document, child, index, &key_path)?;
            }
            Some(EditArraySelector::Tail) => {
                if is_last {
                    let len = ensure_array_len(document, child, &key_path)?;
                    if len == 0 {
                        return Err(EditError::TailOnEmptyArray { path: key_path });
                    }
                    document.remove_array_element(len - 1, child).map_err(|_| {
                        EditError::ExpectedArray {
                            path: prefix_path(path, i),
                        }
                    })?;
                    return Ok(());
                }
                current = get_tail_array_element(document, child, &key_path)?;
            }
        }
    }

    Err(EditError::MissingPath { path: path.clone() })
}

fn ensure_key_child(
    document: &mut EureDocument,
    parent: NodeId,
    key: &EditKey,
    path: &EditPath,
) -> Result<NodeId, EditError> {
    if let Some(node_id) = lookup_key_child(document, parent, key) {
        return Ok(node_id);
    }

    match key {
        EditKey::Ident(identifier) => document
            .add_map_child(ObjectKey::String(identifier.to_string()), parent)
            .map(|child| child.node_id)
            .map_err(|_| EditError::ExpectedMap { path: path.clone() }),
        EditKey::Extension(identifier) => document
            .add_extension(identifier.clone(), parent)
            .map(|child| child.node_id)
            .map_err(|_| EditError::ExpectedMap { path: path.clone() }),
        EditKey::String(value, _) => document
            .add_map_child(ObjectKey::String(value.clone()), parent)
            .map(|child| child.node_id)
            .map_err(|_| EditError::ExpectedMap { path: path.clone() }),
        EditKey::TupleIndex(index) => {
            let tuple_len = match &document.node(parent).content {
                NodeValue::Hole(_) => 0,
                NodeValue::Tuple(tuple) => tuple.len(),
                _ => return Err(EditError::ExpectedTuple { path: path.clone() }),
            };
            if *index as usize != tuple_len {
                return Err(EditError::MissingPath { path: path.clone() });
            }
            document
                .add_tuple_element(*index, parent)
                .map(|child| child.node_id)
                .map_err(|_| EditError::ExpectedTuple { path: path.clone() })
        }
    }
}

fn get_existing_key_child(
    document: &EureDocument,
    parent: NodeId,
    key: &EditKey,
    path: &EditPath,
) -> Result<NodeId, EditError> {
    lookup_key_child(document, parent, key)
        .ok_or_else(|| EditError::MissingPath { path: path.clone() })
}

fn remove_key_child(
    document: &mut EureDocument,
    parent: NodeId,
    key: &EditKey,
    path: &EditPath,
) -> Result<(), EditError> {
    match key {
        EditKey::Ident(identifier) => {
            let key = ObjectKey::String(identifier.to_string());
            document
                .remove_map_child(&key, parent)
                .map_err(|_| EditError::ExpectedMap { path: path.clone() })?;
            Ok(())
        }
        EditKey::Extension(identifier) => {
            document.remove_extension(identifier, parent);
            Ok(())
        }
        EditKey::String(value, _) => {
            let key = ObjectKey::String(value.clone());
            document
                .remove_map_child(&key, parent)
                .map_err(|_| EditError::ExpectedMap { path: path.clone() })?;
            Ok(())
        }
        EditKey::TupleIndex(_) => Err(EditError::ExpectedTuple { path: path.clone() }),
    }
}

fn lookup_key_child(document: &EureDocument, parent: NodeId, key: &EditKey) -> Option<NodeId> {
    match key {
        EditKey::Ident(identifier) => document
            .node(parent)
            .as_map()
            .and_then(|map| map.get_node_id(&ObjectKey::String(identifier.to_string()))),
        EditKey::Extension(identifier) => document.node(parent).get_extension(identifier),
        EditKey::String(value, _) => document
            .node(parent)
            .as_map()
            .and_then(|map| map.get_node_id(&ObjectKey::String(value.clone()))),
        EditKey::TupleIndex(index) => document
            .node(parent)
            .as_tuple()
            .and_then(|tuple| tuple.get(*index as usize)),
    }
}

fn ensure_array_node(
    document: &EureDocument,
    node_id: NodeId,
    path: &EditPath,
) -> Result<(), EditError> {
    if document.node(node_id).as_array().is_some()
        || matches!(document.node(node_id).content, NodeValue::Hole(_))
    {
        Ok(())
    } else {
        Err(EditError::ExpectedArray { path: path.clone() })
    }
}

fn ensure_array_len(
    document: &EureDocument,
    node_id: NodeId,
    path: &EditPath,
) -> Result<usize, EditError> {
    document
        .node(node_id)
        .as_array()
        .map(|array| array.len())
        .ok_or_else(|| EditError::ExpectedArray { path: path.clone() })
}

fn get_existing_array_element(
    document: &EureDocument,
    array_node: NodeId,
    index: usize,
    path: &EditPath,
) -> Result<NodeId, EditError> {
    let len = ensure_array_len(document, array_node, path)?;
    document
        .node(array_node)
        .as_array()
        .and_then(|array| array.get(index))
        .ok_or_else(|| EditError::ArrayIndexOutOfBounds {
            path: path.clone(),
            index,
            len,
        })
}

fn get_tail_array_element(
    document: &EureDocument,
    array_node: NodeId,
    path: &EditPath,
) -> Result<NodeId, EditError> {
    let len = ensure_array_len(document, array_node, path)?;
    if len == 0 {
        return Err(EditError::TailOnEmptyArray { path: path.clone() });
    }
    Ok(document
        .node(array_node)
        .as_array()
        .and_then(|array| array.get(len - 1))
        .expect("non-empty array has a tail element"))
}

fn prefix_path(path: &EditPath, inclusive_segment_index: usize) -> EditPath {
    EditPath(path.0[..=inclusive_segment_index].to_vec())
}

fn last_array_selector(path: &EditPath) -> Option<(usize, EditArraySelector)> {
    let mut last = None;
    for (index, segment) in path.0.iter().enumerate() {
        if let Some(selector) = segment.array {
            last = Some((index, selector));
        }
    }
    last
}

fn rebuild_source_document(
    old_source: &SourceDocument,
    new_document: EureDocument,
) -> Result<SourceDocument, EditError> {
    let mut builder = LayoutPlan::builder(new_document.clone());
    seed_plan_from_source(old_source, &mut builder);
    let mut source = builder.build()?.emit();
    merge_source_metadata(old_source, &mut source);
    Ok(source)
}

fn seed_plan_from_source(old_source: &SourceDocument, builder: &mut PlanBuilder) {
    let reachable: HashSet<NodeId> = traverse::all_reachable_ids(builder.document())
        .into_iter()
        .collect();
    seed_container_plan(
        old_source,
        old_source.root,
        old_source.document.get_root_id(),
        builder,
        &reachable,
    );
}

fn seed_container_plan(
    old_source_doc: &SourceDocument,
    source_id: eure_document::source::SourceId,
    base_node: NodeId,
    builder: &mut PlanBuilder,
    reachable: &HashSet<NodeId>,
) {
    let source = old_source_doc.source(source_id);
    let mut resolver = SourceResolveState::default();

    for binding in &source.bindings {
        let Some(resolved) = resolve_source_path(
            old_source_doc.document(),
            base_node,
            &binding.path,
            &mut resolver,
        ) else {
            continue;
        };
        match &binding.bind {
            BindSource::Value(node) | BindSource::Array { node, .. } => {
                if matches!(
                    builder.document().node(resolved.target).content,
                    NodeValue::Array(_)
                ) && resolved.last_array_parent.is_none()
                {
                    let _ = builder.set_array_form(resolved.target, ArrayForm::Inline);
                } else if let Some((array_parent, indexed)) = resolved.last_array_parent {
                    let form = if indexed {
                        ArrayForm::PerElementIndexed(Form::Inline)
                    } else {
                        ArrayForm::PerElement(Form::Inline)
                    };
                    if reachable.contains(&array_parent) {
                        let _ = builder.set_array_form(array_parent, form);
                    }
                } else if reachable.contains(node) {
                    let _ = builder.set_form(*node, Form::Inline);
                }
            }
            BindSource::Block(inner_id) => {
                let inner = old_source_doc.source(*inner_id);
                let form = if inner.value.is_some() {
                    Form::BindingValueBlock
                } else {
                    Form::BindingBlock
                };
                if let Some((array_parent, indexed)) = resolved.last_array_parent {
                    let array_form = if indexed {
                        ArrayForm::PerElementIndexed(form)
                    } else {
                        ArrayForm::PerElement(form)
                    };
                    if reachable.contains(&array_parent) {
                        let _ = builder.set_array_form(array_parent, array_form);
                    }
                } else if reachable.contains(&resolved.target) {
                    let _ = builder.set_form(resolved.target, form);
                }
                seed_container_plan(
                    old_source_doc,
                    *inner_id,
                    resolved.target,
                    builder,
                    reachable,
                );
            }
        }
    }

    for section in &source.sections {
        let Some(resolved) = resolve_source_path(
            old_source_doc.document(),
            base_node,
            &section.path,
            &mut resolver,
        ) else {
            continue;
        };
        let form = match &section.body {
            SectionBody::Items { value, bindings: _ } => {
                if value.is_some() {
                    Form::SectionValueBlock
                } else {
                    Form::Section
                }
            }
            SectionBody::Block(_) => Form::SectionBlock,
        };
        if let Some((array_parent, indexed)) = resolved.last_array_parent {
            let array_form = if indexed {
                ArrayForm::PerElementIndexed(form)
            } else {
                ArrayForm::PerElement(form)
            };
            if reachable.contains(&array_parent) {
                let _ = builder.set_array_form(array_parent, array_form);
            }
        } else if reachable.contains(&resolved.target) {
            let _ = builder.set_form(resolved.target, form);
        }

        match &section.body {
            SectionBody::Items { bindings, .. } => {
                seed_section_items_plan(
                    old_source_doc,
                    bindings,
                    resolved.target,
                    builder,
                    reachable,
                );
            }
            SectionBody::Block(inner_id) => {
                seed_container_plan(
                    old_source_doc,
                    *inner_id,
                    resolved.target,
                    builder,
                    reachable,
                );
            }
        }
    }
}

fn seed_section_items_plan(
    old_source_doc: &SourceDocument,
    bindings: &[BindingSource],
    base_node: NodeId,
    builder: &mut PlanBuilder,
    reachable: &HashSet<NodeId>,
) {
    let temp_source = EureSource {
        bindings: bindings.to_vec(),
        ..Default::default()
    };
    let temp_doc = SourceDocument {
        document: old_source_doc.document.clone(),
        sources: vec![temp_source],
        root: eure_document::source::SourceId(0),
        multiline_arrays: old_source_doc.multiline_arrays.clone(),
    };
    seed_container_plan(&temp_doc, temp_doc.root, base_node, builder, reachable);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum StatementKind {
    BindingInline,
    BindingBlock,
    SectionItems,
    SectionBlock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct StatementKey {
    target: NodeId,
    kind: StatementKind,
}

#[derive(Default)]
struct SourceResolveState {
    next_push_index: HashMap<NodeId, usize>,
    last_push: HashMap<NodeId, NodeId>,
}

struct ResolvedPath {
    target: NodeId,
    last_array_parent: Option<(NodeId, bool)>,
}

fn resolve_source_path(
    document: &EureDocument,
    base: NodeId,
    path: &SourcePath,
    state: &mut SourceResolveState,
) -> Option<ResolvedPath> {
    let mut current = base;
    let mut last_array_parent = None;
    for segment in path {
        let key_segment = source_key_to_path_segment(&segment.key)?;
        current = traverse::child_node_id(document, current, &key_segment)?;
        if let Some(array) = segment.array {
            let array_id = current;
            let element = match array {
                ArrayIndexKind::Push => {
                    let next = state.next_push_index.entry(array_id).or_default();
                    let element = document.node(array_id).as_array()?.get(*next)?;
                    *next += 1;
                    state.last_push.insert(array_id, element);
                    element
                }
                ArrayIndexKind::Current => *state.last_push.get(&array_id)?,
                ArrayIndexKind::Specific(index) => {
                    let element = document.node(array_id).as_array()?.get(index)?;
                    state.last_push.insert(array_id, element);
                    element
                }
            };
            last_array_parent = Some((array_id, matches!(array, ArrayIndexKind::Specific(_))));
            current = element;
        }
    }
    Some(ResolvedPath {
        target: current,
        last_array_parent,
    })
}

fn source_key_to_path_segment(key: &SourceKey) -> Option<PathSegment> {
    match key {
        SourceKey::Ident(ident) => Some(PathSegment::Ident(ident.clone())),
        SourceKey::Extension(ident) => Some(PathSegment::Extension(ident.clone())),
        SourceKey::Hole(label) => Some(PathSegment::HoleKey(label.clone())),
        SourceKey::String(value, _) => Some(PathSegment::Value(ObjectKey::String(value.clone()))),
        SourceKey::Integer(value) => {
            Some(PathSegment::Value(ObjectKey::Number(BigInt::from(*value))))
        }
        SourceKey::Tuple(values) => Some(PathSegment::Value(ObjectKey::Tuple(Tuple(
            values
                .iter()
                .map(source_key_to_object_key)
                .collect::<Option<Vec<_>>>()?,
        )))),
        SourceKey::TupleIndex(index) => Some(PathSegment::TupleIndex(*index)),
    }
}

fn source_key_to_object_key(key: &SourceKey) -> Option<ObjectKey> {
    match key {
        SourceKey::Ident(ident) => Some(ObjectKey::String(ident.to_string())),
        SourceKey::String(value, _) => Some(ObjectKey::String(value.clone())),
        SourceKey::Integer(value) => Some(ObjectKey::Number(BigInt::from(*value))),
        SourceKey::Tuple(values) => Some(ObjectKey::Tuple(Tuple(
            values
                .iter()
                .map(source_key_to_object_key)
                .collect::<Option<Vec<_>>>()?,
        ))),
        SourceKey::Extension(_) | SourceKey::Hole(_) | SourceKey::TupleIndex(_) => None,
    }
}

fn merge_source_metadata(old_source: &SourceDocument, new_source: &mut SourceDocument) {
    merge_container_metadata(
        old_source,
        old_source.root,
        old_source.document.get_root_id(),
        new_source,
        new_source.root,
        new_source.document.get_root_id(),
    );
}

fn merge_container_metadata(
    old_source_doc: &SourceDocument,
    old_source_id: eure_document::source::SourceId,
    old_base: NodeId,
    new_source_doc: &mut SourceDocument,
    new_source_id: eure_document::source::SourceId,
    new_base: NodeId,
) {
    let old_source = old_source_doc.source(old_source_id);
    let new_source_snapshot = new_source_doc.source(new_source_id).clone();

    let old_statements = collect_statement_infos(old_source_doc, old_source, old_base);
    let new_statements = collect_statement_infos(new_source_doc, &new_source_snapshot, new_base);

    let new_keys: HashSet<StatementKey> = new_statements.iter().map(|info| info.key).collect();
    let old_by_key: HashMap<StatementKey, &StatementInfo> =
        old_statements.iter().map(|info| (info.key, info)).collect();

    let mut forwarded: HashMap<StatementKey, Vec<Trivia>> = HashMap::new();
    let mut overflow = Vec::new();
    for (index, old_info) in old_statements.iter().enumerate() {
        if new_keys.contains(&old_info.key) {
            continue;
        }
        let mut moved = old_info.trivia_before.clone();
        if let Some(comment) = old_info.trailing_comment.clone() {
            moved.push(Trivia::Comment(comment));
        }
        if moved.is_empty() {
            continue;
        }
        if let Some(next_key) = old_statements[index + 1..]
            .iter()
            .map(|info| info.key)
            .find(|key| new_keys.contains(key))
        {
            forwarded.entry(next_key).or_default().extend(moved);
        } else {
            overflow.extend(moved);
        }
    }

    {
        let new_source = new_source_doc.source_mut(new_source_id);
        new_source.leading_trivia = old_source.leading_trivia.clone();
        new_source.trailing_trivia = overflow;
        new_source
            .trailing_trivia
            .extend(old_source.trailing_trivia.clone());
    }

    for new_info in new_statements {
        let extras = forwarded.remove(&new_info.key).unwrap_or_default();
        if let Some(old_info) = old_by_key.get(&new_info.key).copied() {
            match new_info.kind {
                StatementCollectionKind::Binding => {
                    let binding =
                        &mut new_source_doc.source_mut(new_source_id).bindings[new_info.index];
                    binding.path = old_info.path.clone();
                    binding.trailing_comment = old_info.trailing_comment.clone();
                    binding.trivia_before = extras;
                    binding.trivia_before.extend(old_info.trivia_before.clone());
                }
                StatementCollectionKind::Section => {
                    let section =
                        &mut new_source_doc.source_mut(new_source_id).sections[new_info.index];
                    section.path = old_info.path.clone();
                    section.trailing_comment = old_info.trailing_comment.clone();
                    section.trivia_before = extras;
                    section.trivia_before.extend(old_info.trivia_before.clone());
                }
            }

            if let (Some(old_nested), Some(new_nested)) =
                (old_info.nested.clone(), new_info.nested.clone())
            {
                merge_container_metadata(
                    old_source_doc,
                    old_nested.source_id,
                    old_nested.base,
                    new_source_doc,
                    new_nested.source_id,
                    new_nested.base,
                );
            }

            if let (Some(old_items), Some(new_items)) = (
                old_info.section_items.as_ref(),
                new_info.section_items.as_ref(),
            ) {
                merge_section_items_metadata(
                    old_source_doc,
                    old_items,
                    new_source_doc,
                    new_source_id,
                    new_info.index,
                    new_items,
                );
            }
        }
    }
}

#[derive(Clone)]
struct NestedSourceInfo {
    source_id: eure_document::source::SourceId,
    base: NodeId,
}

#[derive(Clone)]
struct StatementInfo {
    key: StatementKey,
    kind: StatementCollectionKind,
    index: usize,
    path: SourcePath,
    trivia_before: Vec<Trivia>,
    trailing_comment: Option<Comment>,
    nested: Option<NestedSourceInfo>,
    section_items: Option<Vec<StatementInfo>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StatementCollectionKind {
    Binding,
    Section,
}

fn collect_statement_infos(
    source_doc: &SourceDocument,
    source: &EureSource,
    base: NodeId,
) -> Vec<StatementInfo> {
    let mut resolver = SourceResolveState::default();
    let mut out = Vec::new();

    for (index, binding) in source.bindings.iter().enumerate() {
        let Some(resolved) =
            resolve_source_path(source_doc.document(), base, &binding.path, &mut resolver)
        else {
            continue;
        };
        let (key_kind, nested) = match &binding.bind {
            BindSource::Value(_) | BindSource::Array { .. } => (StatementKind::BindingInline, None),
            BindSource::Block(source_id) => (
                StatementKind::BindingBlock,
                Some(NestedSourceInfo {
                    source_id: *source_id,
                    base: resolved.target,
                }),
            ),
        };
        out.push(StatementInfo {
            key: StatementKey {
                target: resolved.target,
                kind: key_kind,
            },
            kind: StatementCollectionKind::Binding,
            index,
            path: binding.path.clone(),
            trivia_before: binding.trivia_before.clone(),
            trailing_comment: binding.trailing_comment.clone(),
            nested,
            section_items: None,
        });
    }

    for (index, section) in source.sections.iter().enumerate() {
        let Some(resolved) =
            resolve_source_path(source_doc.document(), base, &section.path, &mut resolver)
        else {
            continue;
        };
        let (statement_kind, nested, section_items) = match &section.body {
            SectionBody::Items { bindings, .. } => (
                StatementKind::SectionItems,
                None,
                Some(collect_section_item_infos(
                    source_doc,
                    bindings,
                    resolved.target,
                )),
            ),
            SectionBody::Block(source_id) => (
                StatementKind::SectionBlock,
                Some(NestedSourceInfo {
                    source_id: *source_id,
                    base: resolved.target,
                }),
                None,
            ),
        };
        out.push(StatementInfo {
            key: StatementKey {
                target: resolved.target,
                kind: statement_kind,
            },
            kind: StatementCollectionKind::Section,
            index,
            path: section.path.clone(),
            trivia_before: section.trivia_before.clone(),
            trailing_comment: section.trailing_comment.clone(),
            nested,
            section_items,
        });
    }

    out
}

fn collect_section_item_infos(
    source_doc: &SourceDocument,
    bindings: &[BindingSource],
    base: NodeId,
) -> Vec<StatementInfo> {
    let temp_source = EureSource {
        bindings: bindings.to_vec(),
        ..Default::default()
    };
    collect_statement_infos(source_doc, &temp_source, base)
}

fn merge_section_items_metadata(
    old_source_doc: &SourceDocument,
    old_items: &[StatementInfo],
    new_source_doc: &mut SourceDocument,
    new_source_id: eure_document::source::SourceId,
    new_section_index: usize,
    new_items: &[StatementInfo],
) {
    let new_keys: HashSet<StatementKey> = new_items.iter().map(|info| info.key).collect();
    let old_by_key: HashMap<StatementKey, &StatementInfo> =
        old_items.iter().map(|info| (info.key, info)).collect();
    let mut forwarded: HashMap<StatementKey, Vec<Trivia>> = HashMap::new();
    for (index, old_info) in old_items.iter().enumerate() {
        if new_keys.contains(&old_info.key) {
            continue;
        }
        let mut moved = old_info.trivia_before.clone();
        if let Some(comment) = old_info.trailing_comment.clone() {
            moved.push(Trivia::Comment(comment));
        }
        if moved.is_empty() {
            continue;
        }
        if let Some(next_key) = old_items[index + 1..]
            .iter()
            .map(|info| info.key)
            .find(|key| new_keys.contains(key))
        {
            forwarded.entry(next_key).or_default().extend(moved);
        }
    }

    for new_info in new_items {
        if let Some(old_info) = old_by_key.get(&new_info.key).copied() {
            {
                let binding = match &mut new_source_doc.source_mut(new_source_id).sections
                    [new_section_index]
                    .body
                {
                    SectionBody::Items { bindings, .. } => &mut bindings[new_info.index],
                    SectionBody::Block(_) => return,
                };
                binding.path = old_info.path.clone();
                binding.trailing_comment = old_info.trailing_comment.clone();
                binding.trivia_before = forwarded.remove(&new_info.key).unwrap_or_default();
                binding.trivia_before.extend(old_info.trivia_before.clone());
            }

            if let (Some(old_nested), Some(new_nested)) =
                (old_info.nested.clone(), new_info.nested.clone())
            {
                merge_container_metadata(
                    old_source_doc,
                    old_nested.source_id,
                    old_nested.base,
                    new_source_doc,
                    new_nested.source_id,
                    new_nested.base,
                );
            }
        }
    }
}
