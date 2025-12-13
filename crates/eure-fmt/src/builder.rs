//! CST to Doc IR builder using CstVisitor.
//!
//! This module converts the concrete syntax tree into the Doc intermediate
//! representation for formatting, using the CstVisitor pattern to properly
//! handle trivia (comments, whitespace, newlines).

use std::convert::Infallible;

use eure_tree::prelude::*;
use eure_tree::tree::{LineNumbers, TerminalData};

use crate::config::FormatConfig;
use crate::doc::Doc;

/// Builder that converts CST to Doc IR using CstVisitor.
pub struct FormatBuilder<'a> {
    input: &'a str,
    config: &'a FormatConfig,
    line_numbers: LineNumbers<'a>,
}

impl<'a> FormatBuilder<'a> {
    /// Create a new format builder.
    pub fn new(input: &'a str, _tree: &'a Cst, config: &'a FormatConfig) -> Self {
        Self {
            input,
            config,
            line_numbers: LineNumbers::new(input),
        }
    }

    /// Build the Doc IR from the CST.
    pub fn build(&self, tree: &Cst) -> Doc {
        let mut visitor = FormatVisitor::new(self.input, self.config, &self.line_numbers);
        let _ = tree.visit_from_root(&mut visitor);
        visitor.finish()
    }
}

/// Visitor state for building Doc IR.
struct FormatVisitor<'a> {
    input: &'a str,
    config: &'a FormatConfig,
    line_numbers: &'a LineNumbers<'a>,
    /// Stack of document fragments being built
    docs: Vec<Doc>,
    /// Track if we've seen a newline since last content (for comment classification)
    seen_newline: bool,
    /// Track position of last content for trailing comment detection
    last_content_end: Option<u32>,
    /// Track if we're at the start (no content output yet)
    at_start: bool,
    /// Track if we just added a hardline (to avoid duplicates)
    pending_newline: bool,
    /// Context stack for nested structures (array, object, etc.)
    context: Vec<FormatContext>,
}

#[derive(Clone, Copy, PartialEq)]
enum FormatContext {
    Root,
    Eure,
    Binding,
    ValueBinding,
    Object,
    Array,
    Tuple,
    Section,
    Keys,
    CodeBlock,
    Text,
}

impl<'a> FormatVisitor<'a> {
    fn new(input: &'a str, config: &'a FormatConfig, line_numbers: &'a LineNumbers<'a>) -> Self {
        Self {
            input,
            config,
            line_numbers,
            docs: Vec::new(),
            seen_newline: true, // Start as if we're on a new line
            last_content_end: None,
            at_start: true,
            pending_newline: false,
            context: vec![FormatContext::Root],
        }
    }

    fn finish(mut self) -> Doc {
        // Ensure file ends with newline
        if !self.docs.is_empty() {
            self.docs.push(Doc::hardline());
        }
        Doc::concat_all(self.docs)
    }

    fn current_context(&self) -> FormatContext {
        self.context.last().copied().unwrap_or(FormatContext::Root)
    }

    fn push_context(&mut self, ctx: FormatContext) {
        self.context.push(ctx);
    }

    fn pop_context(&mut self) {
        self.context.pop();
    }

    fn get_text(&self, data: TerminalData) -> String {
        match data {
            TerminalData::Input(span) => {
                self.input[span.start as usize..span.end as usize].to_string()
            }
            TerminalData::Dynamic(_) => {
                // Dynamic tokens shouldn't occur in formatted output
                String::new()
            }
        }
    }

    /// Emit the text of a terminal directly
    fn emit_terminal(&mut self, data: TerminalData) {
        let text = self.get_text(data);
        if !text.is_empty() {
            self.flush_newline();
            self.docs.push(Doc::text(text));
            self.at_start = false;
        }
        self.mark_content(data);
    }

    fn get_span_end(&self, data: TerminalData) -> Option<u32> {
        match data {
            TerminalData::Input(span) => Some(span.end),
            _ => None,
        }
    }

    fn get_span_start(&self, data: TerminalData) -> Option<u32> {
        match data {
            TerminalData::Input(span) => Some(span.start),
            _ => None,
        }
    }

    fn same_line(&self, pos1: u32, pos2: u32) -> bool {
        self.line_numbers.get_char_info(pos1).line_number
            == self.line_numbers.get_char_info(pos2).line_number
    }

    /// Request a newline before the next content (deferred to avoid duplicates)
    fn request_newline(&mut self) {
        if !self.at_start {
            self.pending_newline = true;
        }
    }

    /// Flush any pending newline before emitting content
    fn flush_newline(&mut self) {
        if self.pending_newline {
            self.docs.push(Doc::hardline());
            self.pending_newline = false;
        }
    }

    /// Output a document fragment
    fn emit(&mut self, doc: Doc) {
        if !matches!(doc, Doc::Nil) {
            self.flush_newline();
            self.docs.push(doc);
            self.at_start = false;
        }
    }

    /// Output text
    fn emit_text(&mut self, text: impl Into<String>) {
        let text: String = text.into();
        if !text.is_empty() {
            self.flush_newline();
            self.docs.push(Doc::text(text));
            self.at_start = false;
        }
    }

    /// Handle a block comment terminal
    fn handle_comment(&mut self, data: TerminalData) {
        let text = self.get_text(data);
        let span_start = self.get_span_start(data);
        let span_end = self.get_span_end(data);

        // Check if this is a trailing comment (same line as previous content)
        let is_trailing = match (self.last_content_end, span_start) {
            (Some(prev_end), Some(start)) => self.same_line(prev_end, start),
            _ => false,
        };

        if is_trailing {
            // Trailing comment: space before, no newline after
            self.docs.push(Doc::text(" "));
            self.emit_text(text);
        } else {
            // Standalone comment: use emit_text to flush pending newline,
            // then request newline after for the next content
            self.emit_text(text);
            self.request_newline();
        }

        // Mark that we've seen content (for next item)
        self.last_content_end = span_end;
        self.seen_newline = false;
    }

    /// Mark that content was output at a position
    fn mark_content(&mut self, data: TerminalData) {
        self.last_content_end = self.get_span_end(data);
        self.seen_newline = false;
        self.at_start = false;
    }
}

impl<F: CstFacade> CstVisitor<F> for FormatVisitor<'_> {
    type Error = Infallible;

    // === Trivia handling ===

    fn visit_new_line_terminal(
        &mut self,
        _terminal: NewLine,
        _data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.seen_newline = true;
        Ok(())
    }

    fn visit_whitespace_terminal(
        &mut self,
        _terminal: Whitespace,
        _data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        // Skip whitespace - we add our own formatting
        Ok(())
    }

    fn visit_line_comment_terminal(
        &mut self,
        _terminal: LineComment,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        // Line comments include their trailing newline, which we need to handle specially
        let text = self.get_text(data);
        let span_start = self.get_span_start(data);
        let span_end = self.get_span_end(data);

        // Strip trailing newline from comment text (we handle newlines via Doc)
        let text = text.trim_end_matches(['\n', '\r']);

        // Check if this is a trailing comment (same line as previous content)
        let is_trailing = match (self.last_content_end, span_start) {
            (Some(prev_end), Some(start)) => self.same_line(prev_end, start),
            _ => false,
        };

        if is_trailing {
            // Trailing comment: space before, request newline after (for next content)
            self.docs.push(Doc::text(" "));
            self.emit_text(text);
            self.request_newline();
        } else {
            // Standalone comment: emit_text flushes pending newline,
            // then request newline after for the next content
            self.emit_text(text);
            self.request_newline();
        }

        self.last_content_end = span_end;
        self.seen_newline = true; // Line comment ends with newline semantically
        Ok(())
    }

    fn visit_block_comment_terminal(
        &mut self,
        _terminal: BlockComment,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.handle_comment(data);
        Ok(())
    }

    // === Structure handling ===

    fn visit_eure(
        &mut self,
        handle: EureHandle,
        view: EureView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.push_context(FormatContext::Eure);
        self.visit_eure_super(handle, view, tree)?;
        self.pop_context();
        Ok(())
    }

    fn visit_binding(
        &mut self,
        handle: BindingHandle,
        view: BindingView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Request newline before binding - will be flushed when first content is emitted
        // This allows trivia (comments) to be processed first and manage their own newlines
        self.request_newline();
        self.push_context(FormatContext::Binding);
        self.visit_binding_super(handle, view, tree)?;
        self.pop_context();
        Ok(())
    }

    fn visit_value_binding(
        &mut self,
        handle: ValueBindingHandle,
        view: ValueBindingView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.push_context(FormatContext::ValueBinding);
        self.visit_value_binding_super(handle, view, tree)?;
        self.pop_context();
        Ok(())
    }

    fn visit_section(
        &mut self,
        handle: SectionHandle,
        view: SectionView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Request newline before section - will be flushed when first content is emitted
        self.request_newline();
        self.push_context(FormatContext::Section);
        self.visit_section_super(handle, view, tree)?;
        self.pop_context();
        Ok(())
    }

    fn visit_keys(
        &mut self,
        handle: KeysHandle,
        view: KeysView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.push_context(FormatContext::Keys);
        self.visit_keys_super(handle, view, tree)?;
        self.pop_context();
        Ok(())
    }

    // === Terminal handling ===

    fn visit_bind_terminal(
        &mut self,
        _terminal: Bind,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        // Only add space before = if there's content before it (i.e., a key)
        if self.at_start {
            self.emit_text("= ");
        } else {
            self.emit_text(" = ");
        }
        self.mark_content(data);
        Ok(())
    }

    fn visit_at_terminal(
        &mut self,
        _terminal: At,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.emit_text("@ ");
        self.mark_content(data);
        Ok(())
    }

    fn visit_dot_terminal(
        &mut self,
        _terminal: Dot,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.emit_text(".");
        self.mark_content(data);
        Ok(())
    }

    fn visit_l_brace_terminal(
        &mut self,
        _terminal: LBrace,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.emit_text("{");
        self.mark_content(data);
        Ok(())
    }

    fn visit_r_brace_terminal(
        &mut self,
        _terminal: RBrace,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.emit_text("}");
        self.mark_content(data);
        Ok(())
    }

    fn visit_l_bracket_terminal(
        &mut self,
        _terminal: LBracket,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.emit_text("[");
        self.mark_content(data);
        Ok(())
    }

    fn visit_r_bracket_terminal(
        &mut self,
        _terminal: RBracket,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.emit_text("]");
        self.mark_content(data);
        Ok(())
    }

    fn visit_l_paren_terminal(
        &mut self,
        _terminal: LParen,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.emit_text("(");
        self.mark_content(data);
        Ok(())
    }

    fn visit_r_paren_terminal(
        &mut self,
        _terminal: RParen,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.emit_text(")");
        self.mark_content(data);
        Ok(())
    }

    fn visit_comma_terminal(
        &mut self,
        _terminal: Comma,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.emit_text(", ");
        self.mark_content(data);
        Ok(())
    }

    fn visit_map_bind_terminal(
        &mut self,
        _terminal: MapBind,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.emit_text(" => ");
        self.mark_content(data);
        Ok(())
    }

    fn visit_ident_terminal(
        &mut self,
        _terminal: Ident,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.emit_terminal(data);
        Ok(())
    }

    fn visit_integer_terminal(
        &mut self,
        _terminal: Integer,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.emit_terminal(data);
        Ok(())
    }

    fn visit_float_terminal(
        &mut self,
        _terminal: Float,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.emit_terminal(data);
        Ok(())
    }

    fn visit_true_terminal(
        &mut self,
        _terminal: True,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.emit_text("true");
        self.mark_content(data);
        Ok(())
    }

    fn visit_false_terminal(
        &mut self,
        _terminal: False,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.emit_text("false");
        self.mark_content(data);
        Ok(())
    }

    fn visit_null_terminal(
        &mut self,
        _terminal: Null,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.emit_text("null");
        self.mark_content(data);
        Ok(())
    }

    fn visit_hole_terminal(
        &mut self,
        _terminal: Hole,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.emit_terminal(data);
        Ok(())
    }

    fn visit_str_terminal(
        &mut self,
        _terminal: Str,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.emit_terminal(data);
        Ok(())
    }

    fn visit_hash_terminal(
        &mut self,
        _terminal: Hash,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.emit_text("#");
        self.mark_content(data);
        Ok(())
    }

    fn visit_esc_terminal(
        &mut self,
        _terminal: Esc,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.emit_text(" \\");
        self.emit(Doc::hardline());
        self.mark_content(data);
        Ok(())
    }

    // Code blocks - preserve verbatim
    fn visit_code_block_start_3_terminal(
        &mut self,
        _terminal: CodeBlockStart3,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        let text = self.get_text(data);
        self.emit_text(text);
        self.mark_content(data);
        Ok(())
    }

    fn visit_code_block_start_4_terminal(
        &mut self,
        _terminal: CodeBlockStart4,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        let text = self.get_text(data);
        self.emit_text(text);
        self.mark_content(data);
        Ok(())
    }

    fn visit_code_block_start_5_terminal(
        &mut self,
        _terminal: CodeBlockStart5,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        let text = self.get_text(data);
        self.emit_text(text);
        self.mark_content(data);
        Ok(())
    }

    fn visit_code_block_start_6_terminal(
        &mut self,
        _terminal: CodeBlockStart6,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        let text = self.get_text(data);
        self.emit_text(text);
        self.mark_content(data);
        Ok(())
    }

    fn visit_code_block_end_3_terminal(
        &mut self,
        _terminal: CodeBlockEnd3,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        let text = self.get_text(data);
        self.emit_text(text);
        self.mark_content(data);
        Ok(())
    }

    fn visit_code_block_end_4_terminal(
        &mut self,
        _terminal: CodeBlockEnd4,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        let text = self.get_text(data);
        self.emit_text(text);
        self.mark_content(data);
        Ok(())
    }

    fn visit_code_block_end_5_terminal(
        &mut self,
        _terminal: CodeBlockEnd5,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        let text = self.get_text(data);
        self.emit_text(text);
        self.mark_content(data);
        Ok(())
    }

    fn visit_code_block_end_6_terminal(
        &mut self,
        _terminal: CodeBlockEnd6,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        let text = self.get_text(data);
        self.emit_text(text);
        self.mark_content(data);
        Ok(())
    }

    fn visit_no_backtick_terminal(
        &mut self,
        _terminal: NoBacktick,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        let text = self.get_text(data);
        self.emit_text(text);
        self.mark_content(data);
        Ok(())
    }

    fn visit_backtick_2_terminal(
        &mut self,
        _terminal: Backtick2,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        let text = self.get_text(data);
        self.emit_text(text);
        self.mark_content(data);
        Ok(())
    }

    fn visit_backtick_3_terminal(
        &mut self,
        _terminal: Backtick3,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        let text = self.get_text(data);
        self.emit_text(text);
        self.mark_content(data);
        Ok(())
    }

    fn visit_backtick_4_terminal(
        &mut self,
        _terminal: Backtick4,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        let text = self.get_text(data);
        self.emit_text(text);
        self.mark_content(data);
        Ok(())
    }

    fn visit_backtick_5_terminal(
        &mut self,
        _terminal: Backtick5,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        let text = self.get_text(data);
        self.emit_text(text);
        self.mark_content(data);
        Ok(())
    }

    // Inline code
    fn visit_inline_code_1_terminal(
        &mut self,
        _terminal: InlineCode1,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        let text = self.get_text(data);
        self.emit_text(text);
        self.mark_content(data);
        Ok(())
    }

    fn visit_inline_code_start_2_terminal(
        &mut self,
        _terminal: InlineCodeStart2,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        let text = self.get_text(data);
        self.emit_text(text);
        self.mark_content(data);
        Ok(())
    }

    fn visit_inline_code_end_2_terminal(
        &mut self,
        _terminal: InlineCodeEnd2,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        let text = self.get_text(data);
        self.emit_text(text);
        self.mark_content(data);
        Ok(())
    }

    fn visit_no_backtick_inline_terminal(
        &mut self,
        _terminal: NoBacktickInline,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        let text = self.get_text(data);
        self.emit_text(text);
        self.mark_content(data);
        Ok(())
    }

    fn visit_backtick_1_terminal(
        &mut self,
        _terminal: Backtick1,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        let text = self.get_text(data);
        self.emit_text(text);
        self.mark_content(data);
        Ok(())
    }

    // Text binding
    fn visit_text_start_terminal(
        &mut self,
        _terminal: TextStart,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.emit_text(":");
        self.mark_content(data);
        Ok(())
    }

    fn visit_text_terminal(
        &mut self,
        _terminal: Text,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        let text = self.get_text(data);
        self.emit_text(text);
        self.mark_content(data);
        Ok(())
    }

    fn visit_grammar_newline_terminal(
        &mut self,
        _terminal: GrammarNewline,
        _data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        // This appears in text bindings, output as newline
        self.emit(Doc::hardline());
        self.seen_newline = true;
        Ok(())
    }

    fn visit_ws_terminal(
        &mut self,
        _terminal: Ws,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        // Whitespace in text contexts - preserve it
        let text = self.get_text(data);
        self.emit_text(text);
        self.mark_content(data);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn format(input: &str) -> String {
        format_width(input, 100)
    }

    fn format_width(input: &str, width: usize) -> String {
        let cst = eure_parol::parse(input).expect("parse failed");
        let config = FormatConfig::default().with_max_width(width);
        let builder = FormatBuilder::new(input, &cst, &config);
        let doc = builder.build(&cst);
        crate::printer::Printer::new(config).print(&doc)
    }

    #[test]
    fn test_simple_binding() {
        let input = "a = 1";
        let output = format(input);
        assert_eq!(output, "a = 1\n");
    }

    #[test]
    fn test_nested_binding() {
        let input = "a.b = 1";
        let output = format(input);
        assert_eq!(output, "a.b = 1\n");
    }

    #[test]
    fn test_array_binding() {
        let input = "= [1, 2, 3]";
        let output = format(input);
        assert_eq!(output, "= [1, 2, 3]\n");
    }

    #[test]
    fn test_object_binding() {
        let input = "= { b => 1 }";
        let output = format(input);
        assert_eq!(output, "= {b => 1}\n");
    }

    #[test]
    fn test_section() {
        let input = "@ foo\na = 1";
        let output = format(input);
        assert_eq!(output, "@ foo\na = 1\n");
    }

    #[test]
    fn test_empty_array() {
        let input = "= []";
        let output = format(input);
        assert_eq!(output, "= []\n");
    }

    #[test]
    fn test_empty_object() {
        let input = "= {}";
        let output = format(input);
        assert_eq!(output, "= {}\n");
    }

    #[test]
    fn test_object_inline_with_commas() {
        // Multiple object entries should have commas when inline
        let input = "= { a => 1, b => 2 }";
        let output = format(input);
        assert_eq!(output, "= {a => 1, b => 2}\n");
    }

    #[test]
    fn test_line_comment_standalone() {
        // Line comments on their own line should be preserved
        let input = "// this is a comment\na = 1";
        let output = format(input);
        assert_eq!(output, "// this is a comment\na = 1\n");
    }

    #[test]
    fn test_line_comment_trailing() {
        // Trailing comments should stay on the same line as the value
        let input = "a = 1 // trailing comment";
        let output = format(input);
        assert_eq!(output, "a = 1 // trailing comment\n");
    }

    #[test]
    fn test_block_comment() {
        // Block comments on their own line should be followed by newline
        let input = "/* block comment */\na = 1";
        let output = format(input);
        assert_eq!(output, "/* block comment */\na = 1\n");
    }

    #[test]
    fn test_comment_between_bindings() {
        // Comments between bindings should have proper newlines
        let input = "a = 1\n// comment\nb = 2";
        let output = format(input);
        assert_eq!(output, "a = 1\n// comment\nb = 2\n");
    }
}
