use eure_tree::tree::InputSpan; // Added import
use eure_tree::{prelude::*, tree::TerminalHandle};
use eure_value::{
    code::Code,
    PrimitiveValue,
    document::{EureDocument, InsertError, constructor::DocumentConstructor},
    path::PathSegment,
};
use num_bigint::BigInt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DocumentConstructionError {
    #[error(transparent)]
    CstError(#[from] CstConstructError),
    #[error("Invalid identifier: {0}")]
    InvalidIdentifier(String),
    #[error("Failed to parse integer: {0}")]
    InvalidInteger(String),
    #[error("Failed to parse float: {0}")]
    InvalidFloat(String),
    #[error("Document insert error: {0}")]
    DocumentInsert(#[from] InsertError),
    #[error("Unprocessed segments: {segments:?}")]
    UnprocessedSegments { segments: Vec<PathSegment> },
    #[error("Dynamic token not found: {0:?}")]
    DynamicTokenNotFound(DynamicTokenId),
    #[error("Failed to parse big integer: {0}")]
    InvalidBigInt(String),
    #[error("Invalid inline code: {0}")]
    InvalidInlineCode(String),
    #[error("Invalid code block: {0}")]
    InvalidCodeBlock(String),
}

#[derive(Debug, Clone, Default)]
struct TerminalTokens {
    terminals: Vec<TerminalToken>,
}

#[derive(Debug, Clone)] // Added derive
enum TerminalToken {
    Input(InputSpan),
    Dynamic(DynamicTokenId),
}

impl TerminalTokens {
    pub fn new() -> Self {
        Self {
            terminals: Vec::new(),
        }
    }

    pub fn push_terminal(&mut self, token: TerminalData) {
        let new_token = match (self.terminals.last_mut(), token) {
            (Some(TerminalToken::Input(span)), TerminalData::Input(input_span))
                if span.end == input_span.start =>
            {
                span.end = input_span.end;
                return;
            }
            (_, TerminalData::Dynamic(id)) => TerminalToken::Dynamic(id),
            (_, TerminalData::Input(input_span)) => TerminalToken::Input(input_span),
        };
        self.terminals.push(new_token);
    }

    pub fn into_string(
        self,
        input: &str,
        cst: &impl CstFacade,
    ) -> Result<String, DocumentConstructionError> {
        let mut string = String::new();
        for token in self.terminals {
            match token {
                TerminalToken::Input(span) => {
                    string.push_str(&input[span.start as usize..span.end as usize])
                }
                TerminalToken::Dynamic(id) => {
                    let str = cst
                        .dynamic_token(id)
                        .ok_or(DocumentConstructionError::DynamicTokenNotFound(id))?;
                    string.push_str(str);
                }
            }
        }
        Ok(string)
    }
}

struct CodeStart {
    /// Number of start backticks for asserting on pop
    backticks: u8,
    language: Option<String>,
    terminals: TerminalTokens,
}

impl CodeStart {
    fn new(backticks: u8, language: Option<String>) -> Self {
        Self {
            backticks,
            language,
            terminals: TerminalTokens::new(),
        }
    }
}

pub struct ValueVisitor<'a> {
    input: &'a str,
    // Main document being built
    document: DocumentConstructor,
    segments: Vec<PathSegment>,
    code_start: Option<CodeStart>,
}

impl<'a> ValueVisitor<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            document: DocumentConstructor::new(),
            segments: vec![],
            code_start: None,
        }
    }

    /// Parse language from InlineCode1 token: [lang]`content`
    /// Grammar: /[a-zA-Z0-9-_]*`[^`\r\n]*`/
    /// Language tag must be alphanumeric/hyphen/underscore only, no whitespace
    fn parse_inline_code_1(token: &str) -> Result<(Option<String>, String), DocumentConstructionError> {
        // Find the first backtick
        let first_backtick = token.find('`')
            .ok_or_else(|| DocumentConstructionError::InvalidInlineCode(
                format!("Missing opening backtick in: {}", token)
            ))?;

        let language_part = &token[..first_backtick];
        let language = if language_part.is_empty() {
            None
        } else {
            Some(language_part.to_string())
        };

        // Extract content between backticks
        let last_backtick = token.rfind('`')
            .ok_or_else(|| DocumentConstructionError::InvalidInlineCode(
                format!("Missing closing backtick in: {}", token)
            ))?;

        if last_backtick <= first_backtick {
            return Err(DocumentConstructionError::InvalidInlineCode(
                format!("Invalid backtick structure in: {}", token)
            ));
        }

        let content = token[first_backtick + 1..last_backtick].to_string();
        Ok((language, content))
    }

    /// Parse language from InlineCodeStart2 token: [lang]``
    /// Grammar: /[a-zA-Z0-9-_]*``/
    /// Language tag must be alphanumeric/hyphen/underscore only, no whitespace
    fn parse_inline_code_start_2(token: &str) -> Result<Option<String>, DocumentConstructionError> {
        // Remove the trailing ``
        let idx = token.find("``")
            .ok_or_else(|| DocumentConstructionError::InvalidInlineCode(
                format!("Missing double backticks in: {}", token)
            ))?;

        let language_part = &token[..idx];
        if language_part.is_empty() {
            Ok(None)
        } else {
            Ok(Some(language_part.to_string()))
        }
    }

    /// Parse language from CodeBlockStart token: ```[lang]\n or ````[lang]\n etc.
    /// Grammar: /`{n}[a-zA-Z0-9-_]*[\s--\r\n]*(\r\n|\r|\n)/
    /// Language tag must be alphanumeric/hyphen/underscore only, followed by optional whitespace
    fn parse_code_block_start(token: &str, backticks: u8) -> Result<Option<String>, DocumentConstructionError> {
        // Skip the backticks at the start
        let after_backticks = &token[backticks as usize..];

        // Find the newline
        let newline_idx = after_backticks.find(|c| c == '\n' || c == '\r')
            .ok_or_else(|| DocumentConstructionError::InvalidCodeBlock(
                format!("Missing newline after code block start in: {}", token)
            ))?;

        // Extract the part before newline
        let before_newline = &after_backticks[..newline_idx];

        // Find where the language tag ends (first non-alphanumeric/hyphen/underscore char)
        let lang_end = before_newline
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
            .unwrap_or(before_newline.len());

        let language_part = &before_newline[..lang_end];

        if language_part.is_empty() {
            Ok(None)
        } else {
            Ok(Some(language_part.to_string()))
        }
    }

    pub fn into_document(self) -> EureDocument {
        self.document.finish()
    }

    fn collect_path_segments(
        &mut self,
        visit: impl FnOnce(&mut Self) -> Result<(), DocumentConstructionError>,
    ) -> Result<Vec<PathSegment>, DocumentConstructionError> {
        if !self.segments.is_empty() {
            return Err(DocumentConstructionError::UnprocessedSegments {
                segments: self.segments.clone(),
            });
        }
        visit(self)?;
        let segments = std::mem::take(&mut self.segments);
        Ok(segments)
    }

    fn get_terminal_str<T: TerminalHandle>(
        &'a self,
        tree: &'a impl CstFacade,
        handle: T,
    ) -> Result<&'a str, DocumentConstructionError> {
        match tree.get_terminal_str(self.input, handle)? {
            Ok(str) => Ok(str),
            Err(id) => Err(DocumentConstructionError::DynamicTokenNotFound(id)),
        }
    }
}

impl<F: CstFacade> CstVisitor<F> for ValueVisitor<'_> {
    type Error = DocumentConstructionError;

    fn visit_null(
        &mut self,
        _handle: NullHandle,
        _view: NullView,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.document.bind_primitive(PrimitiveValue::Null)?;
        Ok(())
    }

    fn visit_true(
        &mut self,
        _handle: TrueHandle,
        _view: TrueView,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.document.bind_primitive(PrimitiveValue::Bool(true))?;
        Ok(())
    }

    fn visit_false(
        &mut self,
        _handle: FalseHandle,
        _view: FalseView,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.document.bind_primitive(PrimitiveValue::Bool(false))?;
        Ok(())
    }

    fn visit_integer(
        &mut self,
        _handle: IntegerHandle,
        view: IntegerView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        let str = self.get_terminal_str(tree, view.integer)?;

        let big_int: BigInt = str
            .parse()
            .map_err(|_| DocumentConstructionError::InvalidBigInt(str.to_string()))?;

        self.document
            .bind_primitive(PrimitiveValue::BigInt(big_int))?;
        Ok(())
    }

    fn visit_inline_code_1(
        &mut self,
        _handle: InlineCode1Handle,
        view: InlineCode1View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        let token_str = self.get_terminal_str(tree, view.inline_code_1)?;
        let (language, content) = Self::parse_inline_code_1(token_str)?;
        let code = Code::new_inline(content);
        let code_with_lang = if language.is_some() {
            Code {
                language,
                ..code
            }
        } else {
            code
        };
        self.document.bind_primitive(PrimitiveValue::Code(code_with_lang))?;
        Ok(())
    }

    fn visit_inline_code_start_2(
        &mut self,
        _handle: InlineCodeStart2Handle,
        view: InlineCodeStart2View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        let token_str = self.get_terminal_str(tree, view.inline_code_start_2)?;
        let language = Self::parse_inline_code_start_2(token_str)?;
        self.code_start = Some(CodeStart::new(2, language));
        Ok(())
    }

    fn visit_inline_code_end_2(
        &mut self,
        _handle: InlineCodeEnd2Handle,
        _view: InlineCodeEnd2View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        if let Some(code_start) = self.code_start.take() {
            let content = code_start.terminals.into_string(self.input, tree)?;
            let code = Code::new_inline(content);
            let code_with_lang = if code_start.language.is_some() {
                Code {
                    language: code_start.language,
                    ..code
                }
            } else {
                code
            };
            self.document.bind_primitive(PrimitiveValue::Code(code_with_lang))?;
        }
        Ok(())
    }

    fn visit_code_block_start_3(
        &mut self,
        _handle: CodeBlockStart3Handle,
        view: CodeBlockStart3View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        let token_str = self.get_terminal_str(tree, view.code_block_start_3)?;
        let language = Self::parse_code_block_start(token_str, 3)?;
        self.code_start = Some(CodeStart::new(3, language));
        Ok(())
    }

    fn visit_code_block_end_3(
        &mut self,
        _handle: CodeBlockEnd3Handle,
        _view: CodeBlockEnd3View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        if let Some(code_start) = self.code_start.take() {
            let content = code_start.terminals.into_string(self.input, tree)?;
            let code = Code::new_block(code_start.language, content);
            self.document.bind_primitive(PrimitiveValue::CodeBlock(code))?;
        }
        Ok(())
    }

    fn visit_code_block_start_4(
        &mut self,
        _handle: CodeBlockStart4Handle,
        view: CodeBlockStart4View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        let token_str = self.get_terminal_str(tree, view.code_block_start_4)?;
        let language = Self::parse_code_block_start(token_str, 4)?;
        self.code_start = Some(CodeStart::new(4, language));
        Ok(())
    }

    fn visit_code_block_end_4(
        &mut self,
        _handle: CodeBlockEnd4Handle,
        _view: CodeBlockEnd4View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        if let Some(code_start) = self.code_start.take() {
            let content = code_start.terminals.into_string(self.input, tree)?;
            let code = Code::new_block(code_start.language, content);
            self.document.bind_primitive(PrimitiveValue::CodeBlock(code))?;
        }
        Ok(())
    }

    fn visit_code_block_start_5(
        &mut self,
        _handle: CodeBlockStart5Handle,
        view: CodeBlockStart5View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        let token_str = self.get_terminal_str(tree, view.code_block_start_5)?;
        let language = Self::parse_code_block_start(token_str, 5)?;
        self.code_start = Some(CodeStart::new(5, language));
        Ok(())
    }

    fn visit_code_block_end_5(
        &mut self,
        _handle: CodeBlockEnd5Handle,
        _view: CodeBlockEnd5View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        if let Some(code_start) = self.code_start.take() {
            let content = code_start.terminals.into_string(self.input, tree)?;
            let code = Code::new_block(code_start.language, content);
            self.document.bind_primitive(PrimitiveValue::CodeBlock(code))?;
        }
        Ok(())
    }

    fn visit_code_block_start_6(
        &mut self,
        _handle: CodeBlockStart6Handle,
        view: CodeBlockStart6View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        let token_str = self.get_terminal_str(tree, view.code_block_start_6)?;
        let language = Self::parse_code_block_start(token_str, 6)?;
        self.code_start = Some(CodeStart::new(6, language));
        Ok(())
    }

    fn visit_code_block_end_6(
        &mut self,
        _handle: CodeBlockEnd6Handle,
        _view: CodeBlockEnd6View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        if let Some(code_start) = self.code_start.take() {
            let content = code_start.terminals.into_string(self.input, tree)?;
            let code = Code::new_block(code_start.language, content);
            self.document.bind_primitive(PrimitiveValue::CodeBlock(code))?;
        }
        Ok(())
    }

    fn visit_terminal(
        &mut self,
        _id: CstNodeId,
        _kind: TerminalKind,
        data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        // If we're inside a code block or inline code, collect the terminals
        if let Some(code_start) = &mut self.code_start {
            code_start.terminals.push_terminal(data);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eure_tree::tree::{ConcreteSyntaxTree, CstNodeData, InputSpan, TerminalData};

    fn create_dummy_cst() -> ConcreteSyntaxTree<TerminalKind, NonTerminalKind> {
        let root_data = CstNodeData::new_non_terminal(
            NonTerminalKind::Root,
            NonTerminalData::Input(InputSpan::EMPTY),
        );
        ConcreteSyntaxTree::new(root_data)
    }

    // Tests for parse_inline_code_1
    mod parse_inline_code_1_tests {
        use super::*;

        #[test]
        fn test_simple_code_without_language() {
            let result = ValueVisitor::parse_inline_code_1("`hello`");
            assert!(result.is_ok());
            let (language, content) = result.unwrap();
            assert_eq!(language, None);
            assert_eq!(content, "hello");
        }

        #[test]
        fn test_code_with_language() {
            let result = ValueVisitor::parse_inline_code_1("rust`fn main() {}`");
            assert!(result.is_ok());
            let (language, content) = result.unwrap();
            assert_eq!(language, Some("rust".to_string()));
            assert_eq!(content, "fn main() {}");
        }

        #[test]
        fn test_empty_code() {
            let result = ValueVisitor::parse_inline_code_1("``");
            assert!(result.is_ok());
            let (language, content) = result.unwrap();
            assert_eq!(language, None);
            assert_eq!(content, "");
        }

        #[test]
        fn test_code_with_special_chars() {
            let result = ValueVisitor::parse_inline_code_1("`hello world!@#$%`");
            assert!(result.is_ok());
            let (language, content) = result.unwrap();
            assert_eq!(language, None);
            assert_eq!(content, "hello world!@#$%");
        }

        #[test]
        fn test_language_with_hyphen_and_underscore() {
            let result = ValueVisitor::parse_inline_code_1("foo-bar_123`content`");
            assert!(result.is_ok());
            let (language, content) = result.unwrap();
            assert_eq!(language, Some("foo-bar_123".to_string()));
            assert_eq!(content, "content");
        }

        #[test]
        fn test_no_backticks() {
            let result = ValueVisitor::parse_inline_code_1("no backticks");
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                DocumentConstructionError::InvalidInlineCode(_)
            ));
        }

        #[test]
        fn test_single_backtick() {
            let result = ValueVisitor::parse_inline_code_1("`");
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                DocumentConstructionError::InvalidInlineCode(_)
            ));
        }
    }

    // Tests for parse_inline_code_start_2
    mod parse_inline_code_start_2_tests {
        use super::*;

        #[test]
        fn test_no_language() {
            let result = ValueVisitor::parse_inline_code_start_2("``");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), None);
        }

        #[test]
        fn test_with_language() {
            let result = ValueVisitor::parse_inline_code_start_2("rust``");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Some("rust".to_string()));
        }

        #[test]
        fn test_with_complex_language() {
            let result = ValueVisitor::parse_inline_code_start_2("foo-bar_123``");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Some("foo-bar_123".to_string()));
        }

        #[test]
        fn test_no_backticks() {
            let result = ValueVisitor::parse_inline_code_start_2("rust");
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                DocumentConstructionError::InvalidInlineCode(_)
            ));
        }
    }

    // Tests for parse_code_block_start
    mod parse_code_block_start_tests {
        use super::*;

        #[test]
        fn test_no_language_3_backticks() {
            let result = ValueVisitor::parse_code_block_start("```\n", 3);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), None);
        }

        #[test]
        fn test_with_language_3_backticks() {
            let result = ValueVisitor::parse_code_block_start("```rust\n", 3);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Some("rust".to_string()));
        }

        #[test]
        fn test_with_language_4_backticks() {
            let result = ValueVisitor::parse_code_block_start("````python\n", 4);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Some("python".to_string()));
        }

        #[test]
        fn test_with_language_5_backticks() {
            let result = ValueVisitor::parse_code_block_start("`````javascript\n", 5);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Some("javascript".to_string()));
        }

        #[test]
        fn test_with_language_6_backticks() {
            let result = ValueVisitor::parse_code_block_start("``````typescript\n", 6);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Some("typescript".to_string()));
        }

        #[test]
        fn test_language_with_trailing_whitespace() {
            let result = ValueVisitor::parse_code_block_start("```rust  \n", 3);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Some("rust".to_string()));
        }

        #[test]
        fn test_language_with_leading_whitespace_is_invalid() {
            // Leading whitespace before language tag is not allowed by grammar
            let result = ValueVisitor::parse_code_block_start("```  rust\n", 3);
            assert!(result.is_ok());
            // The language part should be empty because whitespace stops the language tag
            assert_eq!(result.unwrap(), None);
        }

        #[test]
        fn test_language_with_carriage_return() {
            let result = ValueVisitor::parse_code_block_start("```rust\r\n", 3);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Some("rust".to_string()));
        }

        #[test]
        fn test_language_with_only_carriage_return() {
            let result = ValueVisitor::parse_code_block_start("```rust\r", 3);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Some("rust".to_string()));
        }

        #[test]
        fn test_empty_language_with_spaces() {
            let result = ValueVisitor::parse_code_block_start("```   \n", 3);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), None);
        }

        #[test]
        fn test_no_newline() {
            let result = ValueVisitor::parse_code_block_start("```rust", 3);
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                DocumentConstructionError::InvalidCodeBlock(_)
            ));
        }

        #[test]
        fn test_complex_language_tag() {
            let result = ValueVisitor::parse_code_block_start("```foo-bar_123\n", 3);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Some("foo-bar_123".to_string()));
        }
    }

    #[test]
    fn test_push_input() {
        let mut tokens = TerminalTokens::new();
        let span = InputSpan::new(0, 5);
        tokens.push_terminal(TerminalData::Input(span));

        assert_eq!(tokens.terminals.len(), 1);
        match tokens.terminals[0] {
            TerminalToken::Input(s) => assert_eq!(s, span),
            _ => panic!("Expected Input token"),
        }
    }

    #[test]
    fn test_merge_adjacent_inputs() {
        let mut tokens = TerminalTokens::new();
        let span1 = InputSpan::new(0, 5);
        let span2 = InputSpan::new(5, 10);

        tokens.push_terminal(TerminalData::Input(span1));
        tokens.push_terminal(TerminalData::Input(span2));

        assert_eq!(tokens.terminals.len(), 1);
        match tokens.terminals[0] {
            TerminalToken::Input(s) => {
                assert_eq!(s.start, 0);
                assert_eq!(s.end, 10);
            }
            _ => panic!("Expected Input token"),
        }
    }

    #[test]
    fn test_dont_merge_non_adjacent() {
        let mut tokens = TerminalTokens::new();
        let span1 = InputSpan::new(0, 5);
        let span2 = InputSpan::new(6, 10); // Gap between 5 and 6

        tokens.push_terminal(TerminalData::Input(span1));
        tokens.push_terminal(TerminalData::Input(span2));

        assert_eq!(tokens.terminals.len(), 2);
        match tokens.terminals[0] {
            TerminalToken::Input(s) => assert_eq!(s, span1),
            _ => panic!("Expected Input token at 0"),
        }
        match tokens.terminals[1] {
            TerminalToken::Input(s) => assert_eq!(s, span2),
            _ => panic!("Expected Input token at 1"),
        }
    }

    #[test]
    fn test_dont_merge_dynamic() {
        let mut tokens = TerminalTokens::new();
        let span1 = InputSpan::new(0, 5);
        let id = DynamicTokenId(1);
        let span2 = InputSpan::new(5, 10);

        tokens.push_terminal(TerminalData::Input(span1));
        tokens.push_terminal(TerminalData::Dynamic(id));
        tokens.push_terminal(TerminalData::Input(span2));

        assert_eq!(tokens.terminals.len(), 3);
        match tokens.terminals[0] {
            TerminalToken::Input(s) => assert_eq!(s, span1),
            _ => panic!("Expected Input token at 0"),
        }
        match tokens.terminals[1] {
            TerminalToken::Dynamic(d) => assert_eq!(d, id),
            _ => panic!("Expected Dynamic token at 1"),
        }
        match tokens.terminals[2] {
            TerminalToken::Input(s) => assert_eq!(s, span2),
            _ => panic!("Expected Input token at 2"),
        }
    }

    #[test]
    fn test_into_string() {
        let mut cst = create_dummy_cst();
        let id = cst.insert_dynamic_terminal("world");

        let mut tokens = TerminalTokens::new();
        // "Hello "
        tokens.push_terminal(TerminalData::Input(InputSpan::new(0, 6)));
        // "world"
        tokens.push_terminal(TerminalData::Dynamic(id));
        // "!"
        tokens.push_terminal(TerminalData::Input(InputSpan::new(6, 7)));

        let input = "Hello !"; // Indices 0..6 is "Hello ", 6..7 is "!" (offset by dynamic token?)

        let result = tokens.into_string(input, &cst).expect("Should succeed");
        assert_eq!(result, "Hello world!");
    }

    #[test]
    fn test_into_string_missing_dynamic() {
        let cst = create_dummy_cst(); // Empty CST
        let id = DynamicTokenId(999); // Non-existent ID

        let mut tokens = TerminalTokens::new();
        tokens.push_terminal(TerminalData::Dynamic(id));

        let result = tokens.into_string("", &cst);
        assert!(matches!(
            result,
            Err(DocumentConstructionError::DynamicTokenNotFound(i)) if i == id
        ));
    }
}
