use eure_document::value::Tuple;
use eure_document::{
    document::{EureDocument, constructor::DocumentConstructor},
    identifier::Identifier,
    path::PathSegment,
    text::{Language, SyntaxHint, Text, TextParseError},
    value::ObjectKey,
    value::PrimitiveValue,
};
use eure_tree::tree::{InputSpan, RecursiveView};
use eure_tree::{prelude::*, tree::TerminalHandle};
use num_bigint::BigInt;
use regex::Regex;
use std::sync::LazyLock;

use crate::document::{CodeBlockError, DocumentConstructionError, InlineCodeError, OriginMap};

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

// Grammar: /[a-zA-Z0-9-_]*`[^`\r\n]*`/
static INLINE_CODE_1_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([a-zA-Z0-9_-]*)`([^`\r\n]*)`$").unwrap());

// Grammar: /[a-zA-Z0-9-_]*``/
static INLINE_CODE_START_2_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([a-zA-Z0-9_-]*)``$").unwrap());

// Grammar: /`{n}[a-zA-Z0-9-_]*[\s--\r\n]*(\r\n|\r|\n)/
// [\s--\r\n]* means whitespace except \r\n, i.e., [ \t]*
static CODE_BLOCK_START_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^`+([a-zA-Z0-9_-]*)[ \t]*(?:\r\n|\r|\n)$").unwrap());

/// Stored origin for code blocks and inline code 2
#[derive(Debug, Clone, Copy)]
enum CodeOrigin {
    InlineCode2(InlineCode2Handle),
    CodeBlock(CodeBlockHandle),
}

struct CodeStart {
    language: Language,
    syntax_hint: SyntaxHint,
    terminals: TerminalTokens,
    origin: Option<CodeOrigin>,
}

impl CodeStart {
    fn new(language: Language, syntax_hint: SyntaxHint) -> Self {
        Self {
            language,
            syntax_hint,
            terminals: TerminalTokens::new(),
            origin: None,
        }
    }

    fn with_origin(language: Language, syntax_hint: SyntaxHint, origin: CodeOrigin) -> Self {
        Self {
            language,
            syntax_hint,
            terminals: TerminalTokens::new(),
            origin: Some(origin),
        }
    }
}

pub struct CstInterpreter<'a> {
    input: &'a str,
    // Main document being built
    document: DocumentConstructor,
    code_start: Option<CodeStart>,
    // Stack for collecting ObjectKeys when processing KeyTuple
    collecting_object_keys: Vec<Vec<ObjectKey>>,
    // Origin tracking for error span resolution
    origins: OriginMap,
    // Pending code origin - set by parent visitor, used by start visitor
    pending_code_origin: Option<CodeOrigin>,
}

impl<'a> CstInterpreter<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            document: DocumentConstructor::new(),
            code_start: None,
            collecting_object_keys: vec![],
            origins: OriginMap::new(),
            pending_code_origin: None,
        }
    }

    /// Parse language from InlineCode1 token: [lang]`content`
    /// Grammar: /[a-zA-Z0-9-_]*`[^`\r\n]*`/
    /// Language tag must be alphanumeric/hyphen/underscore only, no whitespace
    fn parse_inline_code_1(token: &str) -> Result<(Language, String), InlineCodeError> {
        let caps = INLINE_CODE_1_REGEX
            .captures(token)
            .ok_or(InlineCodeError::InvalidInlineCode1Pattern)?;

        let lang = caps.get(1).unwrap().as_str();
        let content = caps.get(2).unwrap().as_str();

        let language = if lang.is_empty() {
            Language::Implicit
        } else {
            Language::new(lang.to_string())
        };
        Ok((language, content.to_string()))
    }

    /// Parse language from InlineCodeStart2 token: [lang]``
    /// Grammar: /[a-zA-Z0-9-_]*``/
    /// Language tag must be alphanumeric/hyphen/underscore only, no whitespace
    fn parse_inline_code_start_2(token: &str) -> Result<Language, InlineCodeError> {
        let caps = INLINE_CODE_START_2_REGEX
            .captures(token)
            .ok_or(InlineCodeError::InvalidInlineCodeStart2Pattern)?;

        let lang = caps.get(1).unwrap().as_str();
        let language = if lang.is_empty() {
            Language::Implicit
        } else {
            Language::new(lang.to_string())
        };
        Ok(language)
    }

    /// Parse language from CodeBlockStart token: ```[lang]\n or ````[lang]\n etc.
    /// Grammar: /`{n}[a-zA-Z0-9-_]*[\s--\r\n]*(\r\n|\r|\n)/
    /// Language tag must be alphanumeric/hyphen/underscore only, followed by optional whitespace (space/tab, not newlines)
    fn parse_code_block_start(token: &str) -> Result<Language, CodeBlockError> {
        let caps = CODE_BLOCK_START_REGEX
            .captures(token)
            .ok_or(CodeBlockError::InvalidCodeBlockStartPattern)?;

        let lang = caps.get(1).unwrap().as_str();
        let language = if lang.is_empty() {
            Language::Implicit
        } else {
            Language::new(lang.to_string())
        };
        Ok(language)
    }

    pub fn into_document(self) -> EureDocument {
        self.document.finish()
    }

    pub fn into_document_and_origin_map(self) -> (EureDocument, OriginMap) {
        (self.document.finish(), self.origins)
    }

    /// Record a definition span for a node (typically the key).
    fn record_definition(
        &mut self,
        node_id: eure_document::document::NodeId,
        cst_node_id: CstNodeId,
    ) {
        self.origins.record_definition(node_id, cst_node_id);
    }

    /// Record a value span for a node (the full expression).
    fn record_value(&mut self, node_id: eure_document::document::NodeId, cst_node_id: CstNodeId) {
        self.origins.record_value(node_id, cst_node_id);
    }

    /// Record a map key origin for precise error spans.
    fn record_key_origin(
        &mut self,
        map_node_id: eure_document::document::NodeId,
        key: ObjectKey,
        cst_node_id: CstNodeId,
    ) {
        self.origins.record_key(map_node_id, key, cst_node_id);
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

    /// Parse a Str terminal (with surrounding quotes) into a String
    fn parse_str_terminal(
        &self,
        str_handle: StrHandle,
        tree: &impl CstFacade,
    ) -> Result<String, DocumentConstructionError> {
        let str_view = str_handle.get_view(tree)?;
        let str_with_quotes = self.get_terminal_str(tree, str_view.str)?;

        // Remove surrounding quotes
        let str_content = str_with_quotes
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .ok_or_else(|| DocumentConstructionError::InvalidStringKey {
                node_id: str_handle.node_id(),
                error: TextParseError::InvalidEndOfStringAfterEscape,
            })?;

        // Parse the string content
        let text = Text::parse_quoted_string(str_content).map_err(|error| {
            DocumentConstructionError::InvalidStringKey {
                node_id: str_handle.node_id(),
                error,
            }
        })?;

        Ok(text.content)
    }

    fn get_key_ident_str(
        &'a self,
        tree: &'a impl CstFacade,
        ident_handle: KeyIdentHandle,
    ) -> Result<&'a str, DocumentConstructionError> {
        let ident_view = ident_handle.get_view(tree)?;
        let ident_str = match ident_view {
            KeyIdentView::Ident(ident_handle) => {
                self.get_terminal_str(tree, ident_handle.get_view(tree)?.ident)?
            }
            KeyIdentView::True(true_handle) => {
                self.get_terminal_str(tree, true_handle.get_view(tree)?.r#true)?
            }
            KeyIdentView::False(false_handle) => {
                self.get_terminal_str(tree, false_handle.get_view(tree)?.r#false)?
            }
            KeyIdentView::Null(null_handle) => {
                self.get_terminal_str(tree, null_handle.get_view(tree)?.r#null)?
            }
        };
        Ok(ident_str)
    }
}

impl<F: CstFacade> CstVisitor<F> for CstInterpreter<'_> {
    type Error = DocumentConstructionError;

    fn visit_eure(
        &mut self,
        handle: EureHandle,
        view: EureView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        let root_id = self.document.current_node_id();

        // Only record value span for the true document root (not for section bodies).
        // Section bodies get their value span from visit_section.
        if root_id == self.document.document().get_root_id() {
            self.record_value(root_id, handle.node_id());
        }

        // Visit children using the default super implementation
        self.visit_eure_super(handle, view, tree)?;
        // Check if Eure is truly empty (no ValueBinding, no Bindings, no Sections)
        let has_value_binding = view.eure_opt.get_view(tree)?.is_some();
        if self.document.current_node().content.is_hole() && !has_value_binding {
            self.document.bind_empty_map().map_err(|e| {
                DocumentConstructionError::DocumentInsert {
                    error: e,
                    node_id: handle.node_id(),
                }
            })?;
        }
        Ok(())
    }

    fn visit_object(
        &mut self,
        handle: ObjectHandle,
        view: ObjectView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Record the object container value span
        let container_id = self.document.current_node_id();
        self.record_value(container_id, handle.node_id());

        // Check if there's a value binding (new syntax: { = value, ... })
        let has_value_binding = if let Some(object_opt_view) = view.object_opt.get_view(tree)? {
            // Visit the value binding - this binds the main value
            self.visit_value_binding_handle(object_opt_view.value_binding, tree)?;
            true
        } else {
            false
        };

        // Process each entry in the ObjectList
        // Each entry has: keys => value
        // The keys can be nested (e.g., a.b => 1 becomes { a => { b => 1 } })
        if let Some(object_list_view) = view.object_list.get_view(tree)? {
            for item in object_list_view.get_all(tree)? {
                let scope = self.document.begin_scope();

                // Navigate through the keys
                self.visit_keys_handle(item.keys, tree)?;

                // Validate binding target is a Hole
                let node_id = self.document.current_node_id();
                self.document.require_hole().map_err(|e| {
                    DocumentConstructionError::DocumentInsert {
                        error: e,
                        node_id: handle.node_id(),
                    }
                })?;

                // Record value span for this object entry
                self.record_value(node_id, item.keys.node_id());

                // Visit the value
                self.visit_value_handle(item.value, tree)?;

                // Restore to the Object level
                self.document.end_scope(scope)?;
            }
        }

        if self.document.current_node().content.is_hole() && !has_value_binding {
            // Empty object (no value binding, no entries)
            self.document.bind_empty_map().map_err(|e| {
                DocumentConstructionError::DocumentInsert {
                    error: e,
                    node_id: handle.node_id(),
                }
            })?;
        }

        Ok(())
    }

    fn visit_array(
        &mut self,
        handle: ArrayHandle,
        view: ArrayView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Record the array container value span
        let container_id = self.document.current_node_id();
        self.record_value(container_id, handle.node_id());

        // Process array elements
        if let Some(elements_handle) = view.array_opt.get_view(tree)? {
            // Iterate through array elements
            let mut current = Some(elements_handle);
            let mut index = 0usize;

            while let Some(elem_handle) = current {
                let elem_view = elem_handle.get_view(tree)?;

                // Begin scope and navigate to array index
                let scope = self.document.begin_scope();
                let node_id = self
                    .document
                    .navigate(PathSegment::ArrayIndex(Some(index)))
                    .map_err(|e| DocumentConstructionError::DocumentInsert {
                        error: e,
                        node_id: handle.node_id(),
                    })?;

                // Record value span for this array element
                self.record_value(node_id, handle.node_id());

                // Visit the value at this index
                self.visit_value_handle(elem_view.value, tree)?;

                // End scope to return to array level
                self.document.end_scope(scope)?;

                // Move to next element if any
                current = if let Some(tail_handle) = elem_view.array_elements_opt.get_view(tree)? {
                    let tail_view = tail_handle.get_view(tree)?;
                    tail_view.array_elements_tail_opt.get_view(tree)?
                } else {
                    None
                };

                index += 1;
            }
        } else {
            self.document.bind_empty_array().map_err(|e| {
                DocumentConstructionError::DocumentInsert {
                    error: e,
                    node_id: handle.node_id(),
                }
            })?;
        }

        Ok(())
    }

    fn visit_tuple(
        &mut self,
        handle: TupleHandle,
        view: TupleView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Record the tuple container value span
        let container_id = self.document.current_node_id();
        self.record_value(container_id, handle.node_id());

        // Process tuple elements (similar to array but with TupleIndex path segment)
        if let Some(elements_handle) = view.tuple_opt.get_view(tree)? {
            // Iterate through tuple elements
            let mut current = Some(elements_handle);
            let mut index = 0u8;

            while let Some(elem_handle) = current {
                let elem_view = elem_handle.get_view(tree)?;

                // Begin scope and navigate to tuple index
                let scope = self.document.begin_scope();
                let node_id = self
                    .document
                    .navigate(PathSegment::TupleIndex(index))
                    .map_err(|e| DocumentConstructionError::DocumentInsert {
                        error: e,
                        node_id: handle.node_id(),
                    })?;

                // Record value span for this tuple element
                self.record_value(node_id, handle.node_id());

                // Visit the value at this index
                self.visit_value_handle(elem_view.value, tree)?;

                // End scope to return to tuple level
                self.document.end_scope(scope)?;

                // Move to next element if any
                current = if let Some(tail_handle) = elem_view.tuple_elements_opt.get_view(tree)? {
                    let tail_view = tail_handle.get_view(tree)?;
                    tail_view.tuple_elements_tail_opt.get_view(tree)?
                } else {
                    None
                };

                index = index.saturating_add(1);
            }
        } else {
            self.document.bind_empty_tuple().map_err(|e| {
                DocumentConstructionError::DocumentInsert {
                    error: e,
                    node_id: handle.node_id(),
                }
            })?;
        }

        Ok(())
    }

    fn visit_key(&mut self, handle: KeyHandle, view: KeyView, tree: &F) -> Result<(), Self::Error> {
        // 1. KeyBase から PathSegment を構築
        let key_base_view = view.key_base.get_view(tree)?;

        // Capture container node ID before navigation (for key origin tracking)
        let container_id = self.document.current_node_id();

        // Build segment and optionally capture key origin info for ObjectKey-based keys
        let (segment, key_origin_info) = match key_base_view {
            KeyBaseView::KeyIdent(ident_handle) => {
                let ident_str = self.get_key_ident_str(tree, ident_handle)?;
                let identifier: Identifier = ident_str.parse()?;
                // Record key origin as string key (identifiers become string keys in maps)
                let object_key = ObjectKey::String(ident_str.to_string());
                (
                    PathSegment::Ident(identifier),
                    Some((object_key, ident_handle.node_id())),
                )
            }
            KeyBaseView::ExtensionNameSpace(ext_handle) => {
                let ext_view = ext_handle.get_view(tree)?;
                let ident_str = self.get_key_ident_str(tree, ext_view.key_ident)?;
                let identifier: Identifier = ident_str.parse()?;
                (PathSegment::Extension(identifier), None)
            }
            KeyBaseView::Str(str_handle) => {
                let string = self.parse_str_terminal(str_handle, tree)?;
                let object_key = ObjectKey::String(string);
                (
                    PathSegment::Value(object_key.clone()),
                    Some((object_key, str_handle.node_id())),
                )
            }
            KeyBaseView::Integer(int_handle) => {
                let int_view = int_handle.get_view(tree)?;
                let str = self.get_terminal_str(tree, int_view.integer)?;
                let big_int: BigInt = str
                    .parse()
                    .map_err(|_| DocumentConstructionError::InvalidBigInt(str.to_string()))?;
                let object_key = ObjectKey::Number(big_int);
                (
                    PathSegment::Value(object_key.clone()),
                    Some((object_key, int_handle.node_id())),
                )
            }
            KeyBaseView::KeyTuple(tuple_handle) => {
                // Use visitor pattern to collect ObjectKeys
                self.collecting_object_keys.push(vec![]);
                self.visit_key_tuple_handle(tuple_handle, tree)?;
                let keys = self.collecting_object_keys.pop().expect(
                    "collecting_object_keys stack should not be empty after visiting KeyTuple",
                );
                let object_key = ObjectKey::Tuple(Tuple(keys));
                (
                    PathSegment::Value(object_key.clone()),
                    Some((object_key, tuple_handle.node_id())),
                )
            }
            KeyBaseView::TupleIndex(tuple_index_handle) => {
                let tuple_index_view = tuple_index_handle.get_view(tree)?;
                let int_view = tuple_index_view.integer.get_view(tree)?;
                let str = self.get_terminal_str(tree, int_view.integer)?;
                let length: u8 =
                    str.parse()
                        .map_err(|_| DocumentConstructionError::InvalidTupleIndex {
                            node_id: tuple_index_handle.node_id(),
                            value: str.to_string(),
                        })?;
                (PathSegment::TupleIndex(length), None)
            }
        };

        // Capture CST node ID before consuming key_origin_info
        let key_cst_node = key_origin_info.as_ref().map(|(_, id)| *id);

        // Record key origin for ObjectKey-based segments (for precise error spans)
        if let Some((object_key, key_cst_node_id)) = key_origin_info {
            self.record_key_origin(container_id, object_key, key_cst_node_id);
        }

        // 2. Navigate to this segment
        self.document
            .navigate(segment)
            .map_err(|e| DocumentConstructionError::DocumentInsert {
                error: e,
                node_id: handle.node_id(),
            })?;

        // Record definition span for the navigated node using the key's CST span
        // This ensures MissingRequiredField errors point to the defining key
        if let Some(cst_node_id) = key_cst_node {
            let child_id = self.document.current_node_id();
            self.record_definition(child_id, cst_node_id);
        }

        // 3. ArrayMarker の処理
        let key_opt_view = view.key_opt.get_view(tree)?;
        if let Some(array_marker_handle) = key_opt_view {
            let array_marker_view = array_marker_handle.get_view(tree)?;
            let index =
                if let Some(int_handle) = array_marker_view.array_marker_opt.get_view(tree)? {
                    let int_view = int_handle.get_view(tree)?;
                    let str = self.get_terminal_str(tree, int_view.integer)?;
                    let index: usize = str
                        .parse()
                        .map_err(|_| DocumentConstructionError::InvalidInteger(str.to_string()))?;
                    Some(index)
                } else {
                    None
                };
            self.document
                .navigate(PathSegment::ArrayIndex(index))
                .map_err(|e| DocumentConstructionError::DocumentInsert {
                    error: e,
                    node_id: handle.node_id(),
                })?;
        }

        Ok(())
    }

    fn visit_key_value(
        &mut self,
        _handle: KeyValueHandle,
        view: KeyValueView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Collect ObjectKey into the current collection stack
        let object_key = match view {
            KeyValueView::Integer(int_handle) => {
                let int_view = int_handle.get_view(tree)?;
                let str = self.get_terminal_str(tree, int_view.integer)?;
                let big_int: BigInt = str
                    .parse()
                    .map_err(|_| DocumentConstructionError::InvalidBigInt(str.to_string()))?;
                ObjectKey::Number(big_int)
            }
            KeyValueView::Boolean(bool_handle) => {
                let bool_view = bool_handle.get_view(tree)?;
                match bool_view {
                    BooleanView::True(_) => ObjectKey::String("true".to_string()),
                    BooleanView::False(_) => ObjectKey::String("false".to_string()),
                }
            }
            KeyValueView::Str(str_handle) => {
                let result = self.parse_str_terminal(str_handle, tree)?;
                ObjectKey::String(result)
            }
            KeyValueView::KeyTuple(tuple_handle) => {
                // Recursively handle nested tuple
                self.collecting_object_keys.push(vec![]);
                self.visit_key_tuple_handle(tuple_handle, tree)?;
                let keys = self.collecting_object_keys.pop().expect(
                    "collecting_object_keys stack should not be empty after visiting KeyTuple",
                );
                ObjectKey::Tuple(Tuple(keys))
            }
        };

        // Add to current collection
        self.collecting_object_keys
            .last_mut()
            .expect("collecting_object_keys stack should not be empty when visiting KeyValue")
            .push(object_key);

        Ok(())
    }

    fn visit_binding(
        &mut self,
        handle: BindingHandle,
        view: BindingView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.document.begin_binding();
        let scope = self.document.begin_scope();

        // Navigate through the keys
        self.visit_keys_handle(view.keys, tree)?;

        // Validate binding target is a Hole
        let node_id = self.document.current_node_id();
        self.document
            .require_hole()
            .map_err(|e| DocumentConstructionError::DocumentInsert {
                error: e,
                node_id: handle.node_id(),
            })?;

        self.record_value(node_id, handle.node_id());

        // Determine if this is a block binding (SectionBinding) or value binding
        let binding_rhs_view = view.binding_rhs.get_view(tree)?;
        let is_block = matches!(binding_rhs_view, BindingRhsView::SectionBinding(_));

        if is_block {
            self.document.begin_eure_block();
        }

        self.visit_binding_rhs_handle(view.binding_rhs, tree)?;

        if is_block {
            self.document.end_eure_block().map_err(|e| {
                DocumentConstructionError::DocumentInsert {
                    error: e,
                    node_id: handle.node_id(),
                }
            })?;
        }

        self.document.end_scope(scope)?;

        if is_block {
            self.document.end_binding_block().map_err(|e| {
                DocumentConstructionError::DocumentInsert {
                    error: e,
                    node_id: handle.node_id(),
                }
            })?;
        } else {
            self.document.end_binding_value().map_err(|e| {
                DocumentConstructionError::DocumentInsert {
                    error: e,
                    node_id: handle.node_id(),
                }
            })?;
        }
        Ok(())
    }

    fn visit_section(
        &mut self,
        handle: SectionHandle,
        view: SectionView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        let SectionView {
            at: _,
            keys,
            section_body,
        } = view;

        self.document.begin_section();
        let scope = self.document.begin_scope();

        // Navigate through the keys
        self.visit_keys_handle(keys, tree)?;

        // Validate section target is unbound
        let node_id = self.document.current_node_id();
        self.document
            .require_hole()
            .map_err(|e| DocumentConstructionError::DocumentInsert {
                error: e,
                node_id: handle.node_id(),
            })?;

        self.record_value(node_id, handle.node_id());

        // Determine if this is a block section (@ path { }) or item section (@ path)
        let section_body_view = section_body.get_view(tree)?;
        let is_block = matches!(section_body_view, SectionBodyView::Alt1(_));

        if is_block {
            self.document.begin_eure_block();
        } else {
            self.document.begin_section_items();
        }

        self.visit_section_body_handle(section_body, tree)?;

        if is_block {
            self.document.end_eure_block().map_err(|e| {
                DocumentConstructionError::DocumentInsert {
                    error: e,
                    node_id: handle.node_id(),
                }
            })?;
        }

        self.document.end_scope(scope)?;

        if is_block {
            self.document.end_section_block().map_err(|e| {
                DocumentConstructionError::DocumentInsert {
                    error: e,
                    node_id: handle.node_id(),
                }
            })?;
        } else {
            self.document.end_section_items().map_err(|e| {
                DocumentConstructionError::DocumentInsert {
                    error: e,
                    node_id: handle.node_id(),
                }
            })?;
        }
        Ok(())
    }

    fn visit_section_body(
        &mut self,
        handle: SectionBodyHandle,
        view: SectionBodyView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        match view {
            SectionBodyView::Alt0(alt0) => {
                // TOML-like section: `@ foo` followed by optional bindings
                // Check if section body is truly empty (no ValueBinding, no Bindings)
                let has_value_binding = alt0.section_body_opt.get_view(tree)?.is_some();
                let has_bindings = alt0.section_body_list.get_view(tree)?.is_some();
                let is_empty = !has_value_binding && !has_bindings;

                // Only convert Hole to empty map if section body was truly empty
                if is_empty {
                    self.document.bind_empty_map().map_err(|e| {
                        DocumentConstructionError::DocumentInsert {
                            error: e,
                            node_id: handle.node_id(),
                        }
                    })?;
                } else {
                    // Visit children using the default super implementation
                    self.visit_section_body_super(handle, view, tree)?;
                }
            }
            SectionBodyView::Alt1(_) => {
                // Block-style section: `@ foo { ... }`
                // visit_eure handles the empty case
                self.visit_section_body_super(handle, view, tree)?;
            }
        }
        Ok(())
    }

    fn visit_null(
        &mut self,
        handle: NullHandle,
        _view: NullView,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        let node_id = self.document.current_node_id();
        self.document
            .bind_primitive(PrimitiveValue::Null)
            .map_err(|e| DocumentConstructionError::DocumentInsert {
                error: e,
                node_id: handle.node_id(),
            })?;
        self.record_value(node_id, handle.node_id());
        Ok(())
    }

    fn visit_true(
        &mut self,
        handle: TrueHandle,
        _view: TrueView,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        let node_id = self.document.current_node_id();
        self.document
            .bind_primitive(PrimitiveValue::Bool(true))
            .map_err(|e| DocumentConstructionError::DocumentInsert {
                error: e,
                node_id: handle.node_id(),
            })?;
        self.record_value(node_id, handle.node_id());
        Ok(())
    }

    fn visit_false(
        &mut self,
        handle: FalseHandle,
        _view: FalseView,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        let node_id = self.document.current_node_id();
        self.document
            .bind_primitive(PrimitiveValue::Bool(false))
            .map_err(|e| DocumentConstructionError::DocumentInsert {
                error: e,
                node_id: handle.node_id(),
            })?;
        self.record_value(node_id, handle.node_id());
        Ok(())
    }

    fn visit_integer(
        &mut self,
        handle: IntegerHandle,
        view: IntegerView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        let str = self.get_terminal_str(tree, view.integer)?;

        // Remove underscores for parsing
        let clean_str = str.replace('_', "");

        let big_int: BigInt = clean_str
            .parse()
            .map_err(|_| DocumentConstructionError::InvalidBigInt(str.to_string()))?;

        let node_id = self.document.current_node_id();
        self.document
            .bind_primitive(PrimitiveValue::Integer(big_int))
            .map_err(|e| DocumentConstructionError::DocumentInsert {
                error: e,
                node_id: handle.node_id(),
            })?;
        self.record_value(node_id, handle.node_id());
        Ok(())
    }

    fn visit_inf(&mut self, handle: InfHandle, view: InfView, tree: &F) -> Result<(), Self::Error> {
        let str = self.get_terminal_str(tree, view.inf)?;

        let float = if str.starts_with('-') {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };

        let node_id = self.document.current_node_id();
        self.document
            .bind_primitive(PrimitiveValue::F64(float))
            .map_err(|e| DocumentConstructionError::DocumentInsert {
                error: e,
                node_id: handle.node_id(),
            })?;
        self.record_value(node_id, handle.node_id());
        Ok(())
    }

    fn visit_na_n(
        &mut self,
        handle: NaNHandle,
        _view: NaNView,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        let node_id = self.document.current_node_id();
        self.document
            .bind_primitive(PrimitiveValue::F64(f64::NAN))
            .map_err(|e| DocumentConstructionError::DocumentInsert {
                error: e,
                node_id: handle.node_id(),
            })?;
        self.record_value(node_id, handle.node_id());
        Ok(())
    }

    fn visit_float(
        &mut self,
        handle: FloatHandle,
        view: FloatView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        let str = self.get_terminal_str(tree, view.float)?;

        // Check for f32/f64 suffix
        let (num_str, is_f32) = if let Some(stripped) = str.strip_suffix("f32") {
            (stripped, true)
        } else if let Some(stripped) = str.strip_suffix("f64") {
            (stripped, false)
        } else {
            (str, false)
        };

        // Remove underscores for parsing
        let clean_str = num_str.replace('_', "");

        let node_id = self.document.current_node_id();
        let primitive = if is_f32 {
            let float: f32 = clean_str
                .parse()
                .map_err(|_| DocumentConstructionError::InvalidFloat(str.to_string()))?;
            PrimitiveValue::F32(float)
        } else {
            let float: f64 = clean_str
                .parse()
                .map_err(|_| DocumentConstructionError::InvalidFloat(str.to_string()))?;
            PrimitiveValue::F64(float)
        };

        self.document.bind_primitive(primitive).map_err(|e| {
            DocumentConstructionError::DocumentInsert {
                error: e,
                node_id: handle.node_id(),
            }
        })?;
        self.record_value(node_id, handle.node_id());
        Ok(())
    }

    fn visit_hole(
        &mut self,
        handle: HoleHandle,
        view: HoleView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Extract label from the hole token
        let token_str = self.get_terminal_str(tree, view.hole)?;
        let label = if token_str.len() > 1 {
            // Named hole: `!label` - skip '!' prefix and parse as Identifier
            Some(token_str[1..].parse::<Identifier>()?)
        } else {
            // Anonymous hole: just `!`
            None
        };

        let node_id = self.document.current_node_id();
        self.document
            .bind_hole(label)
            .map_err(|e| DocumentConstructionError::DocumentInsert {
                error: e,
                node_id: handle.node_id(),
            })?;
        self.record_value(node_id, handle.node_id());
        Ok(())
    }

    fn visit_inline_code_1(
        &mut self,
        handle: InlineCode1Handle,
        view: InlineCode1View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        let token_str = self.get_terminal_str(tree, view.inline_code_1)?;
        let (language, content) = Self::parse_inline_code_1(token_str).map_err(|error| {
            DocumentConstructionError::InvalidInlineCode {
                node_id: view.inline_code_1.node_id(),
                error,
            }
        })?;
        let text = Text::with_syntax_hint(content, language, SyntaxHint::Inline1);
        let node_id = self.document.current_node_id();
        self.document
            .bind_primitive(PrimitiveValue::Text(text))
            .map_err(|e| DocumentConstructionError::DocumentInsert {
                error: e,
                node_id: handle.node_id(),
            })?;
        self.record_value(node_id, handle.node_id());
        Ok(())
    }

    fn visit_inline_code_2(
        &mut self,
        handle: InlineCode2Handle,
        view: InlineCode2View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Store the origin before visiting children
        self.pending_code_origin = Some(CodeOrigin::InlineCode2(handle));
        // Visit children (start, content, end)
        self.visit_inline_code_2_super(handle, view, tree)?;
        self.pending_code_origin = None;
        Ok(())
    }

    fn visit_inline_code_start_2(
        &mut self,
        _handle: InlineCodeStart2Handle,
        view: InlineCodeStart2View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        let token_str = self.get_terminal_str(tree, view.inline_code_start_2)?;
        let language = Self::parse_inline_code_start_2(token_str).map_err(|error| {
            DocumentConstructionError::InvalidInlineCode {
                node_id: view.inline_code_start_2.node_id(),
                error,
            }
        })?;
        // Use pending origin if available
        self.code_start = if let Some(origin) = self.pending_code_origin {
            Some(CodeStart::with_origin(
                language,
                SyntaxHint::Inline2,
                origin,
            ))
        } else {
            Some(CodeStart::new(language, SyntaxHint::Inline2))
        };
        Ok(())
    }

    fn visit_inline_code_end_2(
        &mut self,
        handle: InlineCodeEnd2Handle,
        _view: InlineCodeEnd2View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        if let Some(code_start) = self.code_start.take() {
            let content = code_start.terminals.into_string(self.input, tree)?;
            let text = Text::with_syntax_hint(content, code_start.language, code_start.syntax_hint);
            let node_id = self.document.current_node_id();
            self.document
                .bind_primitive(PrimitiveValue::Text(text))
                .map_err(|e| DocumentConstructionError::DocumentInsert {
                    error: e,
                    node_id: handle.node_id(),
                })?;
            // Record origin if available
            if let Some(CodeOrigin::InlineCode2(inline_handle)) = code_start.origin {
                self.record_value(node_id, inline_handle.node_id());
            }
        }
        Ok(())
    }

    fn visit_code_block(
        &mut self,
        handle: CodeBlockHandle,
        view: CodeBlockView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Store the origin before visiting children
        self.pending_code_origin = Some(CodeOrigin::CodeBlock(handle));
        // Visit children
        self.visit_code_block_super(handle, view, tree)?;
        self.pending_code_origin = None;
        Ok(())
    }

    fn visit_code_block_start_3(
        &mut self,
        _handle: CodeBlockStart3Handle,
        view: CodeBlockStart3View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        let token_str = self.get_terminal_str(tree, view.code_block_start_3)?;
        let language = Self::parse_code_block_start(token_str).map_err(|error| {
            DocumentConstructionError::InvalidCodeBlock {
                node_id: view.code_block_start_3.node_id(),
                error,
            }
        })?;
        // Use pending origin if available
        self.code_start = if let Some(origin) = self.pending_code_origin {
            Some(CodeStart::with_origin(language, SyntaxHint::Block3, origin))
        } else {
            Some(CodeStart::new(language, SyntaxHint::Block3))
        };
        Ok(())
    }

    fn visit_code_block_end_3(
        &mut self,
        handle: CodeBlockEnd3Handle,
        _view: CodeBlockEnd3View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        if let Some(code_start) = self.code_start.take() {
            let content = code_start.terminals.into_string(self.input, tree)?;
            let text =
                Text::parse_indented_block(code_start.language, content, code_start.syntax_hint)
                    .map_err(|e| DocumentConstructionError::InvalidCodeBlock {
                        node_id: handle.node_id(),
                        error: CodeBlockError::from(e),
                    })?;
            let node_id = self.document.current_node_id();
            self.document
                .bind_primitive(PrimitiveValue::Text(text))
                .map_err(|e| DocumentConstructionError::DocumentInsert {
                    error: e,
                    node_id: handle.node_id(),
                })?;
            // Record origin if available
            if let Some(CodeOrigin::CodeBlock(block_handle)) = code_start.origin {
                self.record_value(node_id, block_handle.node_id());
            }
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
        let language = Self::parse_code_block_start(token_str).map_err(|error| {
            DocumentConstructionError::InvalidCodeBlock {
                node_id: view.code_block_start_4.node_id(),
                error,
            }
        })?;
        // Use pending origin if available
        self.code_start = if let Some(origin) = self.pending_code_origin {
            Some(CodeStart::with_origin(language, SyntaxHint::Block4, origin))
        } else {
            Some(CodeStart::new(language, SyntaxHint::Block4))
        };
        Ok(())
    }

    fn visit_code_block_end_4(
        &mut self,
        handle: CodeBlockEnd4Handle,
        _view: CodeBlockEnd4View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        if let Some(code_start) = self.code_start.take() {
            let content = code_start.terminals.into_string(self.input, tree)?;
            let text =
                Text::parse_indented_block(code_start.language, content, code_start.syntax_hint)
                    .map_err(|e| DocumentConstructionError::InvalidCodeBlock {
                        node_id: handle.node_id(),
                        error: CodeBlockError::from(e),
                    })?;
            let node_id = self.document.current_node_id();
            self.document
                .bind_primitive(PrimitiveValue::Text(text))
                .map_err(|e| DocumentConstructionError::DocumentInsert {
                    error: e,
                    node_id: handle.node_id(),
                })?;
            // Record origin if available
            if let Some(CodeOrigin::CodeBlock(block_handle)) = code_start.origin {
                self.record_value(node_id, block_handle.node_id());
            }
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
        let language = Self::parse_code_block_start(token_str).map_err(|error| {
            DocumentConstructionError::InvalidCodeBlock {
                node_id: view.code_block_start_5.node_id(),
                error,
            }
        })?;
        // Use pending origin if available
        self.code_start = if let Some(origin) = self.pending_code_origin {
            Some(CodeStart::with_origin(language, SyntaxHint::Block5, origin))
        } else {
            Some(CodeStart::new(language, SyntaxHint::Block5))
        };
        Ok(())
    }

    fn visit_code_block_end_5(
        &mut self,
        handle: CodeBlockEnd5Handle,
        _view: CodeBlockEnd5View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        if let Some(code_start) = self.code_start.take() {
            let content = code_start.terminals.into_string(self.input, tree)?;
            let text =
                Text::parse_indented_block(code_start.language, content, code_start.syntax_hint)
                    .map_err(|e| DocumentConstructionError::InvalidCodeBlock {
                        node_id: handle.node_id(),
                        error: CodeBlockError::from(e),
                    })?;
            let node_id = self.document.current_node_id();
            self.document
                .bind_primitive(PrimitiveValue::Text(text))
                .map_err(|e| DocumentConstructionError::DocumentInsert {
                    error: e,
                    node_id: handle.node_id(),
                })?;
            // Record origin if available
            if let Some(CodeOrigin::CodeBlock(block_handle)) = code_start.origin {
                self.record_value(node_id, block_handle.node_id());
            }
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
        let language = Self::parse_code_block_start(token_str).map_err(|error| {
            DocumentConstructionError::InvalidCodeBlock {
                node_id: view.code_block_start_6.node_id(),
                error,
            }
        })?;
        // Use pending origin if available
        self.code_start = if let Some(origin) = self.pending_code_origin {
            Some(CodeStart::with_origin(language, SyntaxHint::Block6, origin))
        } else {
            Some(CodeStart::new(language, SyntaxHint::Block6))
        };
        Ok(())
    }

    fn visit_code_block_end_6(
        &mut self,
        handle: CodeBlockEnd6Handle,
        _view: CodeBlockEnd6View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        if let Some(code_start) = self.code_start.take() {
            let content = code_start.terminals.into_string(self.input, tree)?;
            let text =
                Text::parse_indented_block(code_start.language, content, code_start.syntax_hint)
                    .map_err(|e| DocumentConstructionError::InvalidCodeBlock {
                        node_id: handle.node_id(),
                        error: CodeBlockError::from(e),
                    })?;
            let node_id = self.document.current_node_id();
            self.document
                .bind_primitive(PrimitiveValue::Text(text))
                .map_err(|e| DocumentConstructionError::DocumentInsert {
                    error: e,
                    node_id: handle.node_id(),
                })?;
            // Record origin if available
            if let Some(CodeOrigin::CodeBlock(block_handle)) = code_start.origin {
                self.record_value(node_id, block_handle.node_id());
            }
        }
        Ok(())
    }

    fn visit_text_binding(
        &mut self,
        handle: TextBindingHandle,
        view: TextBindingView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        let text_view = view.text.get_view(tree)?;
        let text_str = self.get_terminal_str(tree, text_view.text)?;
        let text = Text::parse_text_binding(text_str).map_err(|error| {
            DocumentConstructionError::InvalidStringKey {
                node_id: handle.node_id(),
                error,
            }
        })?;
        let node_id = self.document.current_node_id();
        self.document
            .bind_primitive(PrimitiveValue::Text(text))
            .map_err(|e| DocumentConstructionError::DocumentInsert {
                error: e,
                node_id: handle.node_id(),
            })?;
        self.record_value(node_id, handle.node_id());
        Ok(())
    }

    fn visit_strings(
        &mut self,
        handle: StringsHandle,
        view: StringsView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Parse the first string
        let first_str = self.parse_str_terminal(view.str, tree)?;

        // Check for continuations
        let result = if let Some(list_view) = view.strings_list.get_view(tree)? {
            // There are continuation strings - collect and join them
            let mut parts = vec![first_str];
            for item in list_view.get_all(tree)? {
                let part = self.parse_str_terminal(item.str, tree)?;
                parts.push(part);
            }
            parts.join("")
        } else {
            first_str
        };

        let text = Text::plaintext(result);
        let node_id = self.document.current_node_id();
        self.document
            .bind_primitive(PrimitiveValue::Text(text))
            .map_err(|e| DocumentConstructionError::DocumentInsert {
                error: e,
                node_id: handle.node_id(),
            })?;
        self.record_value(node_id, handle.node_id());
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

    fn then_construct_error(
        &mut self,
        _node_data: Option<CstNode>,
        _parent: CstNodeId,
        _kind: NodeKind,
        error: CstConstructError,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        Err(DocumentConstructionError::CstError(error))
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
            let result = CstInterpreter::parse_inline_code_1("`hello`");
            assert!(result.is_ok());
            let (language, content) = result.unwrap();
            assert_eq!(language, Language::Implicit);
            assert_eq!(content, "hello");
        }

        #[test]
        fn test_code_with_language() {
            let result = CstInterpreter::parse_inline_code_1("rust`fn main() {}`");
            assert!(result.is_ok());
            let (language, content) = result.unwrap();
            assert_eq!(language, Language::Other("rust".into()));
            assert_eq!(content, "fn main() {}");
        }

        #[test]
        fn test_empty_code() {
            let result = CstInterpreter::parse_inline_code_1("``");
            assert!(result.is_ok());
            let (language, content) = result.unwrap();
            assert_eq!(language, Language::Implicit);
            assert_eq!(content, "");
        }

        #[test]
        fn test_code_with_special_chars() {
            let result = CstInterpreter::parse_inline_code_1("`hello world!@#$%`");
            assert!(result.is_ok());
            let (language, content) = result.unwrap();
            assert_eq!(language, Language::Implicit);
            assert_eq!(content, "hello world!@#$%");
        }

        #[test]
        fn test_language_with_hyphen_and_underscore() {
            let result = CstInterpreter::parse_inline_code_1("foo-bar_123`content`");
            assert!(result.is_ok());
            let (language, content) = result.unwrap();
            assert_eq!(language, Language::Other("foo-bar_123".into()));
            assert_eq!(content, "content");
        }

        #[test]
        fn test_no_backticks() {
            let result = CstInterpreter::parse_inline_code_1("no backticks");
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                InlineCodeError::InvalidInlineCode1Pattern
            ));
        }

        #[test]
        fn test_single_backtick() {
            let result = CstInterpreter::parse_inline_code_1("`");
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                InlineCodeError::InvalidInlineCode1Pattern
            ));
        }
    }

    // Tests for parse_inline_code_start_2
    mod parse_inline_code_start_2_tests {
        use super::*;

        #[test]
        fn test_no_language() {
            let result = CstInterpreter::parse_inline_code_start_2("``");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Language::Implicit);
        }

        #[test]
        fn test_with_language() {
            let result = CstInterpreter::parse_inline_code_start_2("rust``");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Language::Other("rust".into()));
        }

        #[test]
        fn test_with_complex_language() {
            let result = CstInterpreter::parse_inline_code_start_2("foo-bar_123``");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Language::Other("foo-bar_123".into()));
        }

        #[test]
        fn test_no_backticks() {
            let result = CstInterpreter::parse_inline_code_start_2("rust");
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                InlineCodeError::InvalidInlineCodeStart2Pattern
            ));
        }
    }

    // Tests for parse_code_block_start
    mod parse_code_block_start_tests {
        use crate::document::CodeBlockError;

        use super::*;

        #[test]
        fn test_no_language_3_backticks() {
            let result = CstInterpreter::parse_code_block_start("```\n");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Language::Implicit);
        }

        #[test]
        fn test_with_language_3_backticks() {
            let result = CstInterpreter::parse_code_block_start("```rust\n");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Language::Other("rust".into()));
        }

        #[test]
        fn test_with_language_4_backticks() {
            let result = CstInterpreter::parse_code_block_start("````python\n");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Language::Other("python".into()));
        }

        #[test]
        fn test_with_language_5_backticks() {
            let result = CstInterpreter::parse_code_block_start("`````javascript\n");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Language::Other("javascript".into()));
        }

        #[test]
        fn test_with_language_6_backticks() {
            let result = CstInterpreter::parse_code_block_start("``````typescript\n");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Language::Other("typescript".into()));
        }

        #[test]
        fn test_language_with_trailing_whitespace() {
            let result = CstInterpreter::parse_code_block_start("```rust  \n");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Language::Other("rust".into()));
        }

        #[test]
        fn test_language_with_leading_whitespace_is_invalid() {
            // Leading whitespace before language tag with non-whitespace after is grammar violation
            // Pattern: ```[a-zA-Z0-9_-]*[ \t]*\n but this has ``` [ \t]+ [a-z]+ which doesn't match
            let result = CstInterpreter::parse_code_block_start("```  rust\n");
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                CodeBlockError::InvalidCodeBlockStartPattern
            ));
        }

        #[test]
        fn test_language_with_carriage_return() {
            let result = CstInterpreter::parse_code_block_start("```rust\r\n");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Language::Other("rust".into()));
        }

        #[test]
        fn test_language_with_only_carriage_return() {
            let result = CstInterpreter::parse_code_block_start("```rust\r");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Language::Other("rust".into()));
        }

        #[test]
        fn test_empty_language_with_spaces() {
            let result = CstInterpreter::parse_code_block_start("```   \n");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Language::Implicit);
        }

        #[test]
        fn test_no_newline() {
            let result = CstInterpreter::parse_code_block_start("```rust");
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                CodeBlockError::InvalidCodeBlockStartPattern
            ));
        }

        #[test]
        fn test_complex_language_tag() {
            let result = CstInterpreter::parse_code_block_start("```foo-bar_123\n");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Language::Other("foo-bar_123".into()));
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
