//! Semantic token support for Eure syntax highlighting.
//!
//! This module provides types and functions for extracting semantic tokens
//! from Eure source code, suitable for use with LSP semantic token features.

use eure_tree::prelude::*;
use eure_tree::tree::InputSpan;

/// Semantic token types specific to Eure.
///
/// These represent the limited subset of LSP semantic token types
/// that are relevant for Eure syntax.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticTokenType {
    /// Keywords: `true`, `false`, `null`, `!` (hole)
    Keyword = 0,
    /// Numeric literals: integers and floats
    Number = 1,
    /// String literals and text blocks
    String = 2,
    /// Line and block comments
    Comment = 3,
    /// Operators: `=`, `=>`, `.`, `,`, `\`
    Operator = 4,
    /// Property/key identifiers
    Property = 5,
    /// Punctuation: `{`, `}`, `[`, `]`, `(`, `)`, `:`, `#`
    Punctuation = 6,
    /// Code block delimiters and content
    Macro = 7,
    /// Code block language tags (e.g., `rust` in ` ```rust`)
    Decorator = 8,
    /// Section marker: `@`
    SectionMarker = 9,
    /// Extension marker: `$`
    ExtensionMarker = 10,
    /// Extension identifiers (after `$`)
    ExtensionIdent = 11,
}

impl SemanticTokenType {
    /// Returns all token types in their defined order.
    pub fn all() -> &'static [SemanticTokenType] {
        &[
            SemanticTokenType::Keyword,
            SemanticTokenType::Number,
            SemanticTokenType::String,
            SemanticTokenType::Comment,
            SemanticTokenType::Operator,
            SemanticTokenType::Property,
            SemanticTokenType::Punctuation,
            SemanticTokenType::Macro,
            SemanticTokenType::Decorator,
            SemanticTokenType::SectionMarker,
            SemanticTokenType::ExtensionMarker,
            SemanticTokenType::ExtensionIdent,
        ]
    }

    /// Returns the token type index for LSP encoding.
    pub fn index(self) -> u32 {
        self as u32
    }
}

/// Semantic token modifiers specific to Eure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticTokenModifier {
    /// Keys/properties being declared
    Declaration = 0,
    /// Section header definitions
    Definition = 1,
    /// Token is within a section header (after `@` and before section body)
    SectionHeader = 2,
}

impl SemanticTokenModifier {
    /// Returns all modifiers in their defined order.
    pub fn all() -> &'static [SemanticTokenModifier] {
        &[
            SemanticTokenModifier::Declaration,
            SemanticTokenModifier::Definition,
            SemanticTokenModifier::SectionHeader,
        ]
    }

    /// Returns the modifier as a bitmask value.
    pub fn bitmask(self) -> u32 {
        1 << (self as u32)
    }
}

/// A single semantic token with position and type information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticToken {
    /// Start offset in the source (byte offset)
    pub start: u32,
    /// Length of the token in bytes
    pub length: u32,
    /// The semantic token type
    pub token_type: SemanticTokenType,
    /// Bitfield of modifiers
    pub modifiers: u32,
}

impl SemanticToken {
    /// Creates a new semantic token.
    pub fn new(start: u32, length: u32, token_type: SemanticTokenType) -> Self {
        Self {
            start,
            length,
            token_type,
            modifiers: 0,
        }
    }

    /// Creates a new semantic token with modifiers.
    pub fn with_modifiers(
        start: u32,
        length: u32,
        token_type: SemanticTokenType,
        modifiers: u32,
    ) -> Self {
        Self {
            start,
            length,
            token_type,
            modifiers,
        }
    }
}

/// Extract semantic tokens from the CST.
///
/// Returns a vector of semantic tokens sorted by their start position.
pub fn semantic_tokens(input: &str, cst: &Cst) -> Vec<SemanticToken> {
    let mut collector = SemanticTokenCollector::new(input);
    collector.collect(cst);
    collector.into_tokens()
}

struct SemanticTokenCollector<'a> {
    input: &'a str,
    tokens: Vec<SemanticToken>,
}

impl<'a> SemanticTokenCollector<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            tokens: Vec::new(),
        }
    }

    fn collect(&mut self, cst: &Cst) {
        self.visit_node(cst, cst.root(), VisitContext::default());
    }

    fn into_tokens(mut self) -> Vec<SemanticToken> {
        // Sort by start position
        self.tokens.sort_by_key(|t| t.start);
        self.tokens
    }

    fn visit_node(&mut self, cst: &Cst, node_id: CstNodeId, ctx: VisitContext) {
        let Some(node_data) = cst.node_data(node_id) else {
            return;
        };

        match node_data {
            CstNode::Terminal { kind, data } => {
                self.handle_terminal(kind, data, ctx);
            }
            CstNode::NonTerminal { kind, .. } => {
                // Update context based on non-terminal kind
                let new_ctx = self.update_context(kind, ctx);

                // Visit children
                for child_id in cst.children(node_id) {
                    self.visit_node(cst, child_id, new_ctx);
                }
            }
        }
    }

    fn update_context(&self, kind: NonTerminalKind, ctx: VisitContext) -> VisitContext {
        match kind {
            // Key context - identifiers here are properties
            NonTerminalKind::Keys
            | NonTerminalKind::Key
            | NonTerminalKind::KeyBase
            | NonTerminalKind::KeyIdent
            | NonTerminalKind::KeyTuple
            | NonTerminalKind::KeyValue => ctx.with_key_context(true),

            // Section header - add section header modifier
            NonTerminalKind::Section => ctx.with_section_header(true),

            // Section body - reset section header (only @ and Keys are in the header)
            NonTerminalKind::SectionBody => ctx.with_section_header(false),

            // Extension namespace - identifiers here are extension idents
            NonTerminalKind::ExtensionNameSpace => ctx.with_extension_namespace(true),

            // Value context - reset key context
            NonTerminalKind::Value
            | NonTerminalKind::ValueBinding
            | NonTerminalKind::Array
            | NonTerminalKind::Object
            | NonTerminalKind::Tuple => ctx.with_key_context(false),

            _ => ctx,
        }
    }

    fn handle_terminal(&mut self, kind: TerminalKind, data: TerminalData, ctx: VisitContext) {
        let TerminalData::Input(span) = data else {
            return;
        };

        // Handle code block starts specially (they contain language tags)
        if self.handle_code_block_start(kind, span) {
            return;
        }

        // Handle inline code specially
        if self.handle_inline_code(kind, span) {
            return;
        }

        // Map terminal kind to token type
        let Some(token_type) = self.terminal_to_token_type(kind, &ctx) else {
            return;
        };

        // Compute modifiers
        let modifiers = self.compute_modifiers(kind, ctx);

        self.tokens.push(SemanticToken::with_modifiers(
            span.start,
            span.end - span.start,
            token_type,
            modifiers,
        ));
    }

    fn handle_code_block_start(&mut self, kind: TerminalKind, span: InputSpan) -> bool {
        let backtick_count = match kind {
            TerminalKind::CodeBlockStart3 => 3,
            TerminalKind::CodeBlockStart4 => 4,
            TerminalKind::CodeBlockStart5 => 5,
            TerminalKind::CodeBlockStart6 => 6,
            _ => return false,
        };

        let text = span.as_str(self.input);

        // Emit backticks as Macro token
        self.tokens.push(SemanticToken::new(
            span.start,
            backtick_count,
            SemanticTokenType::Macro,
        ));

        // Extract language tag (after backticks, before whitespace/newline)
        let after_backticks = &text[backtick_count as usize..];
        let tag_len = after_backticks
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .count();

        if tag_len > 0 {
            self.tokens.push(SemanticToken::new(
                span.start + backtick_count,
                tag_len as u32,
                SemanticTokenType::Decorator,
            ));
        }

        true
    }

    fn handle_inline_code(&mut self, kind: TerminalKind, span: InputSpan) -> bool {
        match kind {
            TerminalKind::InlineCode1 => {
                // Format: lang`code` - single backticks
                let text = span.as_str(self.input);

                // Find the backtick position
                if let Some(backtick_pos) = text.find('`') {
                    let tag_len = backtick_pos;

                    // Emit language tag as Decorator (if present)
                    if tag_len > 0 {
                        self.tokens.push(SemanticToken::new(
                            span.start,
                            tag_len as u32,
                            SemanticTokenType::Decorator,
                        ));
                    }

                    // Emit the rest as Macro (backtick + content + backtick)
                    self.tokens.push(SemanticToken::new(
                        span.start + tag_len as u32,
                        (span.end - span.start) - tag_len as u32,
                        SemanticTokenType::Macro,
                    ));
                }
                true
            }
            TerminalKind::InlineCodeStart2 => {
                // Format: lang`` - double backtick start
                let text = span.as_str(self.input);

                // Find the double backtick position
                if let Some(backtick_pos) = text.find("``") {
                    let tag_len = backtick_pos;

                    // Emit language tag as Decorator (if present)
                    if tag_len > 0 {
                        self.tokens.push(SemanticToken::new(
                            span.start,
                            tag_len as u32,
                            SemanticTokenType::Decorator,
                        ));
                    }

                    // Emit backticks as Macro
                    self.tokens.push(SemanticToken::new(
                        span.start + tag_len as u32,
                        2,
                        SemanticTokenType::Macro,
                    ));
                }
                true
            }
            _ => false,
        }
    }

    fn terminal_to_token_type(
        &self,
        kind: TerminalKind,
        ctx: &VisitContext,
    ) -> Option<SemanticTokenType> {
        match kind {
            // Keywords
            TerminalKind::True | TerminalKind::False | TerminalKind::Null | TerminalKind::Hole => {
                Some(SemanticTokenType::Keyword)
            }

            // Numbers
            TerminalKind::Integer | TerminalKind::Float => Some(SemanticTokenType::Number),

            // Strings
            TerminalKind::Str | TerminalKind::Text => Some(SemanticTokenType::String),

            // Comments
            TerminalKind::LineComment | TerminalKind::BlockComment => {
                Some(SemanticTokenType::Comment)
            }

            // Section marker - distinct from other operators
            TerminalKind::At => Some(SemanticTokenType::SectionMarker),

            // Extension marker - distinct from other operators
            TerminalKind::Dollar => Some(SemanticTokenType::ExtensionMarker),

            // Operators
            TerminalKind::Bind
            | TerminalKind::MapBind
            | TerminalKind::Dot
            | TerminalKind::Comma
            | TerminalKind::Esc => Some(SemanticTokenType::Operator),

            // Punctuation
            TerminalKind::LBrace
            | TerminalKind::RBrace
            | TerminalKind::LBracket
            | TerminalKind::RBracket
            | TerminalKind::LParen
            | TerminalKind::RParen
            | TerminalKind::TextStart
            | TerminalKind::Hash => Some(SemanticTokenType::Punctuation),

            // Identifiers - context dependent
            TerminalKind::Ident => {
                if ctx.in_extension_namespace {
                    Some(SemanticTokenType::ExtensionIdent)
                } else if ctx.in_key_context {
                    Some(SemanticTokenType::Property)
                } else {
                    // Identifiers outside key context (e.g., in boolean context like true/false parsed as ident)
                    // This shouldn't happen often since true/false/null have their own tokens
                    None
                }
            }

            // Code block content and ends
            TerminalKind::InlineCodeEnd2
            | TerminalKind::CodeBlockEnd3
            | TerminalKind::CodeBlockEnd4
            | TerminalKind::CodeBlockEnd5
            | TerminalKind::CodeBlockEnd6
            | TerminalKind::NoBacktick
            | TerminalKind::NoBacktickInline
            | TerminalKind::Backtick1
            | TerminalKind::Backtick2
            | TerminalKind::Backtick3
            | TerminalKind::Backtick4
            | TerminalKind::Backtick5 => Some(SemanticTokenType::Macro),

            // Skip whitespace/newlines
            TerminalKind::Whitespace
            | TerminalKind::NewLine
            | TerminalKind::GrammarNewline
            | TerminalKind::Ws => None,

            // Skip code block starts (handled specially)
            TerminalKind::InlineCode1
            | TerminalKind::InlineCodeStart2
            | TerminalKind::CodeBlockStart3
            | TerminalKind::CodeBlockStart4
            | TerminalKind::CodeBlockStart5
            | TerminalKind::CodeBlockStart6 => None,
        }
    }

    fn compute_modifiers(&self, kind: TerminalKind, ctx: VisitContext) -> u32 {
        let mut modifiers = 0;

        // Add Declaration modifier for keys/properties
        if ctx.in_key_context && kind == TerminalKind::Ident {
            modifiers |= SemanticTokenModifier::Declaration.bitmask();
        }

        // Add Definition modifier for section header identifiers
        if ctx.in_section_header && kind == TerminalKind::Ident {
            modifiers |= SemanticTokenModifier::Definition.bitmask();
        }

        // Add SectionHeader modifier for all tokens in section header
        if ctx.in_section_header {
            modifiers |= SemanticTokenModifier::SectionHeader.bitmask();
        }

        modifiers
    }
}

/// Context for visiting nodes, tracking semantic information.
#[derive(Debug, Clone, Copy, Default)]
struct VisitContext {
    /// Whether we're currently in a key/property context
    in_key_context: bool,
    /// Whether we're in a section header
    in_section_header: bool,
    /// Whether we're in an extension namespace (after `$`)
    in_extension_namespace: bool,
}

impl VisitContext {
    fn with_key_context(self, in_key_context: bool) -> Self {
        Self {
            in_key_context,
            ..self
        }
    }

    fn with_section_header(self, in_section_header: bool) -> Self {
        Self {
            in_section_header,
            ..self
        }
    }

    fn with_extension_namespace(self, in_extension_namespace: bool) -> Self {
        Self {
            in_extension_namespace,
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_and_get_tokens(input: &str) -> Vec<SemanticToken> {
        let cst = eure_parol::parse(input).expect("Failed to parse");
        semantic_tokens(input, &cst)
    }

    #[test]
    fn test_simple_binding() {
        let input = "key = 42";
        let tokens = parse_and_get_tokens(input);

        // Should have: key (Property), = (Operator), 42 (Number)
        assert_eq!(tokens.len(), 3);

        assert_eq!(tokens[0].token_type, SemanticTokenType::Property);
        assert_eq!(
            &input[tokens[0].start as usize..(tokens[0].start + tokens[0].length) as usize],
            "key"
        );

        assert_eq!(tokens[1].token_type, SemanticTokenType::Operator);
        assert_eq!(
            &input[tokens[1].start as usize..(tokens[1].start + tokens[1].length) as usize],
            "="
        );

        assert_eq!(tokens[2].token_type, SemanticTokenType::Number);
        assert_eq!(
            &input[tokens[2].start as usize..(tokens[2].start + tokens[2].length) as usize],
            "42"
        );
    }

    #[test]
    fn test_keywords() {
        let input = "a = true\nb = false\nc = null";
        let tokens = parse_and_get_tokens(input);

        let keywords: Vec<_> = tokens
            .iter()
            .filter(|t| t.token_type == SemanticTokenType::Keyword)
            .collect();

        assert_eq!(keywords.len(), 3);
    }

    #[test]
    fn test_string_literal() {
        let input = r#"name = "hello""#;
        let tokens = parse_and_get_tokens(input);

        let strings: Vec<_> = tokens
            .iter()
            .filter(|t| t.token_type == SemanticTokenType::String)
            .collect();

        assert_eq!(strings.len(), 1);
        assert_eq!(
            &input[strings[0].start as usize..(strings[0].start + strings[0].length) as usize],
            "\"hello\""
        );
    }

    #[test]
    fn test_section_header() {
        let input = "@section.name\nkey = 1";
        let tokens = parse_and_get_tokens(input);

        // Should have @ (SectionMarker), section (Property), . (Operator), name (Property), etc.
        let at_token = tokens.iter().find(|t| {
            let text = &input[t.start as usize..(t.start + t.length) as usize];
            text == "@"
        });
        assert!(at_token.is_some());
        assert_eq!(
            at_token.unwrap().token_type,
            SemanticTokenType::SectionMarker
        );
    }

    #[test]
    fn test_section_header_modifier() {
        let input = "@section.name\nkey = 1";
        let tokens = parse_and_get_tokens(input);

        // All tokens in section header should have SectionHeader modifier
        let section_header_mask = SemanticTokenModifier::SectionHeader.bitmask();

        // @ should have SectionHeader modifier
        let at_token = tokens
            .iter()
            .find(|t| {
                let text = &input[t.start as usize..(t.start + t.length) as usize];
                text == "@"
            })
            .unwrap();
        assert!(
            at_token.modifiers & section_header_mask != 0,
            "@ should have SectionHeader modifier"
        );

        // "section" should have SectionHeader modifier
        let section_token = tokens
            .iter()
            .find(|t| {
                let text = &input[t.start as usize..(t.start + t.length) as usize];
                text == "section"
            })
            .unwrap();
        assert!(
            section_token.modifiers & section_header_mask != 0,
            "section should have SectionHeader modifier"
        );

        // "." should have SectionHeader modifier
        let dot_token = tokens
            .iter()
            .find(|t| {
                let text = &input[t.start as usize..(t.start + t.length) as usize];
                text == "."
            })
            .unwrap();
        assert!(
            dot_token.modifiers & section_header_mask != 0,
            ". should have SectionHeader modifier"
        );

        // "name" should have SectionHeader modifier
        let name_token = tokens
            .iter()
            .find(|t| {
                let text = &input[t.start as usize..(t.start + t.length) as usize];
                text == "name"
            })
            .unwrap();
        assert!(
            name_token.modifiers & section_header_mask != 0,
            "name should have SectionHeader modifier"
        );

        // "key" in section body should NOT have SectionHeader modifier
        let key_token = tokens
            .iter()
            .find(|t| {
                let text = &input[t.start as usize..(t.start + t.length) as usize];
                text == "key"
            })
            .unwrap();
        assert!(
            key_token.modifiers & section_header_mask == 0,
            "key should NOT have SectionHeader modifier"
        );
    }

    #[test]
    fn test_extension_namespace() {
        let input = "$variant = \"some-value\"";
        let tokens = parse_and_get_tokens(input);

        // Should have $ (ExtensionMarker), variant (ExtensionIdent), = (Operator), "some-value" (String)
        let dollar_token = tokens.iter().find(|t| {
            let text = &input[t.start as usize..(t.start + t.length) as usize];
            text == "$"
        });
        assert!(dollar_token.is_some());
        assert_eq!(
            dollar_token.unwrap().token_type,
            SemanticTokenType::ExtensionMarker
        );

        let variant_token = tokens.iter().find(|t| {
            let text = &input[t.start as usize..(t.start + t.length) as usize];
            text == "variant"
        });
        assert!(variant_token.is_some());
        assert_eq!(
            variant_token.unwrap().token_type,
            SemanticTokenType::ExtensionIdent
        );
    }

    #[test]
    fn test_code_block_with_language() {
        let input = "code = ```rust\nfn main() {}\n```";
        let tokens = parse_and_get_tokens(input);

        // Should have language tag as Decorator
        let decorators: Vec<_> = tokens
            .iter()
            .filter(|t| t.token_type == SemanticTokenType::Decorator)
            .collect();

        assert_eq!(decorators.len(), 1);
        assert_eq!(
            &input[decorators[0].start as usize
                ..(decorators[0].start + decorators[0].length) as usize],
            "rust"
        );
    }

    #[test]
    fn test_inline_code() {
        let input = "code = rust`let x = 1;`";
        let tokens = parse_and_get_tokens(input);

        // Should have language tag as Decorator
        let decorators: Vec<_> = tokens
            .iter()
            .filter(|t| t.token_type == SemanticTokenType::Decorator)
            .collect();

        assert_eq!(decorators.len(), 1);
        assert_eq!(
            &input[decorators[0].start as usize
                ..(decorators[0].start + decorators[0].length) as usize],
            "rust"
        );

        // Should have code as Macro
        let macros: Vec<_> = tokens
            .iter()
            .filter(|t| t.token_type == SemanticTokenType::Macro)
            .collect();

        assert!(!macros.is_empty());
    }

    #[test]
    fn test_comment() {
        let input = "// this is a comment\nkey = 1";
        let tokens = parse_and_get_tokens(input);

        let comments: Vec<_> = tokens
            .iter()
            .filter(|t| t.token_type == SemanticTokenType::Comment)
            .collect();

        assert_eq!(comments.len(), 1);
    }

    #[test]
    fn test_punctuation() {
        let input = "arr = [1, 2, 3]";
        let tokens = parse_and_get_tokens(input);

        let punctuation: Vec<_> = tokens
            .iter()
            .filter(|t| t.token_type == SemanticTokenType::Punctuation)
            .collect();

        // [ and ]
        assert!(punctuation.len() >= 2);
    }

    #[test]
    fn test_declaration_modifier() {
        let input = "myKey = 42";
        let tokens = parse_and_get_tokens(input);

        let key_token = tokens
            .iter()
            .find(|t| t.token_type == SemanticTokenType::Property)
            .unwrap();

        assert!(key_token.modifiers & SemanticTokenModifier::Declaration.bitmask() != 0);
    }
}
