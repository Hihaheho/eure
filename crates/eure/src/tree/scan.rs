//! Tolerant, untyped helpers for inspecting a (possibly partial) CST.
//!
//! The typed visitor API (`CstVisitor`, `*View` structs) stops at the first
//! node whose children do not match the grammar. Editor features such as
//! completion run on documents that are being typed, where the CST is almost
//! always partial, so they need to look at whatever children *are* present.
//! The functions in this module never fail: a missing child is simply `None`.

use eure_document::identifier::Identifier;
use eure_document::parse::variant_path::VariantPath;
use eure_document::path::{ArrayIndexKind, PathSegment};
use eure_document::text::Text;
use eure_document::value::ObjectKey;
use eure_tree::prelude::*;
use eure_tree::tree::CstNodeData;
use num_bigint::BigInt;

/// Non-terminal kind of `node`, if it is a non-terminal.
pub fn non_terminal_kind(cst: &Cst, node: CstNodeId) -> Option<NonTerminalKind> {
    match cst.node_data(node)? {
        CstNodeData::NonTerminal { kind, .. } => Some(kind),
        CstNodeData::Terminal { .. } => None,
    }
}

/// Terminal kind and input span of `node`, if it is a terminal backed by input.
pub fn terminal(cst: &Cst, node: CstNodeId) -> Option<(TerminalKind, InputSpan)> {
    match cst.node_data(node)? {
        CstNodeData::Terminal {
            kind,
            data: TerminalData::Input(span),
        } => Some((kind, span)),
        _ => None,
    }
}

/// First direct child of `parent` with the given non-terminal kind.
pub fn child_of_kind(cst: &Cst, parent: CstNodeId, kind: NonTerminalKind) -> Option<CstNodeId> {
    cst.children(parent)
        .find(|&child| non_terminal_kind(cst, child) == Some(kind))
}

/// First direct child of `parent` that is a non-terminal of any of `kinds`.
pub fn child_of_kinds(
    cst: &Cst,
    parent: CstNodeId,
    kinds: &[NonTerminalKind],
) -> Option<(CstNodeId, NonTerminalKind)> {
    cst.children(parent).find_map(|child| {
        let kind = non_terminal_kind(cst, child)?;
        kinds.contains(&kind).then_some((child, kind))
    })
}

/// Span of the first terminal found by depth-first search below `node`
/// (including `node` itself) with the given kind.
///
/// Punctuation terminals are wrapped in a non-terminal of the same name
/// (`Dot` → `. (Dot)`), so this is the usual way to locate them.
pub fn find_terminal(cst: &Cst, node: CstNodeId, kind: TerminalKind) -> Option<InputSpan> {
    if let Some((child_kind, span)) = terminal(cst, node) {
        return (child_kind == kind).then_some(span);
    }
    cst.children(node)
        .find_map(|child| find_terminal(cst, child, kind))
}

/// Collect all nodes of `kind` found while unrolling a right-recursive list
/// non-terminal (`EureList`, `KeysList`, `FlatBodyList`, ...).
///
/// The list node itself has one item child plus an optional nested list of
/// the same kind; the result is in source order.
pub fn list_items(
    cst: &Cst,
    list: Option<CstNodeId>,
    item_kind: NonTerminalKind,
) -> Vec<CstNodeId> {
    let mut items = Vec::new();
    let mut current = list;
    while let Some(node) = current {
        let list_kind = non_terminal_kind(cst, node);
        let mut next = None;
        for child in cst.children(node) {
            let child_kind = non_terminal_kind(cst, child);
            if child_kind == Some(item_kind) {
                items.push(child);
            } else if child_kind.is_some() && child_kind == list_kind {
                next = Some(child);
            }
        }
        current = next;
    }
    items
}

/// Text of a span.
pub fn text(input: &str, span: InputSpan) -> &str {
    let start = (span.start as usize).min(input.len());
    let end = (span.end as usize).min(input.len()).max(start);
    &input[start..end]
}

// =============================================================================
// Keys
// =============================================================================

/// One key of a `Keys` production.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeySegment {
    /// Path segment, or `None` when the key form is not supported for
    /// navigation (tuple keys, split float keys, unparsable identifiers).
    pub segment: Option<PathSegment>,
    /// Trimmed span of the key in the input.
    pub span: InputSpan,
    /// Whether the key is a name (identifier, extension, string) rather than
    /// an array marker or index; only names can be partially typed.
    pub is_name: bool,
}

/// Result of scanning a `Keys` node.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedKeys {
    pub segments: Vec<KeySegment>,
    /// Span of a dangling `.` after the last key (`a.b.` while typing).
    pub trailing_dot: Option<InputSpan>,
}

impl ParsedKeys {
    /// All segments as path segments, or `None` if any key is unsupported.
    pub fn path(&self) -> Option<Vec<PathSegment>> {
        self.segments.iter().map(|k| k.segment.clone()).collect()
    }

    /// Span covering the first key through the last key or trailing dot.
    pub fn span(&self) -> Option<InputSpan> {
        let first = self.segments.first().map(|k| k.span)?;
        let last = self
            .trailing_dot
            .or_else(|| self.segments.last().map(|k| k.span))?;
        Some(InputSpan {
            start: first.start,
            end: last.end,
        })
    }
}

/// Scan a `Keys` node tolerant of missing pieces.
pub fn parse_keys(input: &str, cst: &Cst, keys: CstNodeId) -> ParsedKeys {
    let mut parsed = ParsedKeys::default();

    if let Some(first) = child_of_kind(cst, keys, NonTerminalKind::FirstKey)
        && let Some((node, kind)) = child_of_kinds(
            cst,
            first,
            &[NonTerminalKind::Key, NonTerminalKind::ArrayMarker],
        )
    {
        parsed.segments.push(key_segment(input, cst, node, kind));
    }

    let tails = list_items(
        cst,
        child_of_kind(cst, keys, NonTerminalKind::KeysList),
        NonTerminalKind::KeyTail,
    );
    for tail in tails {
        let Some((node, kind)) = child_of_kinds(
            cst,
            tail,
            &[NonTerminalKind::DotKey, NonTerminalKind::ArrayMarker],
        ) else {
            continue;
        };
        match kind {
            NonTerminalKind::DotKey => {
                // Error recovery may leave a `Key` node holding only the
                // whitespace that followed the dot; that is a dangling dot too.
                let key = child_of_kind(cst, node, NonTerminalKind::Key)
                    .map(|key| key_segment(input, cst, key, NonTerminalKind::Key))
                    .filter(|key| {
                        key.segment.is_some() || !text(input, key.span).trim().is_empty()
                    });
                match key {
                    Some(key) => parsed.segments.push(key),
                    None => {
                        if let Some(dot) = find_terminal(cst, node, TerminalKind::Dot) {
                            parsed.trailing_dot = Some(dot);
                        }
                    }
                }
            }
            _ => parsed.segments.push(key_segment(input, cst, node, kind)),
        }
    }

    parsed
}

fn key_segment(input: &str, cst: &Cst, node: CstNodeId, kind: NonTerminalKind) -> KeySegment {
    let span = cst.span(node).unwrap_or(InputSpan::EMPTY);
    if kind == NonTerminalKind::ArrayMarker {
        let index = child_of_kind(cst, node, NonTerminalKind::ArrayMarkerOpt)
            .and_then(|opt| child_of_kind(cst, opt, NonTerminalKind::ArrayMarkerOptGroup))
            .map(|group| {
                if find_terminal(cst, group, TerminalKind::Circumflex).is_some() {
                    ArrayIndexKind::Current
                } else {
                    find_terminal(cst, group, TerminalKind::Integer)
                        .and_then(|span| text(input, span).parse::<usize>().ok())
                        .map(ArrayIndexKind::Specific)
                        .unwrap_or(ArrayIndexKind::Push)
                }
            })
            .unwrap_or(ArrayIndexKind::Push);
        return KeySegment {
            segment: Some(PathSegment::ArrayIndex(index)),
            span,
            is_name: false,
        };
    }

    let Some((child, child_kind)) = child_of_kinds(
        cst,
        node,
        &[
            NonTerminalKind::KeyIdent,
            NonTerminalKind::ExtensionNameSpace,
            NonTerminalKind::String,
            NonTerminalKind::Integer,
            NonTerminalKind::Float,
            NonTerminalKind::KeyTuple,
            NonTerminalKind::TupleIndex,
            NonTerminalKind::Hole,
        ],
    ) else {
        return KeySegment {
            segment: None,
            span,
            is_name: true,
        };
    };

    let (segment, is_name) = match child_kind {
        NonTerminalKind::KeyIdent => (
            key_ident_text(input, cst, child).map(|name| match name.parse::<Identifier>() {
                Ok(ident) => PathSegment::Ident(ident),
                Err(_) => PathSegment::Value(ObjectKey::String(name.to_string())),
            }),
            true,
        ),
        NonTerminalKind::ExtensionNameSpace => (
            child_of_kind(cst, child, NonTerminalKind::KeyIdent)
                .and_then(|ident| key_ident_text(input, cst, ident))
                .and_then(|name| name.parse::<Identifier>().ok())
                .map(PathSegment::Extension),
            true,
        ),
        NonTerminalKind::String => (
            string_content(input, cst, child).map(|s| PathSegment::Value(ObjectKey::String(s))),
            true,
        ),
        NonTerminalKind::Integer => (
            text(input, span)
                .replace('_', "")
                .parse::<BigInt>()
                .ok()
                .map(|n| PathSegment::Value(ObjectKey::Number(n))),
            false,
        ),
        NonTerminalKind::TupleIndex => (
            find_terminal(cst, child, TerminalKind::Integer)
                .and_then(|s| text(input, s).parse::<u8>().ok())
                .map(PathSegment::TupleIndex),
            false,
        ),
        NonTerminalKind::Hole => (
            find_terminal(cst, child, TerminalKind::Hole).map(|s| {
                let label = text(input, s).strip_prefix('!').unwrap_or("");
                PathSegment::HoleKey(label.parse::<Identifier>().ok())
            }),
            false,
        ),
        _ => (None, false),
    };

    KeySegment {
        segment,
        span,
        is_name,
    }
}

fn key_ident_text<'a>(input: &'a str, cst: &Cst, key_ident: CstNodeId) -> Option<&'a str> {
    let span = cst.span(key_ident)?;
    Some(text(input, span))
}

// =============================================================================
// Values
// =============================================================================

/// Content of a `String` node (`"..."`, `'...'`, or delimited literal strings).
pub fn string_content(input: &str, cst: &Cst, string: CstNodeId) -> Option<String> {
    let (node, kind) = child_of_kinds(
        cst,
        string,
        &[
            NonTerminalKind::Str,
            NonTerminalKind::LitStr,
            NonTerminalKind::LitStr1,
            NonTerminalKind::LitStr2,
            NonTerminalKind::LitStr3,
        ],
    )?;
    match kind {
        NonTerminalKind::Str => {
            let raw = text(input, find_terminal(cst, node, TerminalKind::Str)?);
            let inner = raw.strip_prefix('"')?.strip_suffix('"')?;
            Text::parse_quoted_string(inner).ok().map(|t| t.content)
        }
        NonTerminalKind::LitStr => {
            let raw = text(input, find_terminal(cst, node, TerminalKind::LitStr)?);
            Some(raw.strip_prefix('\'')?.strip_suffix('\'')?.to_string())
        }
        _ => {
            let mut content = String::new();
            collect_terminal_text(
                input,
                cst,
                node,
                &[TerminalKind::NoSQuote, TerminalKind::SQuote],
                &mut content,
            );
            Some(content)
        }
    }
}

fn collect_terminal_text(
    input: &str,
    cst: &Cst,
    node: CstNodeId,
    kinds: &[TerminalKind],
    out: &mut String,
) {
    if let Some((kind, span)) = terminal(cst, node) {
        if kinds.contains(&kind) {
            out.push_str(text(input, span));
        }
        return;
    }
    for child in cst.children(node) {
        collect_terminal_text(input, cst, child, kinds, out);
    }
}

/// String content of a `Value` node when it is a plain string literal.
pub fn value_string(input: &str, cst: &Cst, value: CstNodeId) -> Option<String> {
    let strings = child_of_kind(cst, value, NonTerminalKind::Strings)?;
    let string = child_of_kind(cst, strings, NonTerminalKind::String)?;
    string_content(input, cst, string)
}

/// Text content of a `TextBinding` (`key: text`), trimmed.
pub fn text_binding_content(input: &str, cst: &Cst, text_binding: CstNodeId) -> Option<String> {
    let span = find_terminal(cst, text_binding, TerminalKind::Text)?;
    Some(text(input, span).trim().to_string())
}

/// Variant selected by a binding of the form `$variant = "..."` or `$variant: ...`.
pub fn variant_binding(input: &str, cst: &Cst, binding: CstNodeId) -> Option<VariantPath> {
    let keys = child_of_kind(cst, binding, NonTerminalKind::Keys)?;
    let parsed = parse_keys(input, cst, keys);
    let [only] = parsed.segments.as_slice() else {
        return None;
    };
    match &only.segment {
        Some(PathSegment::Extension(ext)) if ext.as_ref() == "variant" => {}
        _ => return None,
    }
    let rhs = child_of_kind(cst, binding, NonTerminalKind::BindingRhs)?;
    let (node, kind) = child_of_kinds(
        cst,
        rhs,
        &[NonTerminalKind::ValueBinding, NonTerminalKind::TextBinding],
    )?;
    let raw = match kind {
        NonTerminalKind::ValueBinding => {
            let value = child_of_kind(cst, node, NonTerminalKind::Value)?;
            value_string(input, cst, value)?
        }
        _ => text_binding_content(input, cst, node)?,
    };
    if raw.is_empty() {
        return None;
    }
    VariantPath::parse(&raw).ok()
}

/// Variant selected by an object entry `$variant => "..."`, given the entry's
/// `Keys` and `Value` nodes.
pub fn variant_entry(
    input: &str,
    cst: &Cst,
    keys: CstNodeId,
    value: Option<CstNodeId>,
) -> Option<VariantPath> {
    let parsed = parse_keys(input, cst, keys);
    let [only] = parsed.segments.as_slice() else {
        return None;
    };
    match &only.segment {
        Some(PathSegment::Extension(ext)) if ext.as_ref() == "variant" => {}
        _ => return None,
    }
    let raw = value_string(input, cst, value?)?;
    if raw.is_empty() {
        return None;
    }
    VariantPath::parse(&raw).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> Cst {
        eure_parol::parse_tolerant(input, "<input>").cst()
    }

    fn first_keys(cst: &Cst) -> CstNodeId {
        fn find(cst: &Cst, node: CstNodeId) -> Option<CstNodeId> {
            if non_terminal_kind(cst, node) == Some(NonTerminalKind::Keys) {
                return Some(node);
            }
            cst.children(node).find_map(|child| find(cst, child))
        }
        find(cst, cst.root()).expect("keys")
    }

    #[test]
    fn parse_keys_handles_trailing_dot() {
        let input = "@ myfield.";
        let cst = parse(input);
        let parsed = parse_keys(input, &cst, first_keys(&cst));
        assert_eq!(parsed.segments.len(), 1);
        assert_eq!(
            parsed.segments[0].segment,
            Some(PathSegment::Ident("myfield".parse().unwrap()))
        );
        assert_eq!(parsed.trailing_dot, Some(InputSpan { start: 9, end: 10 }));
    }

    #[test]
    fn parse_keys_handles_array_marker_and_extension() {
        let input = "items[].$variant = \"a\"";
        let cst = parse(input);
        let parsed = parse_keys(input, &cst, first_keys(&cst));
        assert_eq!(
            parsed.path(),
            Some(vec![
                PathSegment::Ident("items".parse().unwrap()),
                PathSegment::ArrayIndex(ArrayIndexKind::Push),
                PathSegment::Extension("variant".parse().unwrap()),
            ])
        );
    }

    #[test]
    fn variant_binding_reads_text_and_value_forms() {
        for input in ["$variant: set-text", "$variant = \"set-text\""] {
            let cst = parse(input);
            let binding = child_of_kind(&cst, cst.root(), NonTerminalKind::Eure)
                .and_then(|eure| child_of_kind(&cst, eure, NonTerminalKind::EureList))
                .and_then(|list| child_of_kind(&cst, list, NonTerminalKind::Binding))
                .expect("binding");
            let variant = variant_binding(input, &cst, binding).expect("variant");
            assert_eq!(variant, VariantPath::parse("set-text").unwrap());
        }
    }
}
