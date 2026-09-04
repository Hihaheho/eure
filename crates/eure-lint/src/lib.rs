//! Static analysis for Eure source files.
//!
//! The linter operates on the concrete syntax tree so that rules can reason
//! about the spelling and layout of a document without changing its meaning.

use std::collections::BTreeSet;
use std::convert::Infallible;

use eure_tree::node_kind::{NonTerminalKind, TerminalKind};
use eure_tree::nodes::{
    RootTextBindingHandle, RootTextBindingView, SectionHandle, SectionView, TextBindingHandle,
    TextBindingView,
};
use eure_tree::prelude::{
    Cst, CstFacade, CstNodeId, CstVisitor, CstVisitorSuper as _, InputSpan, NonTerminalHandle as _,
    TerminalData,
};
use thiserror::Error;

/// Stable identifier for a built-in lint rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuleId {
    /// A comment-looking delimiter is part of a `:` text value.
    NoCommentInTextBinding,
    /// `@ path { ... }` is redundant when the path is not an array append.
    RedundantAtWithBraces,
    /// Indentation suggests an `@` section is nested inside a braceless one.
    NestedAtInsideBracelessAt,
}

impl RuleId {
    /// Return the stable, kebab-case rule name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoCommentInTextBinding => "no-comment-in-text-binding",
            Self::RedundantAtWithBraces => "redundant-at-with-braces",
            Self::NestedAtInsideBracelessAt => "nested-at-inside-braceless-at",
        }
    }
}

impl std::fmt::Display for RuleId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Severity assigned to a lint diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Hint,
}

/// Confidence with which a fix can be applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applicability {
    /// The edit is semantics-preserving for every matched input.
    Always,
    /// The edit depends on an inferred intent and requires review.
    MaybeIncorrect,
}

/// A byte-oriented source edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub span: InputSpan,
    pub replacement: String,
}

/// A suggested correction for a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fix {
    pub message: String,
    pub applicability: Applicability,
    pub edits: Vec<TextEdit>,
}

/// A single lint finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub rule: RuleId,
    pub severity: Severity,
    pub message: String,
    pub span: InputSpan,
    pub help: Option<String>,
    pub fix: Option<Fix>,
}

/// Selection of enabled built-in rules.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LintConfig {
    disabled: BTreeSet<RuleId>,
}

impl LintConfig {
    /// Disable a rule.
    #[must_use]
    pub fn without(mut self, rule: RuleId) -> Self {
        self.disabled.insert(rule);
        self
    }

    /// Return whether a rule is enabled.
    pub fn is_enabled(&self, rule: RuleId) -> bool {
        !self.disabled.contains(&rule)
    }
}

/// Run all enabled built-in rules on a valid Eure CST.
pub fn lint(source: &str, cst: &Cst, config: &LintConfig) -> Vec<Diagnostic> {
    let mut collector = Collector::new(source, config);
    // The visitor only uses `Infallible`, so traversal cannot return an error.
    match cst.visit_from_root(&mut collector) {
        Ok(()) => {}
        Err(never) => match never {},
    }
    collector.finish()
}

/// Run all built-in rules with their default configuration.
pub fn lint_default(source: &str, cst: &Cst) -> Vec<Diagnostic> {
    lint(source, cst, &LintConfig::default())
}

/// Which fixes should be included by [`apply_fixes`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixMode {
    /// Apply only fixes marked [`Applicability::Always`].
    Safe,
    /// Also apply fixes whose intent must be reviewed.
    IncludeMaybeIncorrect,
}

/// Failure while applying a collection of lint fixes.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ApplyFixError {
    #[error("lint edit range {start}..{end} is outside the source")]
    OutOfBounds { start: u32, end: u32 },
    #[error("lint edit boundary {offset} is not a UTF-8 character boundary")]
    InvalidCharBoundary { offset: u32 },
    #[error("lint edits overlap at byte range {start}..{end}")]
    OverlappingEdits { start: u32, end: u32 },
}

/// Apply non-overlapping fixes to source text.
pub fn apply_fixes(
    source: &str,
    diagnostics: &[Diagnostic],
    mode: FixMode,
) -> Result<String, ApplyFixError> {
    let mut edits: Vec<&TextEdit> = diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic.fix.as_ref())
        .filter(|fix| {
            fix.applicability == Applicability::Always || mode == FixMode::IncludeMaybeIncorrect
        })
        .flat_map(|fix| fix.edits.iter())
        .collect();

    edits.sort_by_key(|edit| (edit.span.start, edit.span.end));

    let mut previous_end = 0;
    for (index, edit) in edits.iter().enumerate() {
        let start = edit.span.start as usize;
        let end = edit.span.end as usize;
        if start > end || end > source.len() {
            return Err(ApplyFixError::OutOfBounds {
                start: edit.span.start,
                end: edit.span.end,
            });
        }
        if !source.is_char_boundary(start) {
            return Err(ApplyFixError::InvalidCharBoundary {
                offset: edit.span.start,
            });
        }
        if !source.is_char_boundary(end) {
            return Err(ApplyFixError::InvalidCharBoundary {
                offset: edit.span.end,
            });
        }
        if index > 0 && start < previous_end {
            return Err(ApplyFixError::OverlappingEdits {
                start: edit.span.start,
                end: edit.span.end,
            });
        }
        previous_end = end;
    }

    let mut output = source.to_owned();
    for edit in edits.into_iter().rev() {
        output.replace_range(
            edit.span.start as usize..edit.span.end as usize,
            &edit.replacement,
        );
    }
    Ok(output)
}

#[derive(Debug, Clone, Copy)]
struct SectionInfo {
    at_span: InputSpan,
    scope: CstNodeId,
    indentation: usize,
    braceless: bool,
}

struct Collector<'a> {
    source: &'a str,
    config: &'a LintConfig,
    diagnostics: Vec<Diagnostic>,
    sections: Vec<SectionInfo>,
}

impl<'a> Collector<'a> {
    fn new(source: &'a str, config: &'a LintConfig) -> Self {
        Self {
            source,
            config,
            diagnostics: Vec::new(),
            sections: Vec::new(),
        }
    }

    fn finish(mut self) -> Vec<Diagnostic> {
        if self.config.is_enabled(RuleId::NestedAtInsideBracelessAt) {
            self.check_indented_sections();
        }
        self.diagnostics.sort_by_key(|diagnostic| {
            (diagnostic.span.start, diagnostic.span.end, diagnostic.rule)
        });
        self.diagnostics
    }

    fn check_indented_sections(&mut self) {
        for (outer_index, outer) in self.sections.iter().enumerate() {
            if !outer.braceless {
                continue;
            }

            for inner in self.sections.iter().skip(outer_index + 1) {
                if inner.scope != outer.scope {
                    continue;
                }
                if inner.indentation <= outer.indentation {
                    break;
                }

                self.diagnostics.push(Diagnostic {
                    rule: RuleId::NestedAtInsideBracelessAt,
                    severity: Severity::Error,
                    message: "an indented `@` section is not nested inside the preceding braceless `@` section".to_owned(),
                    span: inner.at_span,
                    help: Some(
                        "make the outer section an explicit `path { ... }` block, or write the child's full path"
                            .to_owned(),
                    ),
                    fix: None,
                });
            }
        }
    }

    fn check_text_binding<F: CstFacade>(
        &mut self,
        node: CstNodeId,
        colon_kind: TerminalKind,
        tree: &F,
    ) {
        let Some(text_span) = find_terminal_span(tree, node, TerminalKind::Text) else {
            return;
        };
        let text = text_span.as_str(self.source);
        let Some((comment_offset, marker)) = find_comment_like_delimiter(text) else {
            return;
        };
        let comment_start = text_span.start + comment_offset as u32;
        let comment_span = InputSpan::new(comment_start, comment_start + marker.len() as u32);

        let Some(colon_span) = find_terminal_span(tree, node, colon_kind) else {
            return;
        };
        let colon_start = colon_span.end - 1;
        let value = text[..comment_offset].trim_end();
        let replacement = format!(" = {} ", quote_eure_string(value));

        self.diagnostics.push(Diagnostic {
            rule: RuleId::NoCommentInTextBinding,
            severity: Severity::Error,
            message: format!("`{marker}` is part of this text value, not a comment"),
            span: comment_span,
            help: Some("use `=` with a quoted string before a trailing comment".to_owned()),
            fix: Some(Fix {
                message: "convert the text binding to a quoted value".to_owned(),
                applicability: Applicability::Always,
                edits: vec![TextEdit {
                    span: InputSpan::new(colon_start, comment_start),
                    replacement,
                }],
            }),
        });
    }

    fn check_section<F: CstFacade>(&mut self, handle: SectionHandle, view: SectionView, tree: &F) {
        let node = handle.node_id();
        let Some(at_span) = tree.span(view.at.node_id()) else {
            return;
        };
        let Some(keys_span) = tree.span(view.keys.node_id()) else {
            return;
        };
        let braceless = !has_descendant_non_terminal(
            tree,
            view.section_body.node_id(),
            NonTerminalKind::BlockBody,
        );
        let has_array_marker =
            has_descendant_non_terminal(tree, view.keys.node_id(), NonTerminalKind::ArrayMarker);
        let scope = nearest_ancestor(tree, node, NonTerminalKind::Eure)
            .unwrap_or_else(|| tree.root_handle().node_id());

        self.sections.push(SectionInfo {
            at_span,
            scope,
            indentation: indentation_at(self.source, at_span.start as usize),
            braceless,
        });

        if self.config.is_enabled(RuleId::RedundantAtWithBraces) && !braceless && !has_array_marker
        {
            let after_at = at_span.end as usize;
            let whitespace_end = self.source[after_at..keys_span.start as usize]
                .char_indices()
                .take_while(|(_, character)| matches!(character, ' ' | '\t'))
                .last()
                .map_or(after_at, |(offset, character)| {
                    after_at + offset + character.len_utf8()
                });

            self.diagnostics.push(Diagnostic {
                rule: RuleId::RedundantAtWithBraces,
                severity: Severity::Warning,
                message: "`@` is redundant on a non-array section with braces".to_owned(),
                span: at_span,
                help: Some("remove `@`; the braces already create the scope".to_owned()),
                fix: Some(Fix {
                    message: "remove the redundant `@`".to_owned(),
                    applicability: Applicability::Always,
                    edits: vec![TextEdit {
                        span: InputSpan::new(at_span.start, whitespace_end as u32),
                        replacement: String::new(),
                    }],
                }),
            });
        }
    }
}

impl<F: CstFacade> CstVisitor<F> for Collector<'_> {
    type Error = Infallible;

    fn visit_text_binding(
        &mut self,
        handle: TextBindingHandle,
        view: TextBindingView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        if self.config.is_enabled(RuleId::NoCommentInTextBinding) {
            self.check_text_binding(handle.node_id(), TerminalKind::TextStart, tree);
        }
        self.visit_text_binding_super(handle, view, tree)
    }

    fn visit_root_text_binding(
        &mut self,
        handle: RootTextBindingHandle,
        view: RootTextBindingView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        if self.config.is_enabled(RuleId::NoCommentInTextBinding) {
            self.check_text_binding(handle.node_id(), TerminalKind::NewlineTextStart, tree);
        }
        self.visit_root_text_binding_super(handle, view, tree)
    }

    fn visit_section(
        &mut self,
        handle: SectionHandle,
        view: SectionView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.check_section(handle, view, tree);
        self.visit_section_super(handle, view, tree)
    }
}

fn find_terminal_span<F: CstFacade>(
    tree: &F,
    node: CstNodeId,
    expected: TerminalKind,
) -> Option<InputSpan> {
    match tree.node_data(node)? {
        eure_tree::CstNode::Terminal { kind, data } => {
            if kind == expected {
                match data {
                    TerminalData::Input(span) => Some(span),
                    TerminalData::Dynamic(_) => None,
                }
            } else {
                None
            }
        }
        eure_tree::CstNode::NonTerminal { .. } => tree
            .children(node)
            .find_map(|child| find_terminal_span(tree, child, expected)),
    }
}

fn has_descendant_non_terminal<F: CstFacade>(
    tree: &F,
    node: CstNodeId,
    expected: NonTerminalKind,
) -> bool {
    tree.children(node).any(|child| {
        matches!(
            tree.node_data(child),
            Some(eure_tree::CstNode::NonTerminal { kind, .. }) if kind == expected
        ) || has_descendant_non_terminal(tree, child, expected)
    })
}

fn nearest_ancestor<F: CstFacade>(
    tree: &F,
    node: CstNodeId,
    expected: NonTerminalKind,
) -> Option<CstNodeId> {
    let mut current = tree.parent(node);
    while let Some(candidate) = current {
        if matches!(
            tree.node_data(candidate),
            Some(eure_tree::CstNode::NonTerminal { kind, .. }) if kind == expected
        ) {
            return Some(candidate);
        }
        current = tree.parent(candidate);
    }
    None
}

fn indentation_at(source: &str, offset: usize) -> usize {
    let line_start = source[..offset]
        .rfind(['\n', '\r'])
        .map_or(0, |index| index + 1);
    source[line_start..offset]
        .chars()
        .map(|character| if character == '\t' { 4 } else { 1 })
        .sum()
}

fn find_comment_like_delimiter(text: &str) -> Option<(usize, &'static str)> {
    text.char_indices().find_map(|(offset, _)| {
        if offset != 0
            && !text[..offset]
                .chars()
                .next_back()
                .is_some_and(char::is_whitespace)
        {
            return None;
        }
        let remainder = &text[offset..];
        if remainder.starts_with("//") {
            Some((offset, "//"))
        } else if remainder.starts_with("/*") {
            Some((offset, "/*"))
        } else {
            None
        }
    })
}

fn quote_eure_string(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('"');
    for character in value.chars() {
        match character {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '\t' => quoted.push_str("\\t"),
            control if control.is_control() => {
                use std::fmt::Write as _;
                write!(quoted, "\\u{{{:x}}}", control as u32)
                    .expect("writing to a String cannot fail");
            }
            other => quoted.push(other),
        }
    }
    quoted.push('"');
    quoted
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lint_source(source: &str) -> Vec<Diagnostic> {
        let cst = eure_parol::parse(source, "test.eure").expect("test fixture should parse");
        lint_default(source, &cst)
    }

    #[test]
    fn text_binding_comment_is_reported_and_fixed() {
        let source = "loop: pingpong // none | loop | pingpong\n";
        let diagnostics = lint_source(source);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, RuleId::NoCommentInTextBinding);
        assert_eq!(diagnostics[0].severity, Severity::Error);
        assert_eq!(
            apply_fixes(source, &diagnostics, FixMode::Safe).unwrap(),
            "loop = \"pingpong\" // none | loop | pingpong\n"
        );
    }

    #[test]
    fn block_comment_in_text_binding_is_reported() {
        let diagnostics = lint_source("loop: pingpong /* modes */\n");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, RuleId::NoCommentInTextBinding);
    }

    #[test]
    fn comment_in_section_root_text_binding_is_reported() {
        let source = "@ mode: pingpong // modes\n";
        let diagnostics = lint_source(source);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            apply_fixes(source, &diagnostics, FixMode::Safe).unwrap(),
            "@ mode = \"pingpong\" // modes\n"
        );
    }

    #[test]
    fn url_in_text_binding_is_not_mistaken_for_comment() {
        assert!(lint_source("url: https://eure.dev/docs\n").is_empty());
    }

    #[test]
    fn at_with_braces_is_reported_and_fixed() {
        let source = "@ library {\n  type: animation_clip\n}\n";
        let diagnostics = lint_source(source);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, RuleId::RedundantAtWithBraces);
        assert_eq!(
            apply_fixes(source, &diagnostics, FixMode::Safe).unwrap(),
            "library {\n  type: animation_clip\n}\n"
        );
    }

    #[test]
    fn array_section_with_braces_keeps_at() {
        assert!(lint_source("@ nodes[] {\n  id: n1\n}\n").is_empty());
    }

    #[test]
    fn visually_nested_at_inside_braceless_at_is_reported() {
        let source = "@ nodes[] {\n  @ animation.library.clips.wave\n  length = 2.0\n    @ tracks[]\n    target: A\n}\n";
        let diagnostics = lint_source(source);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, RuleId::NestedAtInsideBracelessAt);
    }

    #[test]
    fn sibling_full_path_headers_are_not_reported() {
        let source = "@ polygon\ntexture: art/limb.png\n\n@ polygon.mesh\npositions = []\n";
        assert!(lint_source(source).is_empty());
    }
}
