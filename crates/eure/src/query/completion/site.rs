//! Locate the cursor site: what the cursor is positioned on, and the
//! document path that position belongs to.
//!
//! The walk runs on the tolerant CST (`ParseCst`), so it works while the
//! document is being typed and does not parse. It never consults the schema;
//! everything it reports is purely syntactic.
//!
//! Completion reads [`CompletionSite::kind`] (what may be inserted here);
//! hover reads [`CompletionSite::anchor`] (the existing key or value the
//! cursor is on, if any).

use eure_document::path::{ArrayIndexKind, EurePath, PathSegment};
use eure_document::value::ObjectKey;
use eure_schema::navigate::VariantHint;
use eure_tree::prelude::*;

use crate::tree::scan::{
    ParsedKeys, child_of_kind, child_of_kinds, find_terminal, list_items, non_terminal_kind,
    parse_keys, text, variant_binding, variant_entry,
};

/// Syntactic description of the cursor position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionSite {
    pub kind: SiteKind,
    /// Text already typed for the item being completed.
    pub partial: String,
    /// Span that a completion replaces (covers `partial`).
    pub replace: InputSpan,
    /// `$variant` selections visible from the cursor, one per enclosing scope
    /// that declares one.
    pub hints: Vec<VariantHint>,
    /// The existing key or value the cursor is on. `None` at free positions
    /// (a blank line, after `=` with no value yet, a dangling `.`).
    pub anchor: Option<Anchor>,
}

/// An existing key or value under the cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchor {
    pub kind: AnchorKind,
    /// Full document path of the node the token belongs to: for a key, the
    /// path ending in that key; for a value, the path it is bound to.
    pub path: EurePath,
    /// Span of the whole token (not only the part before the cursor).
    pub span: InputSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorKind {
    Key,
    Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SiteKind {
    /// A key is being typed: a section key, a binding key, or an object key.
    Key {
        /// Path of the container the key is added to.
        parent: EurePath,
        /// Keys already bound in the same container.
        used: Vec<String>,
    },
    /// A value is being typed for the node at `path`.
    Value { path: EurePath, style: ValueStyle },
}

/// How the value is bound; decides which value forms are meaningful.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueStyle {
    /// `key = value` (any value form).
    Bind,
    /// `key: text` (text only, no quotes).
    Text,
}

/// Find the completion site at byte offset `cursor`.
///
/// Returns `None` when the cursor is somewhere completion makes no sense
/// (inside a code block, on an array marker, ...).
pub fn find_site(input: &str, cst: &Cst, cursor: u32) -> Option<CompletionSite> {
    find_site_with_inline_code(input, cst, cursor, false)
}

/// Navigation also recognizes inline type references, which aren't completion sites.
pub fn find_navigation_site(input: &str, cst: &Cst, cursor: u32) -> Option<CompletionSite> {
    find_site_with_inline_code(input, cst, cursor, true)
}

fn find_site_with_inline_code(
    input: &str,
    cst: &Cst,
    cursor: u32,
    inline_code: bool,
) -> Option<CompletionSite> {
    let cursor = cursor.min(input.len() as u32);
    let eure = child_of_kind(cst, cst.root(), NonTerminalKind::Eure)?;
    let mut walker = Walker {
        input,
        cst,
        cursor,
        inline_code,
        scope: Vec::new(),
        hints: Vec::new(),
    };
    let region = InputSpan {
        start: 0,
        end: input.len() as u32,
    };
    walker.walk_eure(eure, region)
}

struct Walker<'a> {
    inline_code: bool,
    input: &'a str,
    cst: &'a Cst,
    cursor: u32,
    scope: Vec<PathSegment>,
    hints: Vec<VariantHint>,
}

/// A binding or section inside a statement list, with the offset where it
/// starts (leading trivia excluded).
struct Statement {
    node: CstNodeId,
    is_section: bool,
    start: u32,
}

impl<'a> Walker<'a> {
    fn in_span(&self, span: InputSpan) -> bool {
        span.start <= self.cursor && self.cursor <= span.end
    }

    fn span(&self, node: CstNodeId) -> Option<InputSpan> {
        self.cst.span(node)
    }

    // -------------------------------------------------------------------------
    // Scopes
    // -------------------------------------------------------------------------

    fn walk_eure(&mut self, eure: CstNodeId, region: InputSpan) -> Option<CompletionSite> {
        // `= value` / `: text` bound to the scope itself.
        if let Some(opt) = child_of_kind(self.cst, eure, NonTerminalKind::EureOpt)
            && let Some(top) = child_of_kind(self.cst, opt, NonTerminalKind::TopLevelBinding)
            && let Some(site) = self.walk_rhs(top, self.scope.clone(), region)
        {
            return Some(site);
        }

        let bindings = list_items(
            self.cst,
            child_of_kind(self.cst, eure, NonTerminalKind::EureList),
            NonTerminalKind::Binding,
        );
        let sections = list_items(
            self.cst,
            child_of_kind(self.cst, eure, NonTerminalKind::EureList0),
            NonTerminalKind::Section,
        );
        self.walk_statements(&bindings, &sections, region)
    }

    fn walk_statements(
        &mut self,
        bindings: &[CstNodeId],
        sections: &[CstNodeId],
        region: InputSpan,
    ) -> Option<CompletionSite> {
        let mut statements: Vec<Statement> = bindings
            .iter()
            .map(|&node| (node, false))
            .chain(sections.iter().map(|&node| (node, true)))
            .filter_map(|(node, is_section)| {
                let start = self.span(node)?.start;
                Some(Statement {
                    node,
                    is_section,
                    start,
                })
            })
            .collect();
        statements.sort_by_key(|s| s.start);

        let hint_count = self.hints.len();
        for &binding in bindings {
            if let Some(variant) = variant_binding(self.input, self.cst, binding) {
                self.hints.push(VariantHint {
                    prefix_len: self.scope.len(),
                    variant,
                });
            }
        }

        let mut result = None;
        for (index, statement) in statements.iter().enumerate() {
            let next_start = statements
                .get(index + 1)
                .map(|s| s.start)
                .unwrap_or(region.end);
            if self.cursor < statement.start {
                break;
            }
            if index + 1 < statements.len() && self.cursor >= next_start {
                continue;
            }
            let extent = InputSpan {
                start: statement.start,
                end: next_start.max(statement.start),
            };
            result = if statement.is_section {
                self.walk_section(statement.node, extent, &statements)
            } else {
                self.walk_binding(statement.node, extent, &statements)
            };
            break;
        }

        let site = result.or_else(|| {
            self.in_span(region).then(|| {
                let used = self.used_keys(&statements, &[]);
                self.key_site(self.scope.clone(), used)
            })
        });

        self.hints.truncate(hint_count);
        site
    }

    fn walk_section(
        &mut self,
        section: CstNodeId,
        extent: InputSpan,
        siblings: &[Statement],
    ) -> Option<CompletionSite> {
        // Error recovery may produce a `Keys` node holding only whitespace;
        // that is the same as no keys at all.
        let keys = child_of_kind(self.cst, section, NonTerminalKind::Keys)
            .map(|keys| parse_keys(self.input, self.cst, keys))
            .filter(|keys| !keys.segments.is_empty() || keys.trailing_dot.is_some());

        let Some(keys) = keys else {
            // `@|` or `@ |`: the section key is missing entirely.
            let used = self.used_keys(siblings, &[]);
            return Some(self.key_site(self.scope.clone(), used));
        };
        if let Some(site) = self.key_site_in_keys(&keys, siblings) {
            return Some(site);
        }
        let segments = keys.path()?;
        let header_end = keys.span().map(|s| s.end).unwrap_or(extent.start);

        let body = child_of_kind(self.cst, section, NonTerminalKind::SectionBody);
        let body_child = body.and_then(|body| {
            child_of_kinds(
                self.cst,
                body,
                &[NonTerminalKind::BlockBody, NonTerminalKind::SectionBodyOpt],
            )
        });

        let depth = self.scope.len();
        self.scope.extend(segments);
        let result = match body_child {
            Some((block, NonTerminalKind::BlockBody)) => self.walk_block(block, extent),
            Some((opt, _)) => {
                let flat = child_of_kind(self.cst, opt, NonTerminalKind::FlatBody);
                self.walk_flat_body(flat, header_end, extent)
            }
            None => self.walk_flat_body(None, header_end, extent),
        };
        self.scope.truncate(depth);
        result
    }

    /// `{ Eure }` body of a section or binding. The scope is already pushed.
    fn walk_block(&mut self, block: CstNodeId, extent: InputSpan) -> Option<CompletionSite> {
        let begin = find_terminal(self.cst, block, TerminalKind::LBrace)?;
        let end = find_terminal(self.cst, block, TerminalKind::RBrace)
            .map(|s| s.start)
            .unwrap_or(extent.end);
        let region = InputSpan {
            start: begin.end,
            end: end.max(begin.end),
        };
        if !self.in_span(region) {
            return None;
        }
        match child_of_kind(self.cst, block, NonTerminalKind::Eure) {
            Some(eure) => self.walk_eure(eure, region),
            None => self.walk_statements(&[], &[], region),
        }
    }

    /// Flat section body (`@ a` followed by bindings). The scope is already pushed.
    fn walk_flat_body(
        &mut self,
        flat: Option<CstNodeId>,
        header_end: u32,
        extent: InputSpan,
    ) -> Option<CompletionSite> {
        let region = InputSpan {
            start: header_end,
            end: extent.end.max(header_end),
        };
        if self.cursor < region.start {
            return None;
        }
        let Some(flat) = flat else {
            return self.walk_statements(&[], &[], region);
        };

        // `@ a = 1`, `@ a: text`, or `@ a\n= 1`: value bound to the section node.
        if let Some(head) = child_of_kind(self.cst, flat, NonTerminalKind::SectionHead) {
            let rhs = child_of_kind(self.cst, head, NonTerminalKind::RootBinding).or_else(|| {
                child_of_kind(self.cst, head, NonTerminalKind::NewlineHead)
                    .and_then(|nl| child_of_kind(self.cst, nl, NonTerminalKind::NewlineHeadOpt))
                    .and_then(|opt| child_of_kind(self.cst, opt, NonTerminalKind::FlatRootBinding))
            });
            if let Some(rhs) = rhs
                && let Some(site) = self.walk_rhs(rhs, self.scope.clone(), region)
            {
                return Some(site);
            }
        }

        let bindings = list_items(
            self.cst,
            child_of_kind(self.cst, flat, NonTerminalKind::FlatBodyList),
            NonTerminalKind::Binding,
        );
        self.walk_statements(&bindings, &[], region)
    }

    // -------------------------------------------------------------------------
    // Bindings
    // -------------------------------------------------------------------------

    fn walk_binding(
        &mut self,
        binding: CstNodeId,
        extent: InputSpan,
        siblings: &[Statement],
    ) -> Option<CompletionSite> {
        let keys = child_of_kind(self.cst, binding, NonTerminalKind::Keys)?;
        let keys = parse_keys(self.input, self.cst, keys);
        if let Some(site) = self.key_site_in_keys(&keys, siblings) {
            return Some(site);
        }
        let segments = keys.path()?;
        let rhs = child_of_kind(self.cst, binding, NonTerminalKind::BindingRhs)?;

        let mut path = self.scope.clone();
        path.extend(segments.iter().cloned());

        if let Some(block) = child_of_kind(self.cst, rhs, NonTerminalKind::SectionBinding) {
            let depth = self.scope.len();
            self.scope.extend(segments);
            let result = self.walk_block(block, extent);
            self.scope.truncate(depth);
            return result;
        }
        self.walk_rhs(rhs, path, extent)
    }

    /// Value side of a binding. `node` is a `ValueBinding`/`TextBinding` or a
    /// wrapper (`BindingRhs`, `TopLevelBinding`, `RootBinding`, ...) around one.
    fn walk_rhs(
        &mut self,
        node: CstNodeId,
        path: Vec<PathSegment>,
        extent: InputSpan,
    ) -> Option<CompletionSite> {
        let (rhs, kind) = match non_terminal_kind(self.cst, node) {
            Some(kind @ (NonTerminalKind::ValueBinding | NonTerminalKind::TextBinding)) => {
                (node, kind)
            }
            Some(NonTerminalKind::RootValueBinding) => (node, NonTerminalKind::ValueBinding),
            Some(NonTerminalKind::RootTextBinding) => (node, NonTerminalKind::TextBinding),
            _ => child_of_kinds(
                self.cst,
                node,
                &[
                    NonTerminalKind::ValueBinding,
                    NonTerminalKind::TextBinding,
                    NonTerminalKind::RootValueBinding,
                    NonTerminalKind::RootTextBinding,
                ],
            )
            .map(|(child, kind)| match kind {
                NonTerminalKind::RootValueBinding => (child, NonTerminalKind::ValueBinding),
                NonTerminalKind::RootTextBinding => (child, NonTerminalKind::TextBinding),
                other => (child, other),
            })?,
        };

        match kind {
            NonTerminalKind::ValueBinding => {
                let bind = find_terminal(self.cst, rhs, TerminalKind::Bind)
                    .or_else(|| find_terminal(self.cst, rhs, TerminalKind::NewlineBind))?;
                if self.cursor < bind.end {
                    return None;
                }
                match child_of_kind(self.cst, rhs, NonTerminalKind::Value) {
                    Some(value) => {
                        let span = self.span(value)?;
                        if self.in_span(span) {
                            self.walk_value(value, path)
                        } else if self.cursor < span.start
                            && self.input[self.cursor as usize..span.start as usize]
                                .chars()
                                .all(char::is_whitespace)
                        {
                            // Between `=` and a value on a later line. Error
                            // recovery also lands here: a missing value is
                            // filled with the next line's first token.
                            Some(self.value_site(path, ValueStyle::Bind))
                        } else {
                            None
                        }
                    }
                    None => {
                        (self.cursor <= extent.end).then(|| self.value_site(path, ValueStyle::Bind))
                    }
                }
            }
            _ => {
                let start = find_terminal(self.cst, rhs, TerminalKind::TextStart)
                    .or_else(|| find_terminal(self.cst, rhs, TerminalKind::NewlineTextStart))?;
                if self.cursor < start.end {
                    return None;
                }
                let line_end = self.input[start.end as usize..]
                    .find('\n')
                    .map(|i| start.end + i as u32)
                    .unwrap_or(self.input.len() as u32);
                if self.cursor > line_end {
                    return None;
                }
                let raw = &self.input[start.end as usize..self.cursor as usize];
                let trimmed = raw.trim_start();
                let replace = InputSpan {
                    start: self.cursor - trimmed.len() as u32,
                    end: self.cursor,
                };
                // The whole text after `:`, trimmed, is the value under the cursor.
                let line = &self.input[start.end as usize..line_end as usize];
                let content = line.trim();
                let anchor = (!content.is_empty()).then(|| {
                    let content_start = start.end + (line.len() - line.trim_start().len()) as u32;
                    Anchor {
                        kind: AnchorKind::Value,
                        path: EurePath(path.clone()),
                        span: InputSpan {
                            start: content_start,
                            end: content_start + content.len() as u32,
                        },
                    }
                });
                Some(CompletionSite {
                    kind: SiteKind::Value {
                        path: EurePath(path),
                        style: ValueStyle::Text,
                    },
                    partial: trimmed.to_string(),
                    replace,
                    hints: self.hints.clone(),
                    anchor,
                })
            }
        }
    }

    // -------------------------------------------------------------------------
    // Values
    // -------------------------------------------------------------------------

    fn walk_value(&mut self, value: CstNodeId, path: Vec<PathSegment>) -> Option<CompletionSite> {
        let Some((child, kind)) = child_of_kinds(
            self.cst,
            value,
            &[
                NonTerminalKind::Object,
                NonTerminalKind::Array,
                NonTerminalKind::Tuple,
                NonTerminalKind::CodeBlock,
                NonTerminalKind::InlineCode,
            ],
        ) else {
            let span = self.span(value)?;
            let partial = text(self.input, span)[..(self.cursor - span.start) as usize].to_string();
            return Some(CompletionSite {
                kind: SiteKind::Value {
                    path: EurePath(path.clone()),
                    style: ValueStyle::Bind,
                },
                partial,
                replace: span,
                hints: self.hints.clone(),
                anchor: Some(Anchor {
                    kind: AnchorKind::Value,
                    path: EurePath(path),
                    span,
                }),
            });
        };
        match kind {
            NonTerminalKind::Object => self.walk_object(child, path),
            NonTerminalKind::Array => self.walk_array(child, path),
            NonTerminalKind::InlineCode if self.inline_code => {
                let span = self.span(value)?;
                Some(CompletionSite {
                    kind: SiteKind::Value {
                        path: EurePath(path.clone()),
                        style: ValueStyle::Bind,
                    },
                    partial: String::new(),
                    replace: span,
                    hints: self.hints.clone(),
                    anchor: Some(Anchor {
                        kind: AnchorKind::Value,
                        path: EurePath(path),
                        span,
                    }),
                })
            }
            _ => None,
        }
    }

    fn walk_object(&mut self, object: CstNodeId, path: Vec<PathSegment>) -> Option<CompletionSite> {
        let begin = find_terminal(self.cst, object, TerminalKind::LBrace)?;
        let end = find_terminal(self.cst, object, TerminalKind::RBrace)
            .map(|s| s.start)
            .unwrap_or(self.input.len() as u32);
        let region = InputSpan {
            start: begin.end,
            end: end.max(begin.end),
        };
        if !self.in_span(region) {
            return None;
        }

        // Entries: `Keys => Value` pairs from the recursive ObjectList.
        let mut entries: Vec<(CstNodeId, Option<CstNodeId>)> = Vec::new();
        let mut list = child_of_kind(self.cst, object, NonTerminalKind::ObjectList);
        while let Some(node) = list {
            let keys = child_of_kind(self.cst, node, NonTerminalKind::Keys);
            let value = child_of_kind(self.cst, node, NonTerminalKind::Value);
            if let Some(keys) = keys {
                entries.push((keys, value));
            }
            list = child_of_kind(self.cst, node, NonTerminalKind::ObjectList);
        }

        let hint_count = self.hints.len();
        for &(keys, value) in &entries {
            if let Some(variant) = variant_entry(self.input, self.cst, keys, value) {
                self.hints.push(VariantHint {
                    prefix_len: path.len(),
                    variant,
                });
            }
        }

        let depth = self.scope.len();
        let scope_backup = std::mem::replace(&mut self.scope, path.clone());
        let mut result = None;

        // Leading `= value` binds the object node itself.
        if let Some(opt) = child_of_kind(self.cst, object, NonTerminalKind::ObjectOpt)
            && let Some(vb) = child_of_kind(self.cst, opt, NonTerminalKind::ValueBinding)
        {
            result = self.walk_rhs(vb, path.clone(), region);
        }

        let statements: Vec<Statement> = entries
            .iter()
            .filter_map(|&(keys, _)| {
                Some(Statement {
                    node: keys,
                    is_section: false,
                    start: self.span(keys)?.start,
                })
            })
            .collect();

        if result.is_none() {
            for &(keys_node, value) in &entries {
                let keys = parse_keys(self.input, self.cst, keys_node);
                if let Some(site) = self.key_site_in_keys(&keys, &statements) {
                    result = Some(site);
                    break;
                }
                if let Some(value) = value
                    && let Some(span) = self.span(value)
                    && self.in_span(span)
                    && let Some(segments) = keys.path()
                {
                    let mut child_path = path.clone();
                    child_path.extend(segments);
                    result = self.walk_value(value, child_path);
                    break;
                }
            }
        }

        let site = result.or_else(|| {
            let used = self.used_keys(&statements, &[]);
            Some(self.key_site(path, used))
        });

        self.scope = scope_backup;
        self.scope.truncate(depth);
        self.hints.truncate(hint_count);
        site
    }

    fn walk_array(&mut self, array: CstNodeId, path: Vec<PathSegment>) -> Option<CompletionSite> {
        let begin = find_terminal(self.cst, array, TerminalKind::LBracket)?;
        let end = find_terminal(self.cst, array, TerminalKind::RBracket)
            .map(|s| s.start)
            .unwrap_or(self.input.len() as u32);
        let region = InputSpan {
            start: begin.end,
            end: end.max(begin.end),
        };
        if !self.in_span(region) {
            return None;
        }

        let mut element_path = path;
        element_path.push(PathSegment::ArrayIndex(ArrayIndexKind::Push));

        let mut elements = Vec::new();
        collect_array_elements(self.cst, array, &mut elements);
        for element in elements {
            if let Some(span) = self.span(element)
                && self.in_span(span)
            {
                return self.walk_value(element, element_path);
            }
        }
        Some(self.value_site(element_path, ValueStyle::Bind))
    }

    // -------------------------------------------------------------------------
    // Site construction
    // -------------------------------------------------------------------------

    /// Key site when the cursor is on one of `keys` (or on its trailing dot).
    fn key_site_in_keys(
        &self,
        keys: &ParsedKeys,
        siblings: &[Statement],
    ) -> Option<CompletionSite> {
        for (index, key) in keys.segments.iter().enumerate() {
            if !self.in_span(key.span) {
                continue;
            }
            if !key.is_name {
                return None;
            }
            let prefix: Vec<PathSegment> = keys.segments[..index]
                .iter()
                .map(|k| k.segment.clone())
                .collect::<Option<_>>()?;
            let mut parent = self.scope.clone();
            parent.extend(prefix.iter().cloned());
            let partial =
                text(self.input, key.span)[..(self.cursor - key.span.start) as usize].to_string();
            let anchor = key.segment.clone().map(|segment| {
                let mut path = parent.clone();
                path.push(segment);
                Anchor {
                    kind: AnchorKind::Key,
                    path: EurePath(path),
                    span: key.span,
                }
            });
            return Some(CompletionSite {
                kind: SiteKind::Key {
                    parent: EurePath(parent),
                    used: self.used_keys(siblings, &prefix),
                },
                partial,
                replace: key.span,
                hints: self.hints.clone(),
                anchor,
            });
        }
        if let Some(dot) = keys.trailing_dot
            && self.cursor >= dot.end
            && self.input[dot.end as usize..self.cursor as usize]
                .chars()
                .all(char::is_whitespace)
        {
            let prefix = keys.path()?;
            let mut parent = self.scope.clone();
            parent.extend(prefix.iter().cloned());
            return Some(CompletionSite {
                kind: SiteKind::Key {
                    parent: EurePath(parent),
                    used: self.used_keys(siblings, &prefix),
                },
                partial: String::new(),
                replace: InputSpan {
                    start: self.cursor,
                    end: self.cursor,
                },
                hints: self.hints.clone(),
                anchor: None,
            });
        }
        None
    }

    /// Key site for a free position (blank line, after `@`, inside `{ }`).
    fn key_site(&self, parent: Vec<PathSegment>, used: Vec<String>) -> CompletionSite {
        let (partial, replace) = word_before(self.input, self.cursor, is_key_char);
        CompletionSite {
            kind: SiteKind::Key {
                parent: EurePath(parent),
                used,
            },
            partial,
            replace,
            hints: self.hints.clone(),
            anchor: None,
        }
    }

    fn value_site(&self, path: Vec<PathSegment>, style: ValueStyle) -> CompletionSite {
        let (partial, replace) = word_before(self.input, self.cursor, is_value_char);
        CompletionSite {
            kind: SiteKind::Value {
                path: EurePath(path),
                style,
            },
            partial,
            replace,
            hints: self.hints.clone(),
            anchor: None,
        }
    }

    /// Names bound by sibling bindings directly under `scope + prefix`.
    fn used_keys(&self, siblings: &[Statement], prefix: &[PathSegment]) -> Vec<String> {
        let mut used = Vec::new();
        for statement in siblings.iter().filter(|s| !s.is_section) {
            // The statement being typed does not count as bound.
            if self
                .span(statement.node)
                .is_some_and(|span| self.in_span(span))
            {
                continue;
            }
            let keys = match non_terminal_kind(self.cst, statement.node) {
                Some(NonTerminalKind::Keys) => Some(statement.node),
                _ => child_of_kind(self.cst, statement.node, NonTerminalKind::Keys),
            };
            let Some(keys) = keys else {
                continue;
            };
            let Some(path) = parse_keys(self.input, self.cst, keys).path() else {
                continue;
            };
            if path.len() != prefix.len() + 1 || path[..prefix.len()] != *prefix {
                continue;
            }
            let name = match &path[prefix.len()] {
                PathSegment::Ident(ident) => ident.to_string(),
                PathSegment::Value(ObjectKey::String(name)) => name.clone(),
                _ => continue,
            };
            if !used.contains(&name) {
                used.push(name);
            }
        }
        used
    }
}

/// Collect the `Value` nodes that are direct array elements (not values nested
/// inside those elements).
fn collect_array_elements(cst: &Cst, node: CstNodeId, out: &mut Vec<CstNodeId>) {
    for child in cst.children(node) {
        match non_terminal_kind(cst, child) {
            Some(NonTerminalKind::Value) => out.push(child),
            Some(_) => collect_array_elements(cst, child, out),
            None => {}
        }
    }
}

fn is_key_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '$')
}

fn is_value_char(c: char) -> bool {
    is_key_char(c) || matches!(c, '"' | '\'' | '.' | '+')
}

/// The word immediately before `cursor` and the span it occupies.
fn word_before(input: &str, cursor: u32, is_word_char: fn(char) -> bool) -> (String, InputSpan) {
    let cursor = (cursor as usize).min(input.len());
    let before = &input[..cursor];
    let start = before
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_word_char(*c))
        .last()
        .map(|(i, _)| i)
        .unwrap_or(cursor);
    (
        before[start..].to_string(),
        InputSpan {
            start: start as u32,
            end: cursor as u32,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(source: &str) -> Option<CompletionSite> {
        let cursor = source.find("|_|").expect("cursor marker") as u32;
        let input = source.replacen("|_|", "", 1);
        let cst = eure_parol::parse_tolerant(&input, "<input>").cst();
        find_site(&input, &cst, cursor)
    }

    fn path(segments: &[&str]) -> EurePath {
        EurePath(
            segments
                .iter()
                .map(|s| match *s {
                    "[]" => PathSegment::ArrayIndex(ArrayIndexKind::Push),
                    s if s.starts_with('$') => PathSegment::Extension(s[1..].parse().unwrap()),
                    s => PathSegment::Ident(s.parse().unwrap()),
                })
                .collect(),
        )
    }

    fn key(site: &CompletionSite) -> (&EurePath, &[String]) {
        match &site.kind {
            SiteKind::Key { parent, used } => (parent, used),
            other => panic!("expected key site, got {other:?}"),
        }
    }

    fn value(site: &CompletionSite) -> (&EurePath, ValueStyle) {
        match &site.kind {
            SiteKind::Value { path, style } => (path, *style),
            other => panic!("expected value site, got {other:?}"),
        }
    }

    #[test]
    fn empty_document_is_root_key_site() {
        let s = site("|_|").unwrap();
        assert_eq!(key(&s).0, &path(&[]));
        assert_eq!(s.partial, "");
    }

    #[test]
    fn partial_section_key() {
        let s = site("@ scr|_|").unwrap();
        assert_eq!(key(&s).0, &path(&[]));
        assert_eq!(s.partial, "scr");
        assert_eq!(s.replace, InputSpan { start: 2, end: 5 });
    }

    #[test]
    fn after_at_symbol() {
        let s = site("@|_|").unwrap();
        assert_eq!(key(&s).0, &path(&[]));
        assert_eq!(s.partial, "");
    }

    #[test]
    fn trailing_dot_in_section_header() {
        let s = site("@ myfield.|_|").unwrap();
        assert_eq!(key(&s).0, &path(&["myfield"]));
        assert_eq!(s.partial, "");
    }

    #[test]
    fn trailing_dot_in_binding_keys() {
        let s = site("user.address.|_|").unwrap();
        assert_eq!(key(&s).0, &path(&["user", "address"]));
    }

    #[test]
    fn value_after_equals() {
        let s = site("key = |_|").unwrap();
        let (p, style) = value(&s);
        assert_eq!(p, &path(&["key"]));
        assert_eq!(style, ValueStyle::Bind);
    }

    #[test]
    fn value_after_equals_in_flat_section() {
        let s = site("@ a.b.c\nkey = |_|").unwrap();
        assert_eq!(value(&s).0, &path(&["a", "b", "c", "key"]));
    }

    #[test]
    fn partial_value_word() {
        let s = site("key = tr|_|").unwrap();
        assert_eq!(value(&s).0, &path(&["key"]));
        assert_eq!(s.partial, "tr");
    }

    #[test]
    fn text_binding_value() {
        let s = site("@ actions[]\n$variant: |_|").unwrap();
        let (p, style) = value(&s);
        assert_eq!(p, &path(&["actions", "[]", "$variant"]));
        assert_eq!(style, ValueStyle::Text);
    }

    #[test]
    fn blank_line_in_flat_section_with_used_keys_and_hint() {
        let s = site("@ actions[]\n$variant: set-text\nspeaker = \"a\"\n|_|").unwrap();
        let (parent, used) = key(&s);
        assert_eq!(parent, &path(&["actions", "[]"]));
        assert_eq!(used, &["speaker".to_string()]);
        assert_eq!(s.hints.len(), 1);
        assert_eq!(s.hints[0].prefix_len, 2);
    }

    #[test]
    fn partial_key_in_flat_section() {
        let s = site("@ user\nname = \"Alice\"\nem|_|").unwrap();
        let (parent, used) = key(&s);
        assert_eq!(parent, &path(&["user"]));
        assert_eq!(used, &["name".to_string()]);
        assert_eq!(s.partial, "em");
    }

    #[test]
    fn inside_block_body() {
        let s = site("@ user {\n    |_|\n}").unwrap();
        assert_eq!(key(&s).0, &path(&["user"]));
    }

    #[test]
    fn inside_unclosed_block_body() {
        let s = site("@ config {\n    @ servers[] {\n        |_|").unwrap();
        assert_eq!(key(&s).0, &path(&["config", "servers", "[]"]));
    }

    #[test]
    fn later_section_scope_wins() {
        let s =
            site("@ users[]\nname = \"Alice\"\n\n@ users[].roles[]\nrole_name = \"admin\"\n|_|")
                .unwrap();
        let (parent, used) = key(&s);
        assert_eq!(parent, &path(&["users", "[]", "roles", "[]"]));
        assert_eq!(used, &["role_name".to_string()]);
    }

    #[test]
    fn inline_object_entries() {
        let s = site("user = { name => \"Alice\", |_| }").unwrap();
        let (parent, used) = key(&s);
        assert_eq!(parent, &path(&["user"]));
        assert_eq!(used, &["name".to_string()]);
    }

    #[test]
    fn array_element_value() {
        let s = site("tags = [\"a\", |_|]").unwrap();
        assert_eq!(value(&s).0, &path(&["tags", "[]"]));
    }

    #[test]
    fn block_binding_scope() {
        let s = site("user {\n  na|_|\n}").unwrap();
        assert_eq!(key(&s).0, &path(&["user"]));
        assert_eq!(s.partial, "na");
    }

    #[test]
    fn cursor_after_complete_value_is_key_site() {
        let s = site("key = 1\n|_|").unwrap();
        let (parent, used) = key(&s);
        assert_eq!(parent, &path(&[]));
        assert_eq!(used, &["key".to_string()]);
        assert_eq!(s.anchor, None);
    }

    #[test]
    fn anchor_on_section_key_covers_whole_key() {
        let s = site("@ user.add|_|ress.city\nname = 1").unwrap();
        assert_eq!(
            s.anchor,
            Some(Anchor {
                kind: AnchorKind::Key,
                path: path(&["user", "address"]),
                span: InputSpan { start: 7, end: 14 },
            })
        );
    }

    #[test]
    fn anchor_on_binding_key_in_flat_section() {
        let s = site("@ user\nna|_|me = \"Alice\"").unwrap();
        assert_eq!(
            s.anchor,
            Some(Anchor {
                kind: AnchorKind::Key,
                path: path(&["user", "name"]),
                span: InputSpan { start: 7, end: 11 },
            })
        );
    }

    #[test]
    fn anchor_on_primitive_value() {
        let s = site("@ user\nname = \"Al|_|ice\"").unwrap();
        assert_eq!(
            s.anchor,
            Some(Anchor {
                kind: AnchorKind::Value,
                path: path(&["user", "name"]),
                span: InputSpan { start: 14, end: 21 },
            })
        );
    }

    #[test]
    fn anchor_on_text_binding_value() {
        let s = site("@ actions[]\n$variant:  set-t|_|ext  \nspeaker = \"a\"").unwrap();
        assert_eq!(
            s.anchor,
            Some(Anchor {
                kind: AnchorKind::Value,
                path: path(&["actions", "[]", "$variant"]),
                span: InputSpan { start: 23, end: 31 },
            })
        );
    }

    #[test]
    fn anchor_on_object_entry_key() {
        let s = site("user = { na|_|me => \"Alice\" }").unwrap();
        assert_eq!(
            s.anchor,
            Some(Anchor {
                kind: AnchorKind::Key,
                path: path(&["user", "name"]),
                span: InputSpan { start: 9, end: 13 },
            })
        );
    }

    #[test]
    fn free_positions_have_no_anchor() {
        assert_eq!(site("key = |_|").unwrap().anchor, None);
        assert_eq!(site("@ myfield.|_|").unwrap().anchor, None);
        assert_eq!(site("@ user {\n    |_|\n}").unwrap().anchor, None);
    }
}
