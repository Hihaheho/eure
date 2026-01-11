//! Semantic token support for Eure syntax highlighting.
//!
//! This module provides types and functions for extracting semantic tokens
//! from Eure source code, suitable for use with LSP semantic token features.

use crate::tree::*;
use query_flow::{Db, QueryError, query};

use super::assets::TextFile;
use super::parse::ParseCst;

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
    let mut visitor = SemanticTokenVisitor::new(input);
    let _ = cst.visit_from_root(&mut visitor);
    visitor.into_tokens()
}

/// Query to get semantic tokens for a file.
///
/// Uses tolerant parsing to always produce tokens even with syntax errors.
/// Depends on `ParseCst` query.
#[query]
pub fn get_semantic_tokens(db: &impl Db, file: TextFile) -> Result<Vec<SemanticToken>, QueryError> {
    let parsed_cst = db.query(ParseCst::new(file.clone()))?;
    let source = super::parse::read_text_file(db, file)?;

    Ok(semantic_tokens(source.get(), &parsed_cst.cst))
}

/// Visitor that collects semantic tokens from the CST.
struct SemanticTokenVisitor<'a> {
    input: &'a str,
    tokens: Vec<SemanticToken>,
    /// Whether we're currently in a key/property context
    in_key_context: bool,
    /// Whether we're in a section header
    in_section_header: bool,
    /// Whether we're in an extension namespace (after `$`)
    in_extension_namespace: bool,
}

impl<'a> SemanticTokenVisitor<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            tokens: Vec::new(),
            in_key_context: false,
            in_section_header: false,
            in_extension_namespace: false,
        }
    }

    fn into_tokens(mut self) -> Vec<SemanticToken> {
        self.tokens.sort_by_key(|t| t.start);
        self.tokens
    }

    fn emit_token_with_modifiers(
        &mut self,
        span: InputSpan,
        token_type: SemanticTokenType,
        modifiers: u32,
    ) {
        self.tokens.push(SemanticToken::with_modifiers(
            span.start,
            span.end - span.start,
            token_type,
            modifiers,
        ));
    }

    /// Emit code block start token, splitting into backticks and language tag.
    fn emit_code_block_start(&mut self, span: InputSpan, backtick_count: u32) {
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
    }

    /// Emit inline code token with language tag prefix.
    fn emit_inline_code_1(&mut self, span: InputSpan) {
        let text = span.as_str(self.input);

        // Format: lang`code` - find the backtick position
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
    }

    /// Compute modifiers for the current context.
    fn current_modifiers(&self, is_ident: bool) -> u32 {
        let mut modifiers = 0;

        // Add Declaration modifier for keys/properties
        if self.in_key_context && is_ident {
            modifiers |= SemanticTokenModifier::Declaration.bitmask();
        }

        // Add Definition modifier for section header identifiers
        if self.in_section_header && is_ident {
            modifiers |= SemanticTokenModifier::Definition.bitmask();
        }

        // Add SectionHeader modifier for all tokens in section header
        if self.in_section_header {
            modifiers |= SemanticTokenModifier::SectionHeader.bitmask();
        }

        modifiers
    }

    /// Map terminal kind to semantic token type, considering context.
    fn terminal_to_token_type(&self, kind: TerminalKind) -> Option<SemanticTokenType> {
        match kind {
            // Keywords (but not in key context - see visit_key_ident)
            // Hole: `!` or `!label`
            TerminalKind::Hole => Some(SemanticTokenType::Keyword),
            TerminalKind::True | TerminalKind::False | TerminalKind::Null => {
                // In key context, these are properties; otherwise keywords
                if self.in_key_context {
                    Some(SemanticTokenType::Property)
                } else {
                    Some(SemanticTokenType::Keyword)
                }
            }

            // Numbers
            TerminalKind::Integer | TerminalKind::Float | TerminalKind::Inf | TerminalKind::NaN => {
                Some(SemanticTokenType::Number)
            }

            // Strings (escaped and literal)
            TerminalKind::Str | TerminalKind::Text | TerminalKind::LitStr => {
                Some(SemanticTokenType::String)
            }

            // Comments
            TerminalKind::LineComment | TerminalKind::BlockComment => {
                Some(SemanticTokenType::Comment)
            }

            // Section marker
            TerminalKind::At => Some(SemanticTokenType::SectionMarker),

            // Extension marker
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
                if self.in_extension_namespace {
                    Some(SemanticTokenType::ExtensionIdent)
                } else if self.in_key_context {
                    Some(SemanticTokenType::Property)
                } else {
                    None
                }
            }

            // Code block content and ends
            TerminalKind::CodeBlockEnd3
            | TerminalKind::CodeBlockEnd4
            | TerminalKind::CodeBlockEnd5
            | TerminalKind::CodeBlockEnd6
            | TerminalKind::NoBacktick
            | TerminalKind::BacktickDelim1
            | TerminalKind::Backtick2
            | TerminalKind::Backtick3
            | TerminalKind::Backtick4
            | TerminalKind::Backtick5
            | TerminalKind::DelimCodeEnd1
            | TerminalKind::DelimCodeEnd2
            | TerminalKind::DelimCodeEnd3 => Some(SemanticTokenType::Macro),

            // Delimited escaped string content and ends (Str1/2/3)
            TerminalKind::NoQuote
            | TerminalKind::Quote1
            | TerminalKind::Quote2
            | TerminalKind::Str1End
            | TerminalKind::Str2End
            | TerminalKind::Str3End
            // Delimited literal string content and ends (LitStr1/2/3)
            | TerminalKind::NoSQuote
            | TerminalKind::SQuote1
            | TerminalKind::SQuote2
            | TerminalKind::LitStr1End
            | TerminalKind::LitStr2End
            | TerminalKind::LitStr3End => Some(SemanticTokenType::String),

            // Skip whitespace/newlines
            TerminalKind::Whitespace
            | TerminalKind::NewLine
            | TerminalKind::GrammarNewline
            | TerminalKind::Ws => None,

            // Skip code block/inline starts (handled specially via visit methods)
            TerminalKind::InlineCode1
            | TerminalKind::CodeBlockStart3
            | TerminalKind::CodeBlockStart4
            | TerminalKind::CodeBlockStart5
            | TerminalKind::CodeBlockStart6
            | TerminalKind::Str1Start
            | TerminalKind::Str2Start
            | TerminalKind::Str3Start
            | TerminalKind::LitStr1Start
            | TerminalKind::LitStr2Start
            | TerminalKind::LitStr3Start
            | TerminalKind::DelimCodeStart1
            | TerminalKind::DelimCodeStart2
            | TerminalKind::DelimCodeStart3 => None,
        }
    }
}

impl<F: CstFacade> CstVisitor<F> for SemanticTokenVisitor<'_> {
    type Error = std::convert::Infallible;

    fn then_construct_error(
        &mut self,
        node_data: Option<CstNode>,
        parent: CstNodeId,
        kind: NodeKind,
        _error: CstConstructError,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.recover_error(node_data, parent, kind, tree)
    }

    fn visit_terminal(
        &mut self,
        _id: CstNodeId,
        kind: TerminalKind,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        let TerminalData::Input(span) = data else {
            return Ok(());
        };

        let Some(token_type) = self.terminal_to_token_type(kind) else {
            return Ok(());
        };

        let is_ident = matches!(
            kind,
            TerminalKind::Ident | TerminalKind::True | TerminalKind::False | TerminalKind::Null
        );
        let modifiers = self.current_modifiers(is_ident);

        self.emit_token_with_modifiers(span, token_type, modifiers);
        Ok(())
    }

    // Context: Section header (@ keys body)
    fn visit_section(
        &mut self,
        _handle: SectionHandle,
        view: SectionView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Set section header context for @ and keys
        self.in_section_header = true;

        // Visit @ marker
        self.visit_at_handle(view.at, tree)?;

        // Visit keys (already sets key context via visit_keys)
        self.visit_keys_handle(view.keys, tree)?;

        // Reset section header before visiting body
        self.in_section_header = false;

        // Visit section body
        self.visit_section_body_handle(view.section_body, tree)?;

        Ok(())
    }

    // Context: Keys (key context)
    fn visit_keys(
        &mut self,
        handle: KeysHandle,
        view: KeysView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        let prev = self.in_key_context;
        self.in_key_context = true;
        self.visit_keys_super(handle, view, tree)?;
        self.in_key_context = prev;
        Ok(())
    }

    // Context: Extension namespace ($ident)
    fn visit_extension_name_space(
        &mut self,
        handle: ExtensionNameSpaceHandle,
        view: ExtensionNameSpaceView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        let prev = self.in_extension_namespace;
        self.in_extension_namespace = true;
        self.visit_extension_name_space_super(handle, view, tree)?;
        self.in_extension_namespace = prev;
        Ok(())
    }

    // Context: Value resets key context
    fn visit_value(
        &mut self,
        handle: ValueHandle,
        view: ValueView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        let prev = self.in_key_context;
        self.in_key_context = false;
        self.visit_value_super(handle, view, tree)?;
        self.in_key_context = prev;
        Ok(())
    }

    // Handle inline code 1 specially (lang`code`)
    fn visit_inline_code_1(
        &mut self,
        _handle: InlineCode1Handle,
        view: InlineCode1View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        if let Ok(TerminalData::Input(span)) = view.inline_code_1.get_data(tree) {
            self.emit_inline_code_1(span);
        }
        Ok(())
    }

    // Handle code block 3 specially (```lang)
    fn visit_code_block_3(
        &mut self,
        handle: CodeBlock3Handle,
        view: CodeBlock3View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        if let Ok(start_view) = view.code_block_start_3.get_view(tree)
            && let Ok(TerminalData::Input(span)) = start_view.code_block_start_3.get_data(tree)
        {
            self.emit_code_block_start(span, 3);
        }
        self.visit_code_block_3_list_handle(view.code_block_3_list, tree)?;
        self.visit_code_block_end_3_handle(view.code_block_end_3, tree)?;
        let _ = handle;
        Ok(())
    }

    // Handle code block 4 specially (````lang)
    fn visit_code_block_4(
        &mut self,
        handle: CodeBlock4Handle,
        view: CodeBlock4View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        if let Ok(start_view) = view.code_block_start_4.get_view(tree)
            && let Ok(TerminalData::Input(span)) = start_view.code_block_start_4.get_data(tree)
        {
            self.emit_code_block_start(span, 4);
        }
        self.visit_code_block_4_list_handle(view.code_block_4_list, tree)?;
        self.visit_code_block_end_4_handle(view.code_block_end_4, tree)?;
        let _ = handle;
        Ok(())
    }

    // Handle code block 5 specially (`````lang)
    fn visit_code_block_5(
        &mut self,
        handle: CodeBlock5Handle,
        view: CodeBlock5View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        if let Ok(start_view) = view.code_block_start_5.get_view(tree)
            && let Ok(TerminalData::Input(span)) = start_view.code_block_start_5.get_data(tree)
        {
            self.emit_code_block_start(span, 5);
        }
        self.visit_code_block_5_list_handle(view.code_block_5_list, tree)?;
        self.visit_code_block_end_5_handle(view.code_block_end_5, tree)?;
        let _ = handle;
        Ok(())
    }

    // Handle code block 6 specially (``````lang)
    fn visit_code_block_6(
        &mut self,
        handle: CodeBlock6Handle,
        view: CodeBlock6View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        if let Ok(start_view) = view.code_block_start_6.get_view(tree)
            && let Ok(TerminalData::Input(span)) = start_view.code_block_start_6.get_data(tree)
        {
            self.emit_code_block_start(span, 6);
        }
        self.visit_code_block_6_list_handle(view.code_block_6_list, tree)?;
        self.visit_code_block_end_6_handle(view.code_block_end_6, tree)?;
        let _ = handle;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_and_get_tokens(input: &str) -> Vec<SemanticToken> {
        let cst = crate::parol::parse(input).expect("Failed to parse");
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

    #[test]
    fn test_section_header_keyident_true_false_null() {
        // In section headers, true/false/null are KeyIdent (identifiers), not keyword literals.
        // According to grammar: KeyIdent: Ident | True | False | Null ;
        let input = "@true.false.null\nkey = 1";
        let tokens = parse_and_get_tokens(input);

        // true in section header should be Property, not Keyword
        let true_token = tokens
            .iter()
            .find(|t| {
                let text = &input[t.start as usize..(t.start + t.length) as usize];
                text == "true"
            })
            .unwrap();
        assert_eq!(
            true_token.token_type,
            SemanticTokenType::Property,
            "true in section header should be Property, not Keyword"
        );

        // false in section header should be Property, not Keyword
        let false_token = tokens
            .iter()
            .find(|t| {
                let text = &input[t.start as usize..(t.start + t.length) as usize];
                text == "false"
            })
            .unwrap();
        assert_eq!(
            false_token.token_type,
            SemanticTokenType::Property,
            "false in section header should be Property, not Keyword"
        );

        // null in section header should be Property, not Keyword
        let null_token = tokens
            .iter()
            .find(|t| {
                let text = &input[t.start as usize..(t.start + t.length) as usize];
                text == "null"
            })
            .unwrap();
        assert_eq!(
            null_token.token_type,
            SemanticTokenType::Property,
            "null in section header should be Property, not Keyword"
        );

        // All three should also have the SectionHeader modifier
        let section_header_mask = SemanticTokenModifier::SectionHeader.bitmask();
        assert!(
            true_token.modifiers & section_header_mask != 0,
            "true should have SectionHeader modifier"
        );
        assert!(
            false_token.modifiers & section_header_mask != 0,
            "false should have SectionHeader modifier"
        );
        assert!(
            null_token.modifiers & section_header_mask != 0,
            "null should have SectionHeader modifier"
        );
    }
}
