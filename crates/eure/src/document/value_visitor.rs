use eure_tree::prelude::*;
use eure_value::document::{EureDocument, InsertError, constructor::DocumentConstructor};
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
}

pub struct ValueVisitor<'a> {
    input: &'a str,
    // Main document being built
    document: DocumentConstructor,
}

impl<'a> ValueVisitor<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            document: DocumentConstructor::new(),
        }
    }

    pub fn into_document(self) -> EureDocument {
        self.document.finish()
    }
}

impl<F: CstFacade> CstVisitor<F> for ValueVisitor<'_> {
    type Error = DocumentConstructionError;

    // Constructs an array node (e.g., [1, 2, 3])
    fn visit_array(
        &mut self,
        handle: ArrayHandle,
        view: ArrayView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_array_super(handle, view, tree)
    }

    // Processes array elements list
    fn visit_array_elements(
        &mut self,
        handle: ArrayElementsHandle,
        view: ArrayElementsView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_array_elements_super(handle, view, tree)
    }

    // Processes a binding statement (e.g., a = value)
    fn visit_binding(
        &mut self,
        handle: BindingHandle,
        view: BindingView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_binding_super(handle, view, tree)
    }

    // Processes the right-hand side of a binding
    fn visit_binding_rhs(
        &mut self,
        handle: BindingRhsHandle,
        view: BindingRhsView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_binding_rhs_super(handle, view, tree)
    }

    // Processes a boolean value node
    fn visit_boolean(
        &mut self,
        handle: BooleanHandle,
        view: BooleanView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_boolean_super(handle, view, tree)
    }

    // Processes a code value
    fn visit_code(
        &mut self,
        handle: CodeHandle,
        view: CodeView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_super(handle, view, tree)
    }

    // Processes a code block value
    fn visit_code_block(
        &mut self,
        handle: CodeBlockHandle,
        view: CodeBlockView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_super(handle, view, tree)
    }

    // Processes a direct binding (e.g., bind without intermediate path)
    fn visit_direct_bind(
        &mut self,
        handle: DirectBindHandle,
        view: DirectBindView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_direct_bind_super(handle, view, tree)
    }

    // Processes the entire Eure document
    fn visit_eure(
        &mut self,
        handle: EureHandle,
        view: EureView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_eure_super(handle, view, tree)
    }

    // Processes the Eure root node
    fn visit_eure_root(
        &mut self,
        handle: EureRootHandle,
        view: EureRootView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_eure_root_super(handle, view, tree)
    }

    // Processes an extension (e.g., $ext)
    fn visit_ext(&mut self, handle: ExtHandle, view: ExtView, tree: &F) -> Result<(), Self::Error> {
        self.visit_ext_super(handle, view, tree)
    }

    // Processes extension namespace
    fn visit_extension_name_space(
        &mut self,
        handle: ExtensionNameSpaceHandle,
        view: ExtensionNameSpaceView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_extension_name_space_super(handle, view, tree)
    }

    // Processes a false literal node
    fn visit_false(
        &mut self,
        handle: FalseHandle,
        view: FalseView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_false_super(handle, view, tree)
    }

    // Processes a float literal node
    fn visit_float(
        &mut self,
        handle: FloatHandle,
        view: FloatView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_float_super(handle, view, tree)
    }

    // Processes a hole value (_)
    fn visit_hole(
        &mut self,
        handle: HoleHandle,
        view: HoleView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_hole_super(handle, view, tree)
    }

    // Processes an identifier node
    fn visit_ident(
        &mut self,
        handle: IdentHandle,
        view: IdentView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_ident_super(handle, view, tree)
    }

    // Processes an integer literal node
    fn visit_integer(
        &mut self,
        handle: IntegerHandle,
        view: IntegerView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_integer_super(handle, view, tree)
    }

    // Processes an object key
    fn visit_key(&mut self, handle: KeyHandle, view: KeyView, tree: &F) -> Result<(), Self::Error> {
        self.visit_key_super(handle, view, tree)
    }

    // Processes a keys list
    fn visit_keys(
        &mut self,
        handle: KeysHandle,
        view: KeysView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_keys_super(handle, view, tree)
    }

    // Processes a meta-extension (e.g., $$meta)
    fn visit_meta_ext(
        &mut self,
        handle: MetaExtHandle,
        view: MetaExtView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_meta_ext_super(handle, view, tree)
    }

    // Processes a meta-extension key
    fn visit_meta_ext_key(
        &mut self,
        handle: MetaExtKeyHandle,
        view: MetaExtKeyView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_meta_ext_key_super(handle, view, tree)
    }

    // Processes a named code value
    fn visit_named_code(
        &mut self,
        handle: NamedCodeHandle,
        view: NamedCodeView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_named_code_super(handle, view, tree)
    }

    // Processes a null literal node
    fn visit_null(
        &mut self,
        handle: NullHandle,
        view: NullView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_null_super(handle, view, tree)
    }

    // Constructs an object (Map) node
    fn visit_object(
        &mut self,
        handle: ObjectHandle,
        view: ObjectView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_object_super(handle, view, tree)
    }

    // Processes object elements list
    fn visit_object_list(
        &mut self,
        handle: ObjectListHandle,
        view: ObjectListView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_object_list_super(handle, view, tree)
    }

    // Processes a path (e.g., a.b.c)
    fn visit_path(
        &mut self,
        handle: PathHandle,
        view: PathView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_path_super(handle, view, tree)
    }

    // Processes a section
    fn visit_section(
        &mut self,
        handle: SectionHandle,
        view: SectionView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_section_super(handle, view, tree)
    }

    // Processes a section binding
    fn visit_section_binding(
        &mut self,
        handle: SectionBindingHandle,
        view: SectionBindingView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_section_binding_super(handle, view, tree)
    }

    // Processes section body
    fn visit_section_body(
        &mut self,
        handle: SectionBodyHandle,
        view: SectionBodyView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_section_body_super(handle, view, tree)
    }

    // Processes a string literal node
    fn visit_str(&mut self, handle: StrHandle, view: StrView, tree: &F) -> Result<(), Self::Error> {
        self.visit_str_super(handle, view, tree)
    }

    // Processes string concatenation
    fn visit_strings(
        &mut self,
        handle: StringsHandle,
        view: StringsView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_strings_super(handle, view, tree)
    }

    // Processes a text node
    fn visit_text(
        &mut self,
        handle: TextHandle,
        view: TextView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_text_super(handle, view, tree)
    }

    // Processes text binding
    fn visit_text_binding(
        &mut self,
        handle: TextBindingHandle,
        view: TextBindingView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_text_binding_super(handle, view, tree)
    }

    // Processes a true literal node
    fn visit_true(
        &mut self,
        handle: TrueHandle,
        view: TrueView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_true_super(handle, view, tree)
    }

    // Constructs a tuple node (e.g., (1, 2, 3))
    fn visit_tuple(
        &mut self,
        handle: TupleHandle,
        view: TupleView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_tuple_super(handle, view, tree)
    }

    // Processes tuple elements list
    fn visit_tuple_elements(
        &mut self,
        handle: TupleElementsHandle,
        view: TupleElementsView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_tuple_elements_super(handle, view, tree)
    }

    // Processes a value node (any value type)
    fn visit_value(
        &mut self,
        handle: ValueHandle,
        view: ValueView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_value_super(handle, view, tree)
    }

    // Processes value binding
    fn visit_value_binding(
        &mut self,
        handle: ValueBindingHandle,
        view: ValueBindingView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_value_binding_super(handle, view, tree)
    }

    // Entry point for visiting the CST
    fn visit_root(
        &mut self,
        handle: RootHandle,
        view: RootView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_root_super(handle, view, tree)
    }

    // Extracts float value from terminal
    fn visit_float_terminal(
        &mut self,
        terminal: Float,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_float_terminal_super(terminal, data, tree)
    }

    // Extracts integer value from terminal
    fn visit_integer_terminal(
        &mut self,
        terminal: Integer,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_integer_terminal_super(terminal, data, tree)
    }

    // Extracts true boolean value
    fn visit_true_terminal(
        &mut self,
        terminal: True,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_true_terminal_super(terminal, data, tree)
    }

    // Extracts false boolean value
    fn visit_false_terminal(
        &mut self,
        terminal: False,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_false_terminal_super(terminal, data, tree)
    }

    // Extracts null value
    fn visit_null_terminal(
        &mut self,
        terminal: Null,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_null_terminal_super(terminal, data, tree)
    }

    // Extracts hole (_) value
    fn visit_hole_terminal(
        &mut self,
        terminal: Hole,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_hole_terminal_super(terminal, data, tree)
    }

    // Extracts string value from terminal
    fn visit_str_terminal(
        &mut self,
        terminal: Str,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_str_terminal_super(terminal, data, tree)
    }

    // Extracts text value from terminal
    fn visit_text_terminal(
        &mut self,
        terminal: Text,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_text_terminal_super(terminal, data, tree)
    }

    // Extracts code block value from terminal
    fn visit_code_block_terminal(
        &mut self,
        terminal: CodeBlock,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_terminal_super(terminal, data, tree)
    }

    // Extracts named code value from terminal
    fn visit_named_code_terminal(
        &mut self,
        terminal: NamedCode,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_named_code_terminal_super(terminal, data, tree)
    }

    // Extracts code value from terminal
    fn visit_code_terminal(
        &mut self,
        terminal: Code,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_terminal_super(terminal, data, tree)
    }

    // Extracts identifier from terminal
    fn visit_ident_terminal(
        &mut self,
        terminal: Ident,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_ident_terminal_super(terminal, data, tree)
    }

    // Generic handler for non-terminal nodes
    fn visit_non_terminal(
        &mut self,
        id: CstNodeId,
        kind: NonTerminalKind,
        data: NonTerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_non_terminal_super(id, kind, data, tree)
    }

    // Generic handler for closing non-terminal nodes
    fn visit_non_terminal_close(
        &mut self,
        id: CstNodeId,
        kind: NonTerminalKind,
        data: NonTerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_non_terminal_close_super(id, kind, data, tree)
    }

    // Generic handler for terminal nodes
    fn visit_terminal(
        &mut self,
        id: CstNodeId,
        kind: TerminalKind,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_terminal_super(id, kind, data, tree)
    }

    // Recovers from CST construction errors
    fn then_construct_error(
        &mut self,
        node_data: Option<CstNode>,
        parent: CstNodeId,
        kind: NodeKind,
        error: CstConstructError,
        tree: &F,
    ) -> Result<(), Self::Error> {
        let _error = error;
        self.recover_error(node_data, parent, kind, tree)
    }
}
