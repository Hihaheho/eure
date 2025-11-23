use crate::{
    Cst, CstConstructError, NodeKind, CstNode, nodes::*,
    node_kind::{TerminalKind, NonTerminalKind},
    tree::{
        TerminalHandle as _, NonTerminalHandle as _, TerminalData, NonTerminalData,
        CstNodeId, CstFacade,
    },
};
pub trait CstVisitor<F: CstFacade>: CstVisitorSuper<F, Self::Error> {
    type Error;
    fn visit_array(
        &mut self,
        handle: ArrayHandle,
        view: ArrayView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_array_super(handle, view, tree)
    }
    fn visit_array_begin(
        &mut self,
        handle: ArrayBeginHandle,
        view: ArrayBeginView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_array_begin_super(handle, view, tree)
    }
    fn visit_array_elements(
        &mut self,
        handle: ArrayElementsHandle,
        view: ArrayElementsView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_array_elements_super(handle, view, tree)
    }
    fn visit_array_elements_opt(
        &mut self,
        handle: ArrayElementsOptHandle,
        view: ArrayElementsTailHandle,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_array_elements_opt_super(handle, view, tree)
    }
    fn visit_array_elements_tail(
        &mut self,
        handle: ArrayElementsTailHandle,
        view: ArrayElementsTailView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_array_elements_tail_super(handle, view, tree)
    }
    fn visit_array_elements_tail_opt(
        &mut self,
        handle: ArrayElementsTailOptHandle,
        view: ArrayElementsHandle,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_array_elements_tail_opt_super(handle, view, tree)
    }
    fn visit_array_end(
        &mut self,
        handle: ArrayEndHandle,
        view: ArrayEndView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_array_end_super(handle, view, tree)
    }
    fn visit_array_marker(
        &mut self,
        handle: ArrayMarkerHandle,
        view: ArrayMarkerView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_array_marker_super(handle, view, tree)
    }
    fn visit_array_marker_opt(
        &mut self,
        handle: ArrayMarkerOptHandle,
        view: IntegerHandle,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_array_marker_opt_super(handle, view, tree)
    }
    fn visit_array_opt(
        &mut self,
        handle: ArrayOptHandle,
        view: ArrayElementsHandle,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_array_opt_super(handle, view, tree)
    }
    fn visit_at(
        &mut self,
        handle: AtHandle,
        view: AtView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_at_super(handle, view, tree)
    }
    fn visit_backtick_1(
        &mut self,
        handle: Backtick1Handle,
        view: Backtick1View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_backtick_1_super(handle, view, tree)
    }
    fn visit_backtick_2(
        &mut self,
        handle: Backtick2Handle,
        view: Backtick2View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_backtick_2_super(handle, view, tree)
    }
    fn visit_backtick_3(
        &mut self,
        handle: Backtick3Handle,
        view: Backtick3View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_backtick_3_super(handle, view, tree)
    }
    fn visit_backtick_4(
        &mut self,
        handle: Backtick4Handle,
        view: Backtick4View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_backtick_4_super(handle, view, tree)
    }
    fn visit_backtick_5(
        &mut self,
        handle: Backtick5Handle,
        view: Backtick5View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_backtick_5_super(handle, view, tree)
    }
    fn visit_begin(
        &mut self,
        handle: BeginHandle,
        view: BeginView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_begin_super(handle, view, tree)
    }
    fn visit_bind(
        &mut self,
        handle: BindHandle,
        view: BindView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_bind_super(handle, view, tree)
    }
    fn visit_binding(
        &mut self,
        handle: BindingHandle,
        view: BindingView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_binding_super(handle, view, tree)
    }
    fn visit_binding_rhs(
        &mut self,
        handle: BindingRhsHandle,
        view: BindingRhsView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_binding_rhs_super(handle, view, tree)
    }
    fn visit_boolean(
        &mut self,
        handle: BooleanHandle,
        view: BooleanView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_boolean_super(handle, view, tree)
    }
    fn visit_code_block(
        &mut self,
        handle: CodeBlockHandle,
        view: CodeBlockView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_super(handle, view, tree)
    }
    fn visit_code_block_3(
        &mut self,
        handle: CodeBlock3Handle,
        view: CodeBlock3View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_3_super(handle, view, tree)
    }
    fn visit_code_block_3_list(
        &mut self,
        handle: CodeBlock3ListHandle,
        view: CodeBlock3ListView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_3_list_super(handle, view, tree)
    }
    fn visit_code_block_3_list_group(
        &mut self,
        handle: CodeBlock3ListGroupHandle,
        view: CodeBlock3ListGroupView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_3_list_group_super(handle, view, tree)
    }
    fn visit_code_block_4(
        &mut self,
        handle: CodeBlock4Handle,
        view: CodeBlock4View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_4_super(handle, view, tree)
    }
    fn visit_code_block_4_list(
        &mut self,
        handle: CodeBlock4ListHandle,
        view: CodeBlock4ListView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_4_list_super(handle, view, tree)
    }
    fn visit_code_block_4_list_group(
        &mut self,
        handle: CodeBlock4ListGroupHandle,
        view: CodeBlock4ListGroupView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_4_list_group_super(handle, view, tree)
    }
    fn visit_code_block_5(
        &mut self,
        handle: CodeBlock5Handle,
        view: CodeBlock5View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_5_super(handle, view, tree)
    }
    fn visit_code_block_5_list(
        &mut self,
        handle: CodeBlock5ListHandle,
        view: CodeBlock5ListView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_5_list_super(handle, view, tree)
    }
    fn visit_code_block_5_list_group(
        &mut self,
        handle: CodeBlock5ListGroupHandle,
        view: CodeBlock5ListGroupView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_5_list_group_super(handle, view, tree)
    }
    fn visit_code_block_6(
        &mut self,
        handle: CodeBlock6Handle,
        view: CodeBlock6View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_6_super(handle, view, tree)
    }
    fn visit_code_block_6_list(
        &mut self,
        handle: CodeBlock6ListHandle,
        view: CodeBlock6ListView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_6_list_super(handle, view, tree)
    }
    fn visit_code_block_6_list_group(
        &mut self,
        handle: CodeBlock6ListGroupHandle,
        view: CodeBlock6ListGroupView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_6_list_group_super(handle, view, tree)
    }
    fn visit_code_block_end_3(
        &mut self,
        handle: CodeBlockEnd3Handle,
        view: CodeBlockEnd3View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_end_3_super(handle, view, tree)
    }
    fn visit_code_block_end_4(
        &mut self,
        handle: CodeBlockEnd4Handle,
        view: CodeBlockEnd4View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_end_4_super(handle, view, tree)
    }
    fn visit_code_block_end_5(
        &mut self,
        handle: CodeBlockEnd5Handle,
        view: CodeBlockEnd5View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_end_5_super(handle, view, tree)
    }
    fn visit_code_block_end_6(
        &mut self,
        handle: CodeBlockEnd6Handle,
        view: CodeBlockEnd6View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_end_6_super(handle, view, tree)
    }
    fn visit_code_block_start_3(
        &mut self,
        handle: CodeBlockStart3Handle,
        view: CodeBlockStart3View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_start_3_super(handle, view, tree)
    }
    fn visit_code_block_start_4(
        &mut self,
        handle: CodeBlockStart4Handle,
        view: CodeBlockStart4View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_start_4_super(handle, view, tree)
    }
    fn visit_code_block_start_5(
        &mut self,
        handle: CodeBlockStart5Handle,
        view: CodeBlockStart5View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_start_5_super(handle, view, tree)
    }
    fn visit_code_block_start_6(
        &mut self,
        handle: CodeBlockStart6Handle,
        view: CodeBlockStart6View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_start_6_super(handle, view, tree)
    }
    fn visit_comma(
        &mut self,
        handle: CommaHandle,
        view: CommaView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_comma_super(handle, view, tree)
    }
    fn visit_continue(
        &mut self,
        handle: ContinueHandle,
        view: ContinueView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_continue_super(handle, view, tree)
    }
    fn visit_dot(
        &mut self,
        handle: DotHandle,
        view: DotView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_dot_super(handle, view, tree)
    }
    fn visit_end(
        &mut self,
        handle: EndHandle,
        view: EndView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_end_super(handle, view, tree)
    }
    fn visit_eure(
        &mut self,
        handle: EureHandle,
        view: EureView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_eure_super(handle, view, tree)
    }
    fn visit_eure_bindings(
        &mut self,
        handle: EureBindingsHandle,
        view: EureBindingsView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_eure_bindings_super(handle, view, tree)
    }
    fn visit_eure_sections(
        &mut self,
        handle: EureSectionsHandle,
        view: EureSectionsView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_eure_sections_super(handle, view, tree)
    }
    fn visit_eure_opt(
        &mut self,
        handle: EureOptHandle,
        view: ValueBindingHandle,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_eure_opt_super(handle, view, tree)
    }
    fn visit_ext(
        &mut self,
        handle: ExtHandle,
        view: ExtView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_ext_super(handle, view, tree)
    }
    fn visit_extension_name_space(
        &mut self,
        handle: ExtensionNameSpaceHandle,
        view: ExtensionNameSpaceView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_extension_name_space_super(handle, view, tree)
    }
    fn visit_false(
        &mut self,
        handle: FalseHandle,
        view: FalseView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_false_super(handle, view, tree)
    }
    fn visit_float(
        &mut self,
        handle: FloatHandle,
        view: FloatView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_float_super(handle, view, tree)
    }
    fn visit_grammar_newline(
        &mut self,
        handle: GrammarNewlineHandle,
        view: GrammarNewlineView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_grammar_newline_super(handle, view, tree)
    }
    fn visit_hole(
        &mut self,
        handle: HoleHandle,
        view: HoleView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_hole_super(handle, view, tree)
    }
    fn visit_ident(
        &mut self,
        handle: IdentHandle,
        view: IdentView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_ident_super(handle, view, tree)
    }
    fn visit_inline_code(
        &mut self,
        handle: InlineCodeHandle,
        view: InlineCodeView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_inline_code_super(handle, view, tree)
    }
    fn visit_inline_code_1(
        &mut self,
        handle: InlineCode1Handle,
        view: InlineCode1View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_inline_code_1_super(handle, view, tree)
    }
    fn visit_inline_code_2(
        &mut self,
        handle: InlineCode2Handle,
        view: InlineCode2View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_inline_code_2_super(handle, view, tree)
    }
    fn visit_inline_code_2_list(
        &mut self,
        handle: InlineCode2ListHandle,
        view: InlineCode2ListView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_inline_code_2_list_super(handle, view, tree)
    }
    fn visit_inline_code_2_list_group(
        &mut self,
        handle: InlineCode2ListGroupHandle,
        view: InlineCode2ListGroupView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_inline_code_2_list_group_super(handle, view, tree)
    }
    fn visit_inline_code_end_2(
        &mut self,
        handle: InlineCodeEnd2Handle,
        view: InlineCodeEnd2View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_inline_code_end_2_super(handle, view, tree)
    }
    fn visit_inline_code_start_2(
        &mut self,
        handle: InlineCodeStart2Handle,
        view: InlineCodeStart2View,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_inline_code_start_2_super(handle, view, tree)
    }
    fn visit_integer(
        &mut self,
        handle: IntegerHandle,
        view: IntegerView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_integer_super(handle, view, tree)
    }
    fn visit_key(
        &mut self,
        handle: KeyHandle,
        view: KeyView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_key_super(handle, view, tree)
    }
    fn visit_key_base(
        &mut self,
        handle: KeyBaseHandle,
        view: KeyBaseView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_key_base_super(handle, view, tree)
    }
    fn visit_key_ident(
        &mut self,
        handle: KeyIdentHandle,
        view: KeyIdentView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_key_ident_super(handle, view, tree)
    }
    fn visit_key_opt(
        &mut self,
        handle: KeyOptHandle,
        view: ArrayMarkerHandle,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_key_opt_super(handle, view, tree)
    }
    fn visit_key_tuple(
        &mut self,
        handle: KeyTupleHandle,
        view: KeyTupleView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_key_tuple_super(handle, view, tree)
    }
    fn visit_key_tuple_elements(
        &mut self,
        handle: KeyTupleElementsHandle,
        view: KeyTupleElementsView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_key_tuple_elements_super(handle, view, tree)
    }
    fn visit_key_tuple_elements_opt(
        &mut self,
        handle: KeyTupleElementsOptHandle,
        view: KeyTupleElementsTailHandle,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_key_tuple_elements_opt_super(handle, view, tree)
    }
    fn visit_key_tuple_elements_tail(
        &mut self,
        handle: KeyTupleElementsTailHandle,
        view: KeyTupleElementsTailView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_key_tuple_elements_tail_super(handle, view, tree)
    }
    fn visit_key_tuple_elements_tail_opt(
        &mut self,
        handle: KeyTupleElementsTailOptHandle,
        view: KeyTupleElementsHandle,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_key_tuple_elements_tail_opt_super(handle, view, tree)
    }
    fn visit_key_tuple_opt(
        &mut self,
        handle: KeyTupleOptHandle,
        view: KeyTupleElementsHandle,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_key_tuple_opt_super(handle, view, tree)
    }
    fn visit_key_value(
        &mut self,
        handle: KeyValueHandle,
        view: KeyValueView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_key_value_super(handle, view, tree)
    }
    fn visit_keys(
        &mut self,
        handle: KeysHandle,
        view: KeysView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_keys_super(handle, view, tree)
    }
    fn visit_keys_list(
        &mut self,
        handle: KeysListHandle,
        view: KeysListView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_keys_list_super(handle, view, tree)
    }
    fn visit_l_paren(
        &mut self,
        handle: LParenHandle,
        view: LParenView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_l_paren_super(handle, view, tree)
    }
    fn visit_no_backtick(
        &mut self,
        handle: NoBacktickHandle,
        view: NoBacktickView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_no_backtick_super(handle, view, tree)
    }
    fn visit_no_backtick_inline(
        &mut self,
        handle: NoBacktickInlineHandle,
        view: NoBacktickInlineView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_no_backtick_inline_super(handle, view, tree)
    }
    fn visit_null(
        &mut self,
        handle: NullHandle,
        view: NullView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_null_super(handle, view, tree)
    }
    fn visit_object(
        &mut self,
        handle: ObjectHandle,
        view: ObjectView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_object_super(handle, view, tree)
    }
    fn visit_object_list(
        &mut self,
        handle: ObjectListHandle,
        view: ObjectListView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_object_list_super(handle, view, tree)
    }
    fn visit_object_opt(
        &mut self,
        handle: ObjectOptHandle,
        view: CommaHandle,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_object_opt_super(handle, view, tree)
    }
    fn visit_path(
        &mut self,
        handle: PathHandle,
        view: PathView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_path_super(handle, view, tree)
    }
    fn visit_r_paren(
        &mut self,
        handle: RParenHandle,
        view: RParenView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_r_paren_super(handle, view, tree)
    }
    fn visit_section(
        &mut self,
        handle: SectionHandle,
        view: SectionView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_section_super(handle, view, tree)
    }
    fn visit_section_binding(
        &mut self,
        handle: SectionBindingHandle,
        view: SectionBindingView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_section_binding_super(handle, view, tree)
    }
    fn visit_section_body(
        &mut self,
        handle: SectionBodyHandle,
        view: SectionBodyView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_section_body_super(handle, view, tree)
    }
    fn visit_section_body_list(
        &mut self,
        handle: SectionBodyListHandle,
        view: SectionBodyListView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_section_body_list_super(handle, view, tree)
    }
    fn visit_section_body_opt(
        &mut self,
        handle: SectionBodyOptHandle,
        view: ValueBindingHandle,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_section_body_opt_super(handle, view, tree)
    }
    fn visit_str(
        &mut self,
        handle: StrHandle,
        view: StrView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_str_super(handle, view, tree)
    }
    fn visit_strings(
        &mut self,
        handle: StringsHandle,
        view: StringsView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_strings_super(handle, view, tree)
    }
    fn visit_strings_list(
        &mut self,
        handle: StringsListHandle,
        view: StringsListView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_strings_list_super(handle, view, tree)
    }
    fn visit_text(
        &mut self,
        handle: TextHandle,
        view: TextView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_text_super(handle, view, tree)
    }
    fn visit_text_binding(
        &mut self,
        handle: TextBindingHandle,
        view: TextBindingView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_text_binding_super(handle, view, tree)
    }
    fn visit_text_binding_opt(
        &mut self,
        handle: TextBindingOptHandle,
        view: WsHandle,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_text_binding_opt_super(handle, view, tree)
    }
    fn visit_text_binding_opt_0(
        &mut self,
        handle: TextBindingOpt0Handle,
        view: GrammarNewlineHandle,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_text_binding_opt_0_super(handle, view, tree)
    }
    fn visit_text_start(
        &mut self,
        handle: TextStartHandle,
        view: TextStartView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_text_start_super(handle, view, tree)
    }
    fn visit_true(
        &mut self,
        handle: TrueHandle,
        view: TrueView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_true_super(handle, view, tree)
    }
    fn visit_tuple(
        &mut self,
        handle: TupleHandle,
        view: TupleView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_tuple_super(handle, view, tree)
    }
    fn visit_tuple_elements(
        &mut self,
        handle: TupleElementsHandle,
        view: TupleElementsView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_tuple_elements_super(handle, view, tree)
    }
    fn visit_tuple_elements_opt(
        &mut self,
        handle: TupleElementsOptHandle,
        view: TupleElementsTailHandle,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_tuple_elements_opt_super(handle, view, tree)
    }
    fn visit_tuple_elements_tail(
        &mut self,
        handle: TupleElementsTailHandle,
        view: TupleElementsTailView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_tuple_elements_tail_super(handle, view, tree)
    }
    fn visit_tuple_elements_tail_opt(
        &mut self,
        handle: TupleElementsTailOptHandle,
        view: TupleElementsHandle,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_tuple_elements_tail_opt_super(handle, view, tree)
    }
    fn visit_tuple_index(
        &mut self,
        handle: TupleIndexHandle,
        view: TupleIndexView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_tuple_index_super(handle, view, tree)
    }
    fn visit_tuple_opt(
        &mut self,
        handle: TupleOptHandle,
        view: TupleElementsHandle,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_tuple_opt_super(handle, view, tree)
    }
    fn visit_value(
        &mut self,
        handle: ValueHandle,
        view: ValueView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_value_super(handle, view, tree)
    }
    fn visit_value_binding(
        &mut self,
        handle: ValueBindingHandle,
        view: ValueBindingView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_value_binding_super(handle, view, tree)
    }
    fn visit_ws(
        &mut self,
        handle: WsHandle,
        view: WsView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_ws_super(handle, view, tree)
    }
    fn visit_root(
        &mut self,
        handle: RootHandle,
        view: RootView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_root_super(handle, view, tree)
    }
    fn visit_new_line_terminal(
        &mut self,
        terminal: NewLine,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_new_line_terminal_super(terminal, data, tree)
    }
    fn visit_whitespace_terminal(
        &mut self,
        terminal: Whitespace,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_whitespace_terminal_super(terminal, data, tree)
    }
    fn visit_line_comment_terminal(
        &mut self,
        terminal: LineComment,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_line_comment_terminal_super(terminal, data, tree)
    }
    fn visit_block_comment_terminal(
        &mut self,
        terminal: BlockComment,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_block_comment_terminal_super(terminal, data, tree)
    }
    fn visit_hash_terminal(
        &mut self,
        terminal: Hash,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_hash_terminal_super(terminal, data, tree)
    }
    fn visit_float_terminal(
        &mut self,
        terminal: Float,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_float_terminal_super(terminal, data, tree)
    }
    fn visit_integer_terminal(
        &mut self,
        terminal: Integer,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_integer_terminal_super(terminal, data, tree)
    }
    fn visit_true_terminal(
        &mut self,
        terminal: True,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_true_terminal_super(terminal, data, tree)
    }
    fn visit_false_terminal(
        &mut self,
        terminal: False,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_false_terminal_super(terminal, data, tree)
    }
    fn visit_null_terminal(
        &mut self,
        terminal: Null,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_null_terminal_super(terminal, data, tree)
    }
    fn visit_hole_terminal(
        &mut self,
        terminal: Hole,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_hole_terminal_super(terminal, data, tree)
    }
    fn visit_str_terminal(
        &mut self,
        terminal: Str,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_str_terminal_super(terminal, data, tree)
    }
    fn visit_text_terminal(
        &mut self,
        terminal: Text,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_text_terminal_super(terminal, data, tree)
    }
    fn visit_inline_code_1_terminal(
        &mut self,
        terminal: InlineCode1,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_inline_code_1_terminal_super(terminal, data, tree)
    }
    fn visit_inline_code_start_2_terminal(
        &mut self,
        terminal: InlineCodeStart2,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_inline_code_start_2_terminal_super(terminal, data, tree)
    }
    fn visit_code_block_start_3_terminal(
        &mut self,
        terminal: CodeBlockStart3,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_start_3_terminal_super(terminal, data, tree)
    }
    fn visit_code_block_start_4_terminal(
        &mut self,
        terminal: CodeBlockStart4,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_start_4_terminal_super(terminal, data, tree)
    }
    fn visit_code_block_start_5_terminal(
        &mut self,
        terminal: CodeBlockStart5,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_start_5_terminal_super(terminal, data, tree)
    }
    fn visit_code_block_start_6_terminal(
        &mut self,
        terminal: CodeBlockStart6,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_start_6_terminal_super(terminal, data, tree)
    }
    fn visit_code_block_end_3_terminal(
        &mut self,
        terminal: CodeBlockEnd3,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_end_3_terminal_super(terminal, data, tree)
    }
    fn visit_backtick_2_terminal(
        &mut self,
        terminal: Backtick2,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_backtick_2_terminal_super(terminal, data, tree)
    }
    fn visit_code_block_end_4_terminal(
        &mut self,
        terminal: CodeBlockEnd4,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_end_4_terminal_super(terminal, data, tree)
    }
    fn visit_backtick_3_terminal(
        &mut self,
        terminal: Backtick3,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_backtick_3_terminal_super(terminal, data, tree)
    }
    fn visit_code_block_end_5_terminal(
        &mut self,
        terminal: CodeBlockEnd5,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_end_5_terminal_super(terminal, data, tree)
    }
    fn visit_backtick_4_terminal(
        &mut self,
        terminal: Backtick4,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_backtick_4_terminal_super(terminal, data, tree)
    }
    fn visit_code_block_end_6_terminal(
        &mut self,
        terminal: CodeBlockEnd6,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_code_block_end_6_terminal_super(terminal, data, tree)
    }
    fn visit_backtick_5_terminal(
        &mut self,
        terminal: Backtick5,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_backtick_5_terminal_super(terminal, data, tree)
    }
    fn visit_inline_code_end_2_terminal(
        &mut self,
        terminal: InlineCodeEnd2,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_inline_code_end_2_terminal_super(terminal, data, tree)
    }
    fn visit_backtick_1_terminal(
        &mut self,
        terminal: Backtick1,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_backtick_1_terminal_super(terminal, data, tree)
    }
    fn visit_no_backtick_terminal(
        &mut self,
        terminal: NoBacktick,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_no_backtick_terminal_super(terminal, data, tree)
    }
    fn visit_no_backtick_inline_terminal(
        &mut self,
        terminal: NoBacktickInline,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_no_backtick_inline_terminal_super(terminal, data, tree)
    }
    fn visit_grammar_newline_terminal(
        &mut self,
        terminal: GrammarNewline,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_grammar_newline_terminal_super(terminal, data, tree)
    }
    fn visit_ws_terminal(
        &mut self,
        terminal: Ws,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_ws_terminal_super(terminal, data, tree)
    }
    fn visit_at_terminal(
        &mut self,
        terminal: At,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_at_terminal_super(terminal, data, tree)
    }
    fn visit_dollar_terminal(
        &mut self,
        terminal: Dollar,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_dollar_terminal_super(terminal, data, tree)
    }
    fn visit_dot_terminal(
        &mut self,
        terminal: Dot,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_dot_terminal_super(terminal, data, tree)
    }
    fn visit_l_brace_terminal(
        &mut self,
        terminal: LBrace,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_l_brace_terminal_super(terminal, data, tree)
    }
    fn visit_r_brace_terminal(
        &mut self,
        terminal: RBrace,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_r_brace_terminal_super(terminal, data, tree)
    }
    fn visit_l_bracket_terminal(
        &mut self,
        terminal: LBracket,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_l_bracket_terminal_super(terminal, data, tree)
    }
    fn visit_r_bracket_terminal(
        &mut self,
        terminal: RBracket,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_r_bracket_terminal_super(terminal, data, tree)
    }
    fn visit_l_paren_terminal(
        &mut self,
        terminal: LParen,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_l_paren_terminal_super(terminal, data, tree)
    }
    fn visit_r_paren_terminal(
        &mut self,
        terminal: RParen,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_r_paren_terminal_super(terminal, data, tree)
    }
    fn visit_bind_terminal(
        &mut self,
        terminal: Bind,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_bind_terminal_super(terminal, data, tree)
    }
    fn visit_comma_terminal(
        &mut self,
        terminal: Comma,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_comma_terminal_super(terminal, data, tree)
    }
    fn visit_esc_terminal(
        &mut self,
        terminal: Esc,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_esc_terminal_super(terminal, data, tree)
    }
    fn visit_text_start_terminal(
        &mut self,
        terminal: TextStart,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_text_start_terminal_super(terminal, data, tree)
    }
    fn visit_ident_terminal(
        &mut self,
        terminal: Ident,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_ident_terminal_super(terminal, data, tree)
    }
    fn visit_non_terminal(
        &mut self,
        id: CstNodeId,
        kind: NonTerminalKind,
        data: NonTerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_non_terminal_super(id, kind, data, tree)
    }
    fn visit_non_terminal_close(
        &mut self,
        id: CstNodeId,
        kind: NonTerminalKind,
        data: NonTerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_non_terminal_close_super(id, kind, data, tree)
    }
    fn visit_terminal(
        &mut self,
        id: CstNodeId,
        kind: TerminalKind,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_terminal_super(id, kind, data, tree)
    }
    /// This method is called when a construct view fails.
    /// If you return Ok(()), the error is not propagated.
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
mod private {
    pub trait Sealed<F> {}
}
pub trait CstVisitorSuper<F: CstFacade, E>: private::Sealed<F> {
    fn visit_array_handle(&mut self, handle: ArrayHandle, tree: &F) -> Result<(), E>;
    fn visit_array_super(
        &mut self,
        handle: ArrayHandle,
        view: ArrayView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_array_begin_handle(
        &mut self,
        handle: ArrayBeginHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_array_begin_super(
        &mut self,
        handle: ArrayBeginHandle,
        view: ArrayBeginView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_array_elements_handle(
        &mut self,
        handle: ArrayElementsHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_array_elements_super(
        &mut self,
        handle: ArrayElementsHandle,
        view: ArrayElementsView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_array_elements_opt_handle(
        &mut self,
        handle: ArrayElementsOptHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_array_elements_opt_super(
        &mut self,
        handle: ArrayElementsOptHandle,
        view: ArrayElementsTailHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_array_elements_tail_handle(
        &mut self,
        handle: ArrayElementsTailHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_array_elements_tail_super(
        &mut self,
        handle: ArrayElementsTailHandle,
        view: ArrayElementsTailView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_array_elements_tail_opt_handle(
        &mut self,
        handle: ArrayElementsTailOptHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_array_elements_tail_opt_super(
        &mut self,
        handle: ArrayElementsTailOptHandle,
        view: ArrayElementsHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_array_end_handle(
        &mut self,
        handle: ArrayEndHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_array_end_super(
        &mut self,
        handle: ArrayEndHandle,
        view: ArrayEndView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_array_marker_handle(
        &mut self,
        handle: ArrayMarkerHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_array_marker_super(
        &mut self,
        handle: ArrayMarkerHandle,
        view: ArrayMarkerView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_array_marker_opt_handle(
        &mut self,
        handle: ArrayMarkerOptHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_array_marker_opt_super(
        &mut self,
        handle: ArrayMarkerOptHandle,
        view: IntegerHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_array_opt_handle(
        &mut self,
        handle: ArrayOptHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_array_opt_super(
        &mut self,
        handle: ArrayOptHandle,
        view: ArrayElementsHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_at_handle(&mut self, handle: AtHandle, tree: &F) -> Result<(), E>;
    fn visit_at_super(
        &mut self,
        handle: AtHandle,
        view: AtView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_backtick_1_handle(
        &mut self,
        handle: Backtick1Handle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_backtick_1_super(
        &mut self,
        handle: Backtick1Handle,
        view: Backtick1View,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_backtick_2_handle(
        &mut self,
        handle: Backtick2Handle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_backtick_2_super(
        &mut self,
        handle: Backtick2Handle,
        view: Backtick2View,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_backtick_3_handle(
        &mut self,
        handle: Backtick3Handle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_backtick_3_super(
        &mut self,
        handle: Backtick3Handle,
        view: Backtick3View,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_backtick_4_handle(
        &mut self,
        handle: Backtick4Handle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_backtick_4_super(
        &mut self,
        handle: Backtick4Handle,
        view: Backtick4View,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_backtick_5_handle(
        &mut self,
        handle: Backtick5Handle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_backtick_5_super(
        &mut self,
        handle: Backtick5Handle,
        view: Backtick5View,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_begin_handle(&mut self, handle: BeginHandle, tree: &F) -> Result<(), E>;
    fn visit_begin_super(
        &mut self,
        handle: BeginHandle,
        view: BeginView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_bind_handle(&mut self, handle: BindHandle, tree: &F) -> Result<(), E>;
    fn visit_bind_super(
        &mut self,
        handle: BindHandle,
        view: BindView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_binding_handle(&mut self, handle: BindingHandle, tree: &F) -> Result<(), E>;
    fn visit_binding_super(
        &mut self,
        handle: BindingHandle,
        view: BindingView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_binding_rhs_handle(
        &mut self,
        handle: BindingRhsHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_binding_rhs_super(
        &mut self,
        handle: BindingRhsHandle,
        view: BindingRhsView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_boolean_handle(&mut self, handle: BooleanHandle, tree: &F) -> Result<(), E>;
    fn visit_boolean_super(
        &mut self,
        handle: BooleanHandle,
        view: BooleanView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_handle(
        &mut self,
        handle: CodeBlockHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_super(
        &mut self,
        handle: CodeBlockHandle,
        view: CodeBlockView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_3_handle(
        &mut self,
        handle: CodeBlock3Handle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_3_super(
        &mut self,
        handle: CodeBlock3Handle,
        view: CodeBlock3View,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_3_list_handle(
        &mut self,
        handle: CodeBlock3ListHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_3_list_super(
        &mut self,
        handle: CodeBlock3ListHandle,
        view: CodeBlock3ListView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_3_list_group_handle(
        &mut self,
        handle: CodeBlock3ListGroupHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_3_list_group_super(
        &mut self,
        handle: CodeBlock3ListGroupHandle,
        view: CodeBlock3ListGroupView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_4_handle(
        &mut self,
        handle: CodeBlock4Handle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_4_super(
        &mut self,
        handle: CodeBlock4Handle,
        view: CodeBlock4View,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_4_list_handle(
        &mut self,
        handle: CodeBlock4ListHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_4_list_super(
        &mut self,
        handle: CodeBlock4ListHandle,
        view: CodeBlock4ListView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_4_list_group_handle(
        &mut self,
        handle: CodeBlock4ListGroupHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_4_list_group_super(
        &mut self,
        handle: CodeBlock4ListGroupHandle,
        view: CodeBlock4ListGroupView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_5_handle(
        &mut self,
        handle: CodeBlock5Handle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_5_super(
        &mut self,
        handle: CodeBlock5Handle,
        view: CodeBlock5View,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_5_list_handle(
        &mut self,
        handle: CodeBlock5ListHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_5_list_super(
        &mut self,
        handle: CodeBlock5ListHandle,
        view: CodeBlock5ListView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_5_list_group_handle(
        &mut self,
        handle: CodeBlock5ListGroupHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_5_list_group_super(
        &mut self,
        handle: CodeBlock5ListGroupHandle,
        view: CodeBlock5ListGroupView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_6_handle(
        &mut self,
        handle: CodeBlock6Handle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_6_super(
        &mut self,
        handle: CodeBlock6Handle,
        view: CodeBlock6View,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_6_list_handle(
        &mut self,
        handle: CodeBlock6ListHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_6_list_super(
        &mut self,
        handle: CodeBlock6ListHandle,
        view: CodeBlock6ListView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_6_list_group_handle(
        &mut self,
        handle: CodeBlock6ListGroupHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_6_list_group_super(
        &mut self,
        handle: CodeBlock6ListGroupHandle,
        view: CodeBlock6ListGroupView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_end_3_handle(
        &mut self,
        handle: CodeBlockEnd3Handle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_end_3_super(
        &mut self,
        handle: CodeBlockEnd3Handle,
        view: CodeBlockEnd3View,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_end_4_handle(
        &mut self,
        handle: CodeBlockEnd4Handle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_end_4_super(
        &mut self,
        handle: CodeBlockEnd4Handle,
        view: CodeBlockEnd4View,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_end_5_handle(
        &mut self,
        handle: CodeBlockEnd5Handle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_end_5_super(
        &mut self,
        handle: CodeBlockEnd5Handle,
        view: CodeBlockEnd5View,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_end_6_handle(
        &mut self,
        handle: CodeBlockEnd6Handle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_end_6_super(
        &mut self,
        handle: CodeBlockEnd6Handle,
        view: CodeBlockEnd6View,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_start_3_handle(
        &mut self,
        handle: CodeBlockStart3Handle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_start_3_super(
        &mut self,
        handle: CodeBlockStart3Handle,
        view: CodeBlockStart3View,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_start_4_handle(
        &mut self,
        handle: CodeBlockStart4Handle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_start_4_super(
        &mut self,
        handle: CodeBlockStart4Handle,
        view: CodeBlockStart4View,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_start_5_handle(
        &mut self,
        handle: CodeBlockStart5Handle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_start_5_super(
        &mut self,
        handle: CodeBlockStart5Handle,
        view: CodeBlockStart5View,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_start_6_handle(
        &mut self,
        handle: CodeBlockStart6Handle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_start_6_super(
        &mut self,
        handle: CodeBlockStart6Handle,
        view: CodeBlockStart6View,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_comma_handle(&mut self, handle: CommaHandle, tree: &F) -> Result<(), E>;
    fn visit_comma_super(
        &mut self,
        handle: CommaHandle,
        view: CommaView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_continue_handle(
        &mut self,
        handle: ContinueHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_continue_super(
        &mut self,
        handle: ContinueHandle,
        view: ContinueView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_dot_handle(&mut self, handle: DotHandle, tree: &F) -> Result<(), E>;
    fn visit_dot_super(
        &mut self,
        handle: DotHandle,
        view: DotView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_end_handle(&mut self, handle: EndHandle, tree: &F) -> Result<(), E>;
    fn visit_end_super(
        &mut self,
        handle: EndHandle,
        view: EndView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_eure_handle(&mut self, handle: EureHandle, tree: &F) -> Result<(), E>;
    fn visit_eure_super(
        &mut self,
        handle: EureHandle,
        view: EureView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_eure_bindings_handle(
        &mut self,
        handle: EureBindingsHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_eure_bindings_super(
        &mut self,
        handle: EureBindingsHandle,
        view: EureBindingsView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_eure_sections_handle(
        &mut self,
        handle: EureSectionsHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_eure_sections_super(
        &mut self,
        handle: EureSectionsHandle,
        view: EureSectionsView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_eure_opt_handle(
        &mut self,
        handle: EureOptHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_eure_opt_super(
        &mut self,
        handle: EureOptHandle,
        view: ValueBindingHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_ext_handle(&mut self, handle: ExtHandle, tree: &F) -> Result<(), E>;
    fn visit_ext_super(
        &mut self,
        handle: ExtHandle,
        view: ExtView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_extension_name_space_handle(
        &mut self,
        handle: ExtensionNameSpaceHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_extension_name_space_super(
        &mut self,
        handle: ExtensionNameSpaceHandle,
        view: ExtensionNameSpaceView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_false_handle(&mut self, handle: FalseHandle, tree: &F) -> Result<(), E>;
    fn visit_false_super(
        &mut self,
        handle: FalseHandle,
        view: FalseView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_float_handle(&mut self, handle: FloatHandle, tree: &F) -> Result<(), E>;
    fn visit_float_super(
        &mut self,
        handle: FloatHandle,
        view: FloatView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_grammar_newline_handle(
        &mut self,
        handle: GrammarNewlineHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_grammar_newline_super(
        &mut self,
        handle: GrammarNewlineHandle,
        view: GrammarNewlineView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_hole_handle(&mut self, handle: HoleHandle, tree: &F) -> Result<(), E>;
    fn visit_hole_super(
        &mut self,
        handle: HoleHandle,
        view: HoleView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_ident_handle(&mut self, handle: IdentHandle, tree: &F) -> Result<(), E>;
    fn visit_ident_super(
        &mut self,
        handle: IdentHandle,
        view: IdentView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_inline_code_handle(
        &mut self,
        handle: InlineCodeHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_inline_code_super(
        &mut self,
        handle: InlineCodeHandle,
        view: InlineCodeView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_inline_code_1_handle(
        &mut self,
        handle: InlineCode1Handle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_inline_code_1_super(
        &mut self,
        handle: InlineCode1Handle,
        view: InlineCode1View,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_inline_code_2_handle(
        &mut self,
        handle: InlineCode2Handle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_inline_code_2_super(
        &mut self,
        handle: InlineCode2Handle,
        view: InlineCode2View,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_inline_code_2_list_handle(
        &mut self,
        handle: InlineCode2ListHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_inline_code_2_list_super(
        &mut self,
        handle: InlineCode2ListHandle,
        view: InlineCode2ListView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_inline_code_2_list_group_handle(
        &mut self,
        handle: InlineCode2ListGroupHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_inline_code_2_list_group_super(
        &mut self,
        handle: InlineCode2ListGroupHandle,
        view: InlineCode2ListGroupView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_inline_code_end_2_handle(
        &mut self,
        handle: InlineCodeEnd2Handle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_inline_code_end_2_super(
        &mut self,
        handle: InlineCodeEnd2Handle,
        view: InlineCodeEnd2View,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_inline_code_start_2_handle(
        &mut self,
        handle: InlineCodeStart2Handle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_inline_code_start_2_super(
        &mut self,
        handle: InlineCodeStart2Handle,
        view: InlineCodeStart2View,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_integer_handle(&mut self, handle: IntegerHandle, tree: &F) -> Result<(), E>;
    fn visit_integer_super(
        &mut self,
        handle: IntegerHandle,
        view: IntegerView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_key_handle(&mut self, handle: KeyHandle, tree: &F) -> Result<(), E>;
    fn visit_key_super(
        &mut self,
        handle: KeyHandle,
        view: KeyView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_key_base_handle(
        &mut self,
        handle: KeyBaseHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_key_base_super(
        &mut self,
        handle: KeyBaseHandle,
        view: KeyBaseView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_key_ident_handle(
        &mut self,
        handle: KeyIdentHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_key_ident_super(
        &mut self,
        handle: KeyIdentHandle,
        view: KeyIdentView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_key_opt_handle(&mut self, handle: KeyOptHandle, tree: &F) -> Result<(), E>;
    fn visit_key_opt_super(
        &mut self,
        handle: KeyOptHandle,
        view: ArrayMarkerHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_key_tuple_handle(
        &mut self,
        handle: KeyTupleHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_key_tuple_super(
        &mut self,
        handle: KeyTupleHandle,
        view: KeyTupleView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_key_tuple_elements_handle(
        &mut self,
        handle: KeyTupleElementsHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_key_tuple_elements_super(
        &mut self,
        handle: KeyTupleElementsHandle,
        view: KeyTupleElementsView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_key_tuple_elements_opt_handle(
        &mut self,
        handle: KeyTupleElementsOptHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_key_tuple_elements_opt_super(
        &mut self,
        handle: KeyTupleElementsOptHandle,
        view: KeyTupleElementsTailHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_key_tuple_elements_tail_handle(
        &mut self,
        handle: KeyTupleElementsTailHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_key_tuple_elements_tail_super(
        &mut self,
        handle: KeyTupleElementsTailHandle,
        view: KeyTupleElementsTailView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_key_tuple_elements_tail_opt_handle(
        &mut self,
        handle: KeyTupleElementsTailOptHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_key_tuple_elements_tail_opt_super(
        &mut self,
        handle: KeyTupleElementsTailOptHandle,
        view: KeyTupleElementsHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_key_tuple_opt_handle(
        &mut self,
        handle: KeyTupleOptHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_key_tuple_opt_super(
        &mut self,
        handle: KeyTupleOptHandle,
        view: KeyTupleElementsHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_key_value_handle(
        &mut self,
        handle: KeyValueHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_key_value_super(
        &mut self,
        handle: KeyValueHandle,
        view: KeyValueView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_keys_handle(&mut self, handle: KeysHandle, tree: &F) -> Result<(), E>;
    fn visit_keys_super(
        &mut self,
        handle: KeysHandle,
        view: KeysView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_keys_list_handle(
        &mut self,
        handle: KeysListHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_keys_list_super(
        &mut self,
        handle: KeysListHandle,
        view: KeysListView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_l_paren_handle(&mut self, handle: LParenHandle, tree: &F) -> Result<(), E>;
    fn visit_l_paren_super(
        &mut self,
        handle: LParenHandle,
        view: LParenView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_no_backtick_handle(
        &mut self,
        handle: NoBacktickHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_no_backtick_super(
        &mut self,
        handle: NoBacktickHandle,
        view: NoBacktickView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_no_backtick_inline_handle(
        &mut self,
        handle: NoBacktickInlineHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_no_backtick_inline_super(
        &mut self,
        handle: NoBacktickInlineHandle,
        view: NoBacktickInlineView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_null_handle(&mut self, handle: NullHandle, tree: &F) -> Result<(), E>;
    fn visit_null_super(
        &mut self,
        handle: NullHandle,
        view: NullView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_object_handle(&mut self, handle: ObjectHandle, tree: &F) -> Result<(), E>;
    fn visit_object_super(
        &mut self,
        handle: ObjectHandle,
        view: ObjectView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_object_list_handle(
        &mut self,
        handle: ObjectListHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_object_list_super(
        &mut self,
        handle: ObjectListHandle,
        view: ObjectListView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_object_opt_handle(
        &mut self,
        handle: ObjectOptHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_object_opt_super(
        &mut self,
        handle: ObjectOptHandle,
        view: CommaHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_path_handle(&mut self, handle: PathHandle, tree: &F) -> Result<(), E>;
    fn visit_path_super(
        &mut self,
        handle: PathHandle,
        view: PathView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_r_paren_handle(&mut self, handle: RParenHandle, tree: &F) -> Result<(), E>;
    fn visit_r_paren_super(
        &mut self,
        handle: RParenHandle,
        view: RParenView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_section_handle(&mut self, handle: SectionHandle, tree: &F) -> Result<(), E>;
    fn visit_section_super(
        &mut self,
        handle: SectionHandle,
        view: SectionView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_section_binding_handle(
        &mut self,
        handle: SectionBindingHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_section_binding_super(
        &mut self,
        handle: SectionBindingHandle,
        view: SectionBindingView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_section_body_handle(
        &mut self,
        handle: SectionBodyHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_section_body_super(
        &mut self,
        handle: SectionBodyHandle,
        view: SectionBodyView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_section_body_list_handle(
        &mut self,
        handle: SectionBodyListHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_section_body_list_super(
        &mut self,
        handle: SectionBodyListHandle,
        view: SectionBodyListView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_section_body_opt_handle(
        &mut self,
        handle: SectionBodyOptHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_section_body_opt_super(
        &mut self,
        handle: SectionBodyOptHandle,
        view: ValueBindingHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_str_handle(&mut self, handle: StrHandle, tree: &F) -> Result<(), E>;
    fn visit_str_super(
        &mut self,
        handle: StrHandle,
        view: StrView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_strings_handle(&mut self, handle: StringsHandle, tree: &F) -> Result<(), E>;
    fn visit_strings_super(
        &mut self,
        handle: StringsHandle,
        view: StringsView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_strings_list_handle(
        &mut self,
        handle: StringsListHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_strings_list_super(
        &mut self,
        handle: StringsListHandle,
        view: StringsListView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_text_handle(&mut self, handle: TextHandle, tree: &F) -> Result<(), E>;
    fn visit_text_super(
        &mut self,
        handle: TextHandle,
        view: TextView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_text_binding_handle(
        &mut self,
        handle: TextBindingHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_text_binding_super(
        &mut self,
        handle: TextBindingHandle,
        view: TextBindingView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_text_binding_opt_handle(
        &mut self,
        handle: TextBindingOptHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_text_binding_opt_super(
        &mut self,
        handle: TextBindingOptHandle,
        view: WsHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_text_binding_opt_0_handle(
        &mut self,
        handle: TextBindingOpt0Handle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_text_binding_opt_0_super(
        &mut self,
        handle: TextBindingOpt0Handle,
        view: GrammarNewlineHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_text_start_handle(
        &mut self,
        handle: TextStartHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_text_start_super(
        &mut self,
        handle: TextStartHandle,
        view: TextStartView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_true_handle(&mut self, handle: TrueHandle, tree: &F) -> Result<(), E>;
    fn visit_true_super(
        &mut self,
        handle: TrueHandle,
        view: TrueView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_tuple_handle(&mut self, handle: TupleHandle, tree: &F) -> Result<(), E>;
    fn visit_tuple_super(
        &mut self,
        handle: TupleHandle,
        view: TupleView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_tuple_elements_handle(
        &mut self,
        handle: TupleElementsHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_tuple_elements_super(
        &mut self,
        handle: TupleElementsHandle,
        view: TupleElementsView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_tuple_elements_opt_handle(
        &mut self,
        handle: TupleElementsOptHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_tuple_elements_opt_super(
        &mut self,
        handle: TupleElementsOptHandle,
        view: TupleElementsTailHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_tuple_elements_tail_handle(
        &mut self,
        handle: TupleElementsTailHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_tuple_elements_tail_super(
        &mut self,
        handle: TupleElementsTailHandle,
        view: TupleElementsTailView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_tuple_elements_tail_opt_handle(
        &mut self,
        handle: TupleElementsTailOptHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_tuple_elements_tail_opt_super(
        &mut self,
        handle: TupleElementsTailOptHandle,
        view: TupleElementsHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_tuple_index_handle(
        &mut self,
        handle: TupleIndexHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_tuple_index_super(
        &mut self,
        handle: TupleIndexHandle,
        view: TupleIndexView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_tuple_opt_handle(
        &mut self,
        handle: TupleOptHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_tuple_opt_super(
        &mut self,
        handle: TupleOptHandle,
        view: TupleElementsHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_value_handle(&mut self, handle: ValueHandle, tree: &F) -> Result<(), E>;
    fn visit_value_super(
        &mut self,
        handle: ValueHandle,
        view: ValueView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_value_binding_handle(
        &mut self,
        handle: ValueBindingHandle,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_value_binding_super(
        &mut self,
        handle: ValueBindingHandle,
        view: ValueBindingView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_ws_handle(&mut self, handle: WsHandle, tree: &F) -> Result<(), E>;
    fn visit_ws_super(
        &mut self,
        handle: WsHandle,
        view: WsView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_root_handle(&mut self, handle: RootHandle, tree: &F) -> Result<(), E>;
    fn visit_root_super(
        &mut self,
        handle: RootHandle,
        view: RootView,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_new_line_terminal_super(
        &mut self,
        terminal: NewLine,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_whitespace_terminal_super(
        &mut self,
        terminal: Whitespace,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_line_comment_terminal_super(
        &mut self,
        terminal: LineComment,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_block_comment_terminal_super(
        &mut self,
        terminal: BlockComment,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_hash_terminal_super(
        &mut self,
        terminal: Hash,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_float_terminal_super(
        &mut self,
        terminal: Float,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_integer_terminal_super(
        &mut self,
        terminal: Integer,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_true_terminal_super(
        &mut self,
        terminal: True,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_false_terminal_super(
        &mut self,
        terminal: False,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_null_terminal_super(
        &mut self,
        terminal: Null,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_hole_terminal_super(
        &mut self,
        terminal: Hole,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_str_terminal_super(
        &mut self,
        terminal: Str,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_text_terminal_super(
        &mut self,
        terminal: Text,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_inline_code_1_terminal_super(
        &mut self,
        terminal: InlineCode1,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_inline_code_start_2_terminal_super(
        &mut self,
        terminal: InlineCodeStart2,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_start_3_terminal_super(
        &mut self,
        terminal: CodeBlockStart3,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_start_4_terminal_super(
        &mut self,
        terminal: CodeBlockStart4,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_start_5_terminal_super(
        &mut self,
        terminal: CodeBlockStart5,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_start_6_terminal_super(
        &mut self,
        terminal: CodeBlockStart6,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_end_3_terminal_super(
        &mut self,
        terminal: CodeBlockEnd3,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_backtick_2_terminal_super(
        &mut self,
        terminal: Backtick2,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_end_4_terminal_super(
        &mut self,
        terminal: CodeBlockEnd4,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_backtick_3_terminal_super(
        &mut self,
        terminal: Backtick3,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_end_5_terminal_super(
        &mut self,
        terminal: CodeBlockEnd5,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_backtick_4_terminal_super(
        &mut self,
        terminal: Backtick4,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_code_block_end_6_terminal_super(
        &mut self,
        terminal: CodeBlockEnd6,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_backtick_5_terminal_super(
        &mut self,
        terminal: Backtick5,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_inline_code_end_2_terminal_super(
        &mut self,
        terminal: InlineCodeEnd2,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_backtick_1_terminal_super(
        &mut self,
        terminal: Backtick1,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_no_backtick_terminal_super(
        &mut self,
        terminal: NoBacktick,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_no_backtick_inline_terminal_super(
        &mut self,
        terminal: NoBacktickInline,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_grammar_newline_terminal_super(
        &mut self,
        terminal: GrammarNewline,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_ws_terminal_super(
        &mut self,
        terminal: Ws,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_at_terminal_super(
        &mut self,
        terminal: At,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_dollar_terminal_super(
        &mut self,
        terminal: Dollar,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_dot_terminal_super(
        &mut self,
        terminal: Dot,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_l_brace_terminal_super(
        &mut self,
        terminal: LBrace,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_r_brace_terminal_super(
        &mut self,
        terminal: RBrace,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_l_bracket_terminal_super(
        &mut self,
        terminal: LBracket,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_r_bracket_terminal_super(
        &mut self,
        terminal: RBracket,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_l_paren_terminal_super(
        &mut self,
        terminal: LParen,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_r_paren_terminal_super(
        &mut self,
        terminal: RParen,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_bind_terminal_super(
        &mut self,
        terminal: Bind,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_comma_terminal_super(
        &mut self,
        terminal: Comma,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_esc_terminal_super(
        &mut self,
        terminal: Esc,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_text_start_terminal_super(
        &mut self,
        terminal: TextStart,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_ident_terminal_super(
        &mut self,
        terminal: Ident,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_non_terminal_super(
        &mut self,
        id: CstNodeId,
        kind: NonTerminalKind,
        data: NonTerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_non_terminal_close_super(
        &mut self,
        id: CstNodeId,
        kind: NonTerminalKind,
        data: NonTerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_terminal_super(
        &mut self,
        id: CstNodeId,
        kind: TerminalKind,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_any(&mut self, id: CstNodeId, node: CstNode, tree: &F) -> Result<(), E>;
    /// Recover from a construct error. This eagerly visits the children of the node.
    fn recover_error(
        &mut self,
        node_data: Option<CstNode>,
        id: CstNodeId,
        kind: NodeKind,
        tree: &F,
    ) -> Result<(), E>;
}
impl<V: CstVisitor<F>, F: CstFacade> private::Sealed<F> for V {}
impl<V: CstVisitor<F>, F: CstFacade> CstVisitorSuper<F, V::Error> for V {
    fn visit_array_handle(
        &mut self,
        handle: ArrayHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (visit.visit_array(handle, view, tree), visit),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_array_begin_handle(
        &mut self,
        handle: ArrayBeginHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_array_begin(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_array_elements_handle(
        &mut self,
        handle: ArrayElementsHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_array_elements(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_array_elements_opt_handle(
        &mut self,
        handle: ArrayElementsOptHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    if let Some(view) = view {
                        visit.visit_array_elements_opt(handle, view, tree)
                    } else {
                        Ok(())
                    },
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_array_elements_tail_handle(
        &mut self,
        handle: ArrayElementsTailHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_array_elements_tail(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_array_elements_tail_opt_handle(
        &mut self,
        handle: ArrayElementsTailOptHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    if let Some(view) = view {
                        visit.visit_array_elements_tail_opt(handle, view, tree)
                    } else {
                        Ok(())
                    },
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_array_end_handle(
        &mut self,
        handle: ArrayEndHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_array_end(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_array_marker_handle(
        &mut self,
        handle: ArrayMarkerHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_array_marker(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_array_marker_opt_handle(
        &mut self,
        handle: ArrayMarkerOptHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    if let Some(view) = view {
                        visit.visit_array_marker_opt(handle, view, tree)
                    } else {
                        Ok(())
                    },
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_array_opt_handle(
        &mut self,
        handle: ArrayOptHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    if let Some(view) = view {
                        visit.visit_array_opt(handle, view, tree)
                    } else {
                        Ok(())
                    },
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_at_handle(&mut self, handle: AtHandle, tree: &F) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (visit.visit_at(handle, view, tree), visit),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_backtick_1_handle(
        &mut self,
        handle: Backtick1Handle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_backtick_1(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_backtick_2_handle(
        &mut self,
        handle: Backtick2Handle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_backtick_2(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_backtick_3_handle(
        &mut self,
        handle: Backtick3Handle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_backtick_3(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_backtick_4_handle(
        &mut self,
        handle: Backtick4Handle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_backtick_4(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_backtick_5_handle(
        &mut self,
        handle: Backtick5Handle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_backtick_5(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_begin_handle(
        &mut self,
        handle: BeginHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (visit.visit_begin(handle, view, tree), visit),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_bind_handle(
        &mut self,
        handle: BindHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (visit.visit_bind(handle, view, tree), visit),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_binding_handle(
        &mut self,
        handle: BindingHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_binding(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_binding_rhs_handle(
        &mut self,
        handle: BindingRhsHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_binding_rhs(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_boolean_handle(
        &mut self,
        handle: BooleanHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_boolean(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_code_block_handle(
        &mut self,
        handle: CodeBlockHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_code_block(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_code_block_3_handle(
        &mut self,
        handle: CodeBlock3Handle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_code_block_3(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_code_block_3_list_handle(
        &mut self,
        handle: CodeBlock3ListHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    if let Some(view) = view {
                        visit.visit_code_block_3_list(handle, view, tree)
                    } else {
                        Ok(())
                    },
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_code_block_3_list_group_handle(
        &mut self,
        handle: CodeBlock3ListGroupHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_code_block_3_list_group(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_code_block_4_handle(
        &mut self,
        handle: CodeBlock4Handle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_code_block_4(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_code_block_4_list_handle(
        &mut self,
        handle: CodeBlock4ListHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    if let Some(view) = view {
                        visit.visit_code_block_4_list(handle, view, tree)
                    } else {
                        Ok(())
                    },
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_code_block_4_list_group_handle(
        &mut self,
        handle: CodeBlock4ListGroupHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_code_block_4_list_group(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_code_block_5_handle(
        &mut self,
        handle: CodeBlock5Handle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_code_block_5(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_code_block_5_list_handle(
        &mut self,
        handle: CodeBlock5ListHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    if let Some(view) = view {
                        visit.visit_code_block_5_list(handle, view, tree)
                    } else {
                        Ok(())
                    },
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_code_block_5_list_group_handle(
        &mut self,
        handle: CodeBlock5ListGroupHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_code_block_5_list_group(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_code_block_6_handle(
        &mut self,
        handle: CodeBlock6Handle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_code_block_6(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_code_block_6_list_handle(
        &mut self,
        handle: CodeBlock6ListHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    if let Some(view) = view {
                        visit.visit_code_block_6_list(handle, view, tree)
                    } else {
                        Ok(())
                    },
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_code_block_6_list_group_handle(
        &mut self,
        handle: CodeBlock6ListGroupHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_code_block_6_list_group(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_code_block_end_3_handle(
        &mut self,
        handle: CodeBlockEnd3Handle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_code_block_end_3(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_code_block_end_4_handle(
        &mut self,
        handle: CodeBlockEnd4Handle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_code_block_end_4(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_code_block_end_5_handle(
        &mut self,
        handle: CodeBlockEnd5Handle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_code_block_end_5(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_code_block_end_6_handle(
        &mut self,
        handle: CodeBlockEnd6Handle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_code_block_end_6(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_code_block_start_3_handle(
        &mut self,
        handle: CodeBlockStart3Handle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_code_block_start_3(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_code_block_start_4_handle(
        &mut self,
        handle: CodeBlockStart4Handle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_code_block_start_4(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_code_block_start_5_handle(
        &mut self,
        handle: CodeBlockStart5Handle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_code_block_start_5(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_code_block_start_6_handle(
        &mut self,
        handle: CodeBlockStart6Handle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_code_block_start_6(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_comma_handle(
        &mut self,
        handle: CommaHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (visit.visit_comma(handle, view, tree), visit),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_continue_handle(
        &mut self,
        handle: ContinueHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_continue(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_dot_handle(&mut self, handle: DotHandle, tree: &F) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (visit.visit_dot(handle, view, tree), visit),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_end_handle(&mut self, handle: EndHandle, tree: &F) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (visit.visit_end(handle, view, tree), visit),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_eure_handle(
        &mut self,
        handle: EureHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (visit.visit_eure(handle, view, tree), visit),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_eure_bindings_handle(
        &mut self,
        handle: EureBindingsHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    if let Some(view) = view {
                        visit.visit_eure_bindings(handle, view, tree)
                    } else {
                        Ok(())
                    },
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_eure_sections_handle(
        &mut self,
        handle: EureSectionsHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    if let Some(view) = view {
                        visit.visit_eure_sections(handle, view, tree)
                    } else {
                        Ok(())
                    },
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_eure_opt_handle(
        &mut self,
        handle: EureOptHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    if let Some(view) = view {
                        visit.visit_eure_opt(handle, view, tree)
                    } else {
                        Ok(())
                    },
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_ext_handle(&mut self, handle: ExtHandle, tree: &F) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (visit.visit_ext(handle, view, tree), visit),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_extension_name_space_handle(
        &mut self,
        handle: ExtensionNameSpaceHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_extension_name_space(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_false_handle(
        &mut self,
        handle: FalseHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (visit.visit_false(handle, view, tree), visit),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_float_handle(
        &mut self,
        handle: FloatHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (visit.visit_float(handle, view, tree), visit),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_grammar_newline_handle(
        &mut self,
        handle: GrammarNewlineHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_grammar_newline(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_hole_handle(
        &mut self,
        handle: HoleHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (visit.visit_hole(handle, view, tree), visit),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_ident_handle(
        &mut self,
        handle: IdentHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (visit.visit_ident(handle, view, tree), visit),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_inline_code_handle(
        &mut self,
        handle: InlineCodeHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_inline_code(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_inline_code_1_handle(
        &mut self,
        handle: InlineCode1Handle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_inline_code_1(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_inline_code_2_handle(
        &mut self,
        handle: InlineCode2Handle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_inline_code_2(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_inline_code_2_list_handle(
        &mut self,
        handle: InlineCode2ListHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    if let Some(view) = view {
                        visit.visit_inline_code_2_list(handle, view, tree)
                    } else {
                        Ok(())
                    },
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_inline_code_2_list_group_handle(
        &mut self,
        handle: InlineCode2ListGroupHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_inline_code_2_list_group(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_inline_code_end_2_handle(
        &mut self,
        handle: InlineCodeEnd2Handle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_inline_code_end_2(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_inline_code_start_2_handle(
        &mut self,
        handle: InlineCodeStart2Handle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_inline_code_start_2(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_integer_handle(
        &mut self,
        handle: IntegerHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_integer(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_key_handle(&mut self, handle: KeyHandle, tree: &F) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (visit.visit_key(handle, view, tree), visit),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_key_base_handle(
        &mut self,
        handle: KeyBaseHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_key_base(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_key_ident_handle(
        &mut self,
        handle: KeyIdentHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_key_ident(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_key_opt_handle(
        &mut self,
        handle: KeyOptHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    if let Some(view) = view {
                        visit.visit_key_opt(handle, view, tree)
                    } else {
                        Ok(())
                    },
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_key_tuple_handle(
        &mut self,
        handle: KeyTupleHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_key_tuple(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_key_tuple_elements_handle(
        &mut self,
        handle: KeyTupleElementsHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_key_tuple_elements(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_key_tuple_elements_opt_handle(
        &mut self,
        handle: KeyTupleElementsOptHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    if let Some(view) = view {
                        visit.visit_key_tuple_elements_opt(handle, view, tree)
                    } else {
                        Ok(())
                    },
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_key_tuple_elements_tail_handle(
        &mut self,
        handle: KeyTupleElementsTailHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_key_tuple_elements_tail(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_key_tuple_elements_tail_opt_handle(
        &mut self,
        handle: KeyTupleElementsTailOptHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    if let Some(view) = view {
                        visit.visit_key_tuple_elements_tail_opt(handle, view, tree)
                    } else {
                        Ok(())
                    },
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_key_tuple_opt_handle(
        &mut self,
        handle: KeyTupleOptHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    if let Some(view) = view {
                        visit.visit_key_tuple_opt(handle, view, tree)
                    } else {
                        Ok(())
                    },
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_key_value_handle(
        &mut self,
        handle: KeyValueHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_key_value(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_keys_handle(
        &mut self,
        handle: KeysHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (visit.visit_keys(handle, view, tree), visit),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_keys_list_handle(
        &mut self,
        handle: KeysListHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    if let Some(view) = view {
                        visit.visit_keys_list(handle, view, tree)
                    } else {
                        Ok(())
                    },
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_l_paren_handle(
        &mut self,
        handle: LParenHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_l_paren(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_no_backtick_handle(
        &mut self,
        handle: NoBacktickHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_no_backtick(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_no_backtick_inline_handle(
        &mut self,
        handle: NoBacktickInlineHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_no_backtick_inline(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_null_handle(
        &mut self,
        handle: NullHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (visit.visit_null(handle, view, tree), visit),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_object_handle(
        &mut self,
        handle: ObjectHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (visit.visit_object(handle, view, tree), visit),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_object_list_handle(
        &mut self,
        handle: ObjectListHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    if let Some(view) = view {
                        visit.visit_object_list(handle, view, tree)
                    } else {
                        Ok(())
                    },
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_object_opt_handle(
        &mut self,
        handle: ObjectOptHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    if let Some(view) = view {
                        visit.visit_object_opt(handle, view, tree)
                    } else {
                        Ok(())
                    },
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_path_handle(
        &mut self,
        handle: PathHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (visit.visit_path(handle, view, tree), visit),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_r_paren_handle(
        &mut self,
        handle: RParenHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_r_paren(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_section_handle(
        &mut self,
        handle: SectionHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_section(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_section_binding_handle(
        &mut self,
        handle: SectionBindingHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_section_binding(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_section_body_handle(
        &mut self,
        handle: SectionBodyHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_section_body(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_section_body_list_handle(
        &mut self,
        handle: SectionBodyListHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    if let Some(view) = view {
                        visit.visit_section_body_list(handle, view, tree)
                    } else {
                        Ok(())
                    },
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_section_body_opt_handle(
        &mut self,
        handle: SectionBodyOptHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    if let Some(view) = view {
                        visit.visit_section_body_opt(handle, view, tree)
                    } else {
                        Ok(())
                    },
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_str_handle(&mut self, handle: StrHandle, tree: &F) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (visit.visit_str(handle, view, tree), visit),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_strings_handle(
        &mut self,
        handle: StringsHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_strings(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_strings_list_handle(
        &mut self,
        handle: StringsListHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    if let Some(view) = view {
                        visit.visit_strings_list(handle, view, tree)
                    } else {
                        Ok(())
                    },
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_text_handle(
        &mut self,
        handle: TextHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (visit.visit_text(handle, view, tree), visit),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_text_binding_handle(
        &mut self,
        handle: TextBindingHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_text_binding(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_text_binding_opt_handle(
        &mut self,
        handle: TextBindingOptHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    if let Some(view) = view {
                        visit.visit_text_binding_opt(handle, view, tree)
                    } else {
                        Ok(())
                    },
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_text_binding_opt_0_handle(
        &mut self,
        handle: TextBindingOpt0Handle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    if let Some(view) = view {
                        visit.visit_text_binding_opt_0(handle, view, tree)
                    } else {
                        Ok(())
                    },
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_text_start_handle(
        &mut self,
        handle: TextStartHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_text_start(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_true_handle(
        &mut self,
        handle: TrueHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (visit.visit_true(handle, view, tree), visit),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_tuple_handle(
        &mut self,
        handle: TupleHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (visit.visit_tuple(handle, view, tree), visit),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_tuple_elements_handle(
        &mut self,
        handle: TupleElementsHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_tuple_elements(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_tuple_elements_opt_handle(
        &mut self,
        handle: TupleElementsOptHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    if let Some(view) = view {
                        visit.visit_tuple_elements_opt(handle, view, tree)
                    } else {
                        Ok(())
                    },
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_tuple_elements_tail_handle(
        &mut self,
        handle: TupleElementsTailHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_tuple_elements_tail(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_tuple_elements_tail_opt_handle(
        &mut self,
        handle: TupleElementsTailOptHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    if let Some(view) = view {
                        visit.visit_tuple_elements_tail_opt(handle, view, tree)
                    } else {
                        Ok(())
                    },
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_tuple_index_handle(
        &mut self,
        handle: TupleIndexHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_tuple_index(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_tuple_opt_handle(
        &mut self,
        handle: TupleOptHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    if let Some(view) = view {
                        visit.visit_tuple_opt(handle, view, tree)
                    } else {
                        Ok(())
                    },
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_value_handle(
        &mut self,
        handle: ValueHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (visit.visit_value(handle, view, tree), visit),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_value_binding_handle(
        &mut self,
        handle: ValueBindingHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (
                    visit.visit_value_binding(handle, view, tree),
                    visit,
                ),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_ws_handle(&mut self, handle: WsHandle, tree: &F) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (visit.visit_ws(handle, view, tree), visit),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_root_handle(
        &mut self,
        handle: RootHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let nt_data = match tree.get_non_terminal(handle.node_id(), handle.kind()) {
            Ok(nt_data) => nt_data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        handle.node_id(),
                        NodeKind::NonTerminal(handle.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_non_terminal(handle.node_id(), handle.kind(), nt_data, tree)?;
        let result = match handle
            .get_view_with_visit(
                tree,
                |view, visit: &mut Self| (visit.visit_root(handle, view, tree), visit),
                self,
            )
            .map_err(|e| e.extract_error())
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(Ok(e)) => Err(e),
            Err(Err(e)) => {
                self.then_construct_error(
                    Some(CstNode::new_non_terminal(handle.kind(), nt_data)),
                    handle.node_id(),
                    NodeKind::NonTerminal(handle.kind()),
                    e,
                    tree,
                )
            }
        };
        self.visit_non_terminal_close(handle.node_id(), handle.kind(), nt_data, tree)?;
        result
    }
    fn visit_array_super(
        &mut self,
        handle: ArrayHandle,
        view_param: ArrayView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let ArrayView { array_begin, array_opt, array_end } = view_param;
        self.visit_array_begin_handle(array_begin, tree)?;
        self.visit_array_opt_handle(array_opt, tree)?;
        self.visit_array_end_handle(array_end, tree)?;
        Ok(())
    }
    fn visit_array_begin_super(
        &mut self,
        handle: ArrayBeginHandle,
        view_param: ArrayBeginView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let ArrayBeginView { l_bracket } = view_param;
        let data = match l_bracket.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        l_bracket.0,
                        NodeKind::Terminal(l_bracket.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_l_bracket_terminal(l_bracket, data, tree)?;
        Ok(())
    }
    fn visit_array_elements_super(
        &mut self,
        handle: ArrayElementsHandle,
        view_param: ArrayElementsView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let ArrayElementsView { value, array_elements_opt } = view_param;
        self.visit_value_handle(value, tree)?;
        self.visit_array_elements_opt_handle(array_elements_opt, tree)?;
        Ok(())
    }
    fn visit_array_elements_opt_super(
        &mut self,
        handle: ArrayElementsOptHandle,
        view_param: ArrayElementsTailHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        self.visit_array_elements_tail_handle(view_param, tree)?;
        Ok(())
    }
    fn visit_array_elements_tail_super(
        &mut self,
        handle: ArrayElementsTailHandle,
        view_param: ArrayElementsTailView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let ArrayElementsTailView { comma, array_elements_tail_opt } = view_param;
        self.visit_comma_handle(comma, tree)?;
        self.visit_array_elements_tail_opt_handle(array_elements_tail_opt, tree)?;
        Ok(())
    }
    fn visit_array_elements_tail_opt_super(
        &mut self,
        handle: ArrayElementsTailOptHandle,
        view_param: ArrayElementsHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        self.visit_array_elements_handle(view_param, tree)?;
        Ok(())
    }
    fn visit_array_end_super(
        &mut self,
        handle: ArrayEndHandle,
        view_param: ArrayEndView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let ArrayEndView { r_bracket } = view_param;
        let data = match r_bracket.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        r_bracket.0,
                        NodeKind::Terminal(r_bracket.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_r_bracket_terminal(r_bracket, data, tree)?;
        Ok(())
    }
    fn visit_array_marker_super(
        &mut self,
        handle: ArrayMarkerHandle,
        view_param: ArrayMarkerView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let ArrayMarkerView { array_begin, array_marker_opt, array_end } = view_param;
        self.visit_array_begin_handle(array_begin, tree)?;
        self.visit_array_marker_opt_handle(array_marker_opt, tree)?;
        self.visit_array_end_handle(array_end, tree)?;
        Ok(())
    }
    fn visit_array_marker_opt_super(
        &mut self,
        handle: ArrayMarkerOptHandle,
        view_param: IntegerHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        self.visit_integer_handle(view_param, tree)?;
        Ok(())
    }
    fn visit_array_opt_super(
        &mut self,
        handle: ArrayOptHandle,
        view_param: ArrayElementsHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        self.visit_array_elements_handle(view_param, tree)?;
        Ok(())
    }
    fn visit_at_super(
        &mut self,
        handle: AtHandle,
        view_param: AtView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let AtView { at } = view_param;
        let data = match at.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        at.0,
                        NodeKind::Terminal(at.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_at_terminal(at, data, tree)?;
        Ok(())
    }
    fn visit_backtick_1_super(
        &mut self,
        handle: Backtick1Handle,
        view_param: Backtick1View,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let Backtick1View { backtick_1 } = view_param;
        let data = match backtick_1.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        backtick_1.0,
                        NodeKind::Terminal(backtick_1.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_backtick_1_terminal(backtick_1, data, tree)?;
        Ok(())
    }
    fn visit_backtick_2_super(
        &mut self,
        handle: Backtick2Handle,
        view_param: Backtick2View,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let Backtick2View { backtick_2 } = view_param;
        let data = match backtick_2.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        backtick_2.0,
                        NodeKind::Terminal(backtick_2.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_backtick_2_terminal(backtick_2, data, tree)?;
        Ok(())
    }
    fn visit_backtick_3_super(
        &mut self,
        handle: Backtick3Handle,
        view_param: Backtick3View,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let Backtick3View { backtick_3 } = view_param;
        let data = match backtick_3.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        backtick_3.0,
                        NodeKind::Terminal(backtick_3.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_backtick_3_terminal(backtick_3, data, tree)?;
        Ok(())
    }
    fn visit_backtick_4_super(
        &mut self,
        handle: Backtick4Handle,
        view_param: Backtick4View,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let Backtick4View { backtick_4 } = view_param;
        let data = match backtick_4.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        backtick_4.0,
                        NodeKind::Terminal(backtick_4.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_backtick_4_terminal(backtick_4, data, tree)?;
        Ok(())
    }
    fn visit_backtick_5_super(
        &mut self,
        handle: Backtick5Handle,
        view_param: Backtick5View,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let Backtick5View { backtick_5 } = view_param;
        let data = match backtick_5.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        backtick_5.0,
                        NodeKind::Terminal(backtick_5.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_backtick_5_terminal(backtick_5, data, tree)?;
        Ok(())
    }
    fn visit_begin_super(
        &mut self,
        handle: BeginHandle,
        view_param: BeginView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let BeginView { l_brace } = view_param;
        let data = match l_brace.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        l_brace.0,
                        NodeKind::Terminal(l_brace.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_l_brace_terminal(l_brace, data, tree)?;
        Ok(())
    }
    fn visit_bind_super(
        &mut self,
        handle: BindHandle,
        view_param: BindView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let BindView { bind } = view_param;
        let data = match bind.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        bind.0,
                        NodeKind::Terminal(bind.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_bind_terminal(bind, data, tree)?;
        Ok(())
    }
    fn visit_binding_super(
        &mut self,
        handle: BindingHandle,
        view_param: BindingView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let BindingView { keys, binding_rhs } = view_param;
        self.visit_keys_handle(keys, tree)?;
        self.visit_binding_rhs_handle(binding_rhs, tree)?;
        Ok(())
    }
    fn visit_binding_rhs_super(
        &mut self,
        handle: BindingRhsHandle,
        view_param: BindingRhsView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        match view_param {
            BindingRhsView::ValueBinding(item) => {
                self.visit_value_binding_handle(item, tree)?;
            }
            BindingRhsView::SectionBinding(item) => {
                self.visit_section_binding_handle(item, tree)?;
            }
            BindingRhsView::TextBinding(item) => {
                self.visit_text_binding_handle(item, tree)?;
            }
        }
        Ok(())
    }
    fn visit_boolean_super(
        &mut self,
        handle: BooleanHandle,
        view_param: BooleanView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        match view_param {
            BooleanView::True(item) => {
                self.visit_true_handle(item, tree)?;
            }
            BooleanView::False(item) => {
                self.visit_false_handle(item, tree)?;
            }
        }
        Ok(())
    }
    fn visit_code_block_super(
        &mut self,
        handle: CodeBlockHandle,
        view_param: CodeBlockView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        match view_param {
            CodeBlockView::CodeBlock3(item) => {
                self.visit_code_block_3_handle(item, tree)?;
            }
            CodeBlockView::CodeBlock4(item) => {
                self.visit_code_block_4_handle(item, tree)?;
            }
            CodeBlockView::CodeBlock5(item) => {
                self.visit_code_block_5_handle(item, tree)?;
            }
            CodeBlockView::CodeBlock6(item) => {
                self.visit_code_block_6_handle(item, tree)?;
            }
        }
        Ok(())
    }
    fn visit_code_block_3_super(
        &mut self,
        handle: CodeBlock3Handle,
        view_param: CodeBlock3View,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let CodeBlock3View { code_block_start_3, code_block_3_list, code_block_end_3 } = view_param;
        self.visit_code_block_start_3_handle(code_block_start_3, tree)?;
        self.visit_code_block_3_list_handle(code_block_3_list, tree)?;
        self.visit_code_block_end_3_handle(code_block_end_3, tree)?;
        Ok(())
    }
    fn visit_code_block_3_list_super(
        &mut self,
        handle: CodeBlock3ListHandle,
        view_param: CodeBlock3ListView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let CodeBlock3ListView { code_block_3_list_group, code_block_3_list } = view_param;
        self.visit_code_block_3_list_group_handle(code_block_3_list_group, tree)?;
        self.visit_code_block_3_list_handle(code_block_3_list, tree)?;
        Ok(())
    }
    fn visit_code_block_3_list_group_super(
        &mut self,
        handle: CodeBlock3ListGroupHandle,
        view_param: CodeBlock3ListGroupView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        match view_param {
            CodeBlock3ListGroupView::NoBacktick(item) => {
                self.visit_no_backtick_handle(item, tree)?;
            }
            CodeBlock3ListGroupView::Backtick2(item) => {
                self.visit_backtick_2_handle(item, tree)?;
            }
        }
        Ok(())
    }
    fn visit_code_block_4_super(
        &mut self,
        handle: CodeBlock4Handle,
        view_param: CodeBlock4View,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let CodeBlock4View { code_block_start_4, code_block_4_list, code_block_end_4 } = view_param;
        self.visit_code_block_start_4_handle(code_block_start_4, tree)?;
        self.visit_code_block_4_list_handle(code_block_4_list, tree)?;
        self.visit_code_block_end_4_handle(code_block_end_4, tree)?;
        Ok(())
    }
    fn visit_code_block_4_list_super(
        &mut self,
        handle: CodeBlock4ListHandle,
        view_param: CodeBlock4ListView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let CodeBlock4ListView { code_block_4_list_group, code_block_4_list } = view_param;
        self.visit_code_block_4_list_group_handle(code_block_4_list_group, tree)?;
        self.visit_code_block_4_list_handle(code_block_4_list, tree)?;
        Ok(())
    }
    fn visit_code_block_4_list_group_super(
        &mut self,
        handle: CodeBlock4ListGroupHandle,
        view_param: CodeBlock4ListGroupView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        match view_param {
            CodeBlock4ListGroupView::NoBacktick(item) => {
                self.visit_no_backtick_handle(item, tree)?;
            }
            CodeBlock4ListGroupView::Backtick3(item) => {
                self.visit_backtick_3_handle(item, tree)?;
            }
        }
        Ok(())
    }
    fn visit_code_block_5_super(
        &mut self,
        handle: CodeBlock5Handle,
        view_param: CodeBlock5View,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let CodeBlock5View { code_block_start_5, code_block_5_list, code_block_end_5 } = view_param;
        self.visit_code_block_start_5_handle(code_block_start_5, tree)?;
        self.visit_code_block_5_list_handle(code_block_5_list, tree)?;
        self.visit_code_block_end_5_handle(code_block_end_5, tree)?;
        Ok(())
    }
    fn visit_code_block_5_list_super(
        &mut self,
        handle: CodeBlock5ListHandle,
        view_param: CodeBlock5ListView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let CodeBlock5ListView { code_block_5_list_group, code_block_5_list } = view_param;
        self.visit_code_block_5_list_group_handle(code_block_5_list_group, tree)?;
        self.visit_code_block_5_list_handle(code_block_5_list, tree)?;
        Ok(())
    }
    fn visit_code_block_5_list_group_super(
        &mut self,
        handle: CodeBlock5ListGroupHandle,
        view_param: CodeBlock5ListGroupView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        match view_param {
            CodeBlock5ListGroupView::NoBacktick(item) => {
                self.visit_no_backtick_handle(item, tree)?;
            }
            CodeBlock5ListGroupView::Backtick4(item) => {
                self.visit_backtick_4_handle(item, tree)?;
            }
        }
        Ok(())
    }
    fn visit_code_block_6_super(
        &mut self,
        handle: CodeBlock6Handle,
        view_param: CodeBlock6View,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let CodeBlock6View { code_block_start_6, code_block_6_list, code_block_end_6 } = view_param;
        self.visit_code_block_start_6_handle(code_block_start_6, tree)?;
        self.visit_code_block_6_list_handle(code_block_6_list, tree)?;
        self.visit_code_block_end_6_handle(code_block_end_6, tree)?;
        Ok(())
    }
    fn visit_code_block_6_list_super(
        &mut self,
        handle: CodeBlock6ListHandle,
        view_param: CodeBlock6ListView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let CodeBlock6ListView { code_block_6_list_group, code_block_6_list } = view_param;
        self.visit_code_block_6_list_group_handle(code_block_6_list_group, tree)?;
        self.visit_code_block_6_list_handle(code_block_6_list, tree)?;
        Ok(())
    }
    fn visit_code_block_6_list_group_super(
        &mut self,
        handle: CodeBlock6ListGroupHandle,
        view_param: CodeBlock6ListGroupView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        match view_param {
            CodeBlock6ListGroupView::NoBacktick(item) => {
                self.visit_no_backtick_handle(item, tree)?;
            }
            CodeBlock6ListGroupView::Backtick5(item) => {
                self.visit_backtick_5_handle(item, tree)?;
            }
        }
        Ok(())
    }
    fn visit_code_block_end_3_super(
        &mut self,
        handle: CodeBlockEnd3Handle,
        view_param: CodeBlockEnd3View,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let CodeBlockEnd3View { code_block_end_3 } = view_param;
        let data = match code_block_end_3.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        code_block_end_3.0,
                        NodeKind::Terminal(code_block_end_3.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_code_block_end_3_terminal(code_block_end_3, data, tree)?;
        Ok(())
    }
    fn visit_code_block_end_4_super(
        &mut self,
        handle: CodeBlockEnd4Handle,
        view_param: CodeBlockEnd4View,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let CodeBlockEnd4View { code_block_end_4 } = view_param;
        let data = match code_block_end_4.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        code_block_end_4.0,
                        NodeKind::Terminal(code_block_end_4.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_code_block_end_4_terminal(code_block_end_4, data, tree)?;
        Ok(())
    }
    fn visit_code_block_end_5_super(
        &mut self,
        handle: CodeBlockEnd5Handle,
        view_param: CodeBlockEnd5View,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let CodeBlockEnd5View { code_block_end_5 } = view_param;
        let data = match code_block_end_5.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        code_block_end_5.0,
                        NodeKind::Terminal(code_block_end_5.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_code_block_end_5_terminal(code_block_end_5, data, tree)?;
        Ok(())
    }
    fn visit_code_block_end_6_super(
        &mut self,
        handle: CodeBlockEnd6Handle,
        view_param: CodeBlockEnd6View,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let CodeBlockEnd6View { code_block_end_6 } = view_param;
        let data = match code_block_end_6.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        code_block_end_6.0,
                        NodeKind::Terminal(code_block_end_6.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_code_block_end_6_terminal(code_block_end_6, data, tree)?;
        Ok(())
    }
    fn visit_code_block_start_3_super(
        &mut self,
        handle: CodeBlockStart3Handle,
        view_param: CodeBlockStart3View,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let CodeBlockStart3View { code_block_start_3 } = view_param;
        let data = match code_block_start_3.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        code_block_start_3.0,
                        NodeKind::Terminal(code_block_start_3.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_code_block_start_3_terminal(code_block_start_3, data, tree)?;
        Ok(())
    }
    fn visit_code_block_start_4_super(
        &mut self,
        handle: CodeBlockStart4Handle,
        view_param: CodeBlockStart4View,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let CodeBlockStart4View { code_block_start_4 } = view_param;
        let data = match code_block_start_4.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        code_block_start_4.0,
                        NodeKind::Terminal(code_block_start_4.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_code_block_start_4_terminal(code_block_start_4, data, tree)?;
        Ok(())
    }
    fn visit_code_block_start_5_super(
        &mut self,
        handle: CodeBlockStart5Handle,
        view_param: CodeBlockStart5View,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let CodeBlockStart5View { code_block_start_5 } = view_param;
        let data = match code_block_start_5.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        code_block_start_5.0,
                        NodeKind::Terminal(code_block_start_5.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_code_block_start_5_terminal(code_block_start_5, data, tree)?;
        Ok(())
    }
    fn visit_code_block_start_6_super(
        &mut self,
        handle: CodeBlockStart6Handle,
        view_param: CodeBlockStart6View,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let CodeBlockStart6View { code_block_start_6 } = view_param;
        let data = match code_block_start_6.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        code_block_start_6.0,
                        NodeKind::Terminal(code_block_start_6.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_code_block_start_6_terminal(code_block_start_6, data, tree)?;
        Ok(())
    }
    fn visit_comma_super(
        &mut self,
        handle: CommaHandle,
        view_param: CommaView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let CommaView { comma } = view_param;
        let data = match comma.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        comma.0,
                        NodeKind::Terminal(comma.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_comma_terminal(comma, data, tree)?;
        Ok(())
    }
    fn visit_continue_super(
        &mut self,
        handle: ContinueHandle,
        view_param: ContinueView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let ContinueView { esc } = view_param;
        let data = match esc.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        esc.0,
                        NodeKind::Terminal(esc.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_esc_terminal(esc, data, tree)?;
        Ok(())
    }
    fn visit_dot_super(
        &mut self,
        handle: DotHandle,
        view_param: DotView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let DotView { dot } = view_param;
        let data = match dot.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        dot.0,
                        NodeKind::Terminal(dot.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_dot_terminal(dot, data, tree)?;
        Ok(())
    }
    fn visit_end_super(
        &mut self,
        handle: EndHandle,
        view_param: EndView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let EndView { r_brace } = view_param;
        let data = match r_brace.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        r_brace.0,
                        NodeKind::Terminal(r_brace.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_r_brace_terminal(r_brace, data, tree)?;
        Ok(())
    }
    fn visit_eure_super(
        &mut self,
        handle: EureHandle,
        view_param: EureView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let EureView { eure_opt, eure_bindings, eure_sections } = view_param;
        self.visit_eure_opt_handle(eure_opt, tree)?;
        self.visit_eure_bindings_handle(eure_bindings, tree)?;
        self.visit_eure_sections_handle(eure_sections, tree)?;
        Ok(())
    }
    fn visit_eure_bindings_super(
        &mut self,
        handle: EureBindingsHandle,
        view_param: EureBindingsView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let EureBindingsView { binding, eure_bindings } = view_param;
        self.visit_binding_handle(binding, tree)?;
        self.visit_eure_bindings_handle(eure_bindings, tree)?;
        Ok(())
    }
    fn visit_eure_sections_super(
        &mut self,
        handle: EureSectionsHandle,
        view_param: EureSectionsView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let EureSectionsView { section, eure_sections } = view_param;
        self.visit_section_handle(section, tree)?;
        self.visit_eure_sections_handle(eure_sections, tree)?;
        Ok(())
    }
    fn visit_eure_opt_super(
        &mut self,
        handle: EureOptHandle,
        view_param: ValueBindingHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        self.visit_value_binding_handle(view_param, tree)?;
        Ok(())
    }
    fn visit_ext_super(
        &mut self,
        handle: ExtHandle,
        view_param: ExtView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let ExtView { dollar } = view_param;
        let data = match dollar.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        dollar.0,
                        NodeKind::Terminal(dollar.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_dollar_terminal(dollar, data, tree)?;
        Ok(())
    }
    fn visit_extension_name_space_super(
        &mut self,
        handle: ExtensionNameSpaceHandle,
        view_param: ExtensionNameSpaceView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let ExtensionNameSpaceView { ext, key_ident } = view_param;
        self.visit_ext_handle(ext, tree)?;
        self.visit_key_ident_handle(key_ident, tree)?;
        Ok(())
    }
    fn visit_false_super(
        &mut self,
        handle: FalseHandle,
        view_param: FalseView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let FalseView { r#false } = view_param;
        let data = match r#false.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        r#false.0,
                        NodeKind::Terminal(r#false.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_false_terminal(r#false, data, tree)?;
        Ok(())
    }
    fn visit_float_super(
        &mut self,
        handle: FloatHandle,
        view_param: FloatView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let FloatView { float } = view_param;
        let data = match float.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        float.0,
                        NodeKind::Terminal(float.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_float_terminal(float, data, tree)?;
        Ok(())
    }
    fn visit_grammar_newline_super(
        &mut self,
        handle: GrammarNewlineHandle,
        view_param: GrammarNewlineView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let GrammarNewlineView { grammar_newline } = view_param;
        let data = match grammar_newline.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        grammar_newline.0,
                        NodeKind::Terminal(grammar_newline.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_grammar_newline_terminal(grammar_newline, data, tree)?;
        Ok(())
    }
    fn visit_hole_super(
        &mut self,
        handle: HoleHandle,
        view_param: HoleView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let HoleView { hole } = view_param;
        let data = match hole.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        hole.0,
                        NodeKind::Terminal(hole.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_hole_terminal(hole, data, tree)?;
        Ok(())
    }
    fn visit_ident_super(
        &mut self,
        handle: IdentHandle,
        view_param: IdentView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let IdentView { ident } = view_param;
        let data = match ident.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        ident.0,
                        NodeKind::Terminal(ident.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_ident_terminal(ident, data, tree)?;
        Ok(())
    }
    fn visit_inline_code_super(
        &mut self,
        handle: InlineCodeHandle,
        view_param: InlineCodeView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        match view_param {
            InlineCodeView::InlineCode2(item) => {
                self.visit_inline_code_2_handle(item, tree)?;
            }
            InlineCodeView::InlineCode1(item) => {
                self.visit_inline_code_1_handle(item, tree)?;
            }
        }
        Ok(())
    }
    fn visit_inline_code_1_super(
        &mut self,
        handle: InlineCode1Handle,
        view_param: InlineCode1View,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let InlineCode1View { inline_code_1 } = view_param;
        let data = match inline_code_1.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        inline_code_1.0,
                        NodeKind::Terminal(inline_code_1.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_inline_code_1_terminal(inline_code_1, data, tree)?;
        Ok(())
    }
    fn visit_inline_code_2_super(
        &mut self,
        handle: InlineCode2Handle,
        view_param: InlineCode2View,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let InlineCode2View {
            inline_code_start_2,
            inline_code_2_list,
            inline_code_end_2,
        } = view_param;
        self.visit_inline_code_start_2_handle(inline_code_start_2, tree)?;
        self.visit_inline_code_2_list_handle(inline_code_2_list, tree)?;
        self.visit_inline_code_end_2_handle(inline_code_end_2, tree)?;
        Ok(())
    }
    fn visit_inline_code_2_list_super(
        &mut self,
        handle: InlineCode2ListHandle,
        view_param: InlineCode2ListView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let InlineCode2ListView { inline_code_2_list_group, inline_code_2_list } = view_param;
        self.visit_inline_code_2_list_group_handle(inline_code_2_list_group, tree)?;
        self.visit_inline_code_2_list_handle(inline_code_2_list, tree)?;
        Ok(())
    }
    fn visit_inline_code_2_list_group_super(
        &mut self,
        handle: InlineCode2ListGroupHandle,
        view_param: InlineCode2ListGroupView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        match view_param {
            InlineCode2ListGroupView::NoBacktickInline(item) => {
                self.visit_no_backtick_inline_handle(item, tree)?;
            }
            InlineCode2ListGroupView::Backtick1(item) => {
                self.visit_backtick_1_handle(item, tree)?;
            }
        }
        Ok(())
    }
    fn visit_inline_code_end_2_super(
        &mut self,
        handle: InlineCodeEnd2Handle,
        view_param: InlineCodeEnd2View,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let InlineCodeEnd2View { inline_code_end_2 } = view_param;
        let data = match inline_code_end_2.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        inline_code_end_2.0,
                        NodeKind::Terminal(inline_code_end_2.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_inline_code_end_2_terminal(inline_code_end_2, data, tree)?;
        Ok(())
    }
    fn visit_inline_code_start_2_super(
        &mut self,
        handle: InlineCodeStart2Handle,
        view_param: InlineCodeStart2View,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let InlineCodeStart2View { inline_code_start_2 } = view_param;
        let data = match inline_code_start_2.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        inline_code_start_2.0,
                        NodeKind::Terminal(inline_code_start_2.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_inline_code_start_2_terminal(inline_code_start_2, data, tree)?;
        Ok(())
    }
    fn visit_integer_super(
        &mut self,
        handle: IntegerHandle,
        view_param: IntegerView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let IntegerView { integer } = view_param;
        let data = match integer.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        integer.0,
                        NodeKind::Terminal(integer.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_integer_terminal(integer, data, tree)?;
        Ok(())
    }
    fn visit_key_super(
        &mut self,
        handle: KeyHandle,
        view_param: KeyView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let KeyView { key_base, key_opt } = view_param;
        self.visit_key_base_handle(key_base, tree)?;
        self.visit_key_opt_handle(key_opt, tree)?;
        Ok(())
    }
    fn visit_key_base_super(
        &mut self,
        handle: KeyBaseHandle,
        view_param: KeyBaseView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        match view_param {
            KeyBaseView::KeyIdent(item) => {
                self.visit_key_ident_handle(item, tree)?;
            }
            KeyBaseView::ExtensionNameSpace(item) => {
                self.visit_extension_name_space_handle(item, tree)?;
            }
            KeyBaseView::Str(item) => {
                self.visit_str_handle(item, tree)?;
            }
            KeyBaseView::Integer(item) => {
                self.visit_integer_handle(item, tree)?;
            }
            KeyBaseView::KeyTuple(item) => {
                self.visit_key_tuple_handle(item, tree)?;
            }
            KeyBaseView::TupleIndex(item) => {
                self.visit_tuple_index_handle(item, tree)?;
            }
        }
        Ok(())
    }
    fn visit_key_ident_super(
        &mut self,
        handle: KeyIdentHandle,
        view_param: KeyIdentView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        match view_param {
            KeyIdentView::Ident(item) => {
                self.visit_ident_handle(item, tree)?;
            }
            KeyIdentView::True(item) => {
                self.visit_true_handle(item, tree)?;
            }
            KeyIdentView::False(item) => {
                self.visit_false_handle(item, tree)?;
            }
            KeyIdentView::Null(item) => {
                self.visit_null_handle(item, tree)?;
            }
        }
        Ok(())
    }
    fn visit_key_opt_super(
        &mut self,
        handle: KeyOptHandle,
        view_param: ArrayMarkerHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        self.visit_array_marker_handle(view_param, tree)?;
        Ok(())
    }
    fn visit_key_tuple_super(
        &mut self,
        handle: KeyTupleHandle,
        view_param: KeyTupleView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let KeyTupleView { l_paren, key_tuple_opt, r_paren } = view_param;
        self.visit_l_paren_handle(l_paren, tree)?;
        self.visit_key_tuple_opt_handle(key_tuple_opt, tree)?;
        self.visit_r_paren_handle(r_paren, tree)?;
        Ok(())
    }
    fn visit_key_tuple_elements_super(
        &mut self,
        handle: KeyTupleElementsHandle,
        view_param: KeyTupleElementsView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let KeyTupleElementsView { key_value, key_tuple_elements_opt } = view_param;
        self.visit_key_value_handle(key_value, tree)?;
        self.visit_key_tuple_elements_opt_handle(key_tuple_elements_opt, tree)?;
        Ok(())
    }
    fn visit_key_tuple_elements_opt_super(
        &mut self,
        handle: KeyTupleElementsOptHandle,
        view_param: KeyTupleElementsTailHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        self.visit_key_tuple_elements_tail_handle(view_param, tree)?;
        Ok(())
    }
    fn visit_key_tuple_elements_tail_super(
        &mut self,
        handle: KeyTupleElementsTailHandle,
        view_param: KeyTupleElementsTailView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let KeyTupleElementsTailView { comma, key_tuple_elements_tail_opt } = view_param;
        self.visit_comma_handle(comma, tree)?;
        self.visit_key_tuple_elements_tail_opt_handle(
            key_tuple_elements_tail_opt,
            tree,
        )?;
        Ok(())
    }
    fn visit_key_tuple_elements_tail_opt_super(
        &mut self,
        handle: KeyTupleElementsTailOptHandle,
        view_param: KeyTupleElementsHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        self.visit_key_tuple_elements_handle(view_param, tree)?;
        Ok(())
    }
    fn visit_key_tuple_opt_super(
        &mut self,
        handle: KeyTupleOptHandle,
        view_param: KeyTupleElementsHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        self.visit_key_tuple_elements_handle(view_param, tree)?;
        Ok(())
    }
    fn visit_key_value_super(
        &mut self,
        handle: KeyValueHandle,
        view_param: KeyValueView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        match view_param {
            KeyValueView::Integer(item) => {
                self.visit_integer_handle(item, tree)?;
            }
            KeyValueView::Boolean(item) => {
                self.visit_boolean_handle(item, tree)?;
            }
            KeyValueView::Str(item) => {
                self.visit_str_handle(item, tree)?;
            }
            KeyValueView::KeyTuple(item) => {
                self.visit_key_tuple_handle(item, tree)?;
            }
        }
        Ok(())
    }
    fn visit_keys_super(
        &mut self,
        handle: KeysHandle,
        view_param: KeysView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let KeysView { key, keys_list } = view_param;
        self.visit_key_handle(key, tree)?;
        self.visit_keys_list_handle(keys_list, tree)?;
        Ok(())
    }
    fn visit_keys_list_super(
        &mut self,
        handle: KeysListHandle,
        view_param: KeysListView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let KeysListView { dot, key, keys_list } = view_param;
        self.visit_dot_handle(dot, tree)?;
        self.visit_key_handle(key, tree)?;
        self.visit_keys_list_handle(keys_list, tree)?;
        Ok(())
    }
    fn visit_l_paren_super(
        &mut self,
        handle: LParenHandle,
        view_param: LParenView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let LParenView { l_paren } = view_param;
        let data = match l_paren.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        l_paren.0,
                        NodeKind::Terminal(l_paren.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_l_paren_terminal(l_paren, data, tree)?;
        Ok(())
    }
    fn visit_no_backtick_super(
        &mut self,
        handle: NoBacktickHandle,
        view_param: NoBacktickView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let NoBacktickView { no_backtick } = view_param;
        let data = match no_backtick.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        no_backtick.0,
                        NodeKind::Terminal(no_backtick.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_no_backtick_terminal(no_backtick, data, tree)?;
        Ok(())
    }
    fn visit_no_backtick_inline_super(
        &mut self,
        handle: NoBacktickInlineHandle,
        view_param: NoBacktickInlineView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let NoBacktickInlineView { no_backtick_inline } = view_param;
        let data = match no_backtick_inline.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        no_backtick_inline.0,
                        NodeKind::Terminal(no_backtick_inline.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_no_backtick_inline_terminal(no_backtick_inline, data, tree)?;
        Ok(())
    }
    fn visit_null_super(
        &mut self,
        handle: NullHandle,
        view_param: NullView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let NullView { null } = view_param;
        let data = match null.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        null.0,
                        NodeKind::Terminal(null.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_null_terminal(null, data, tree)?;
        Ok(())
    }
    fn visit_object_super(
        &mut self,
        handle: ObjectHandle,
        view_param: ObjectView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let ObjectView { begin, object_list, end } = view_param;
        self.visit_begin_handle(begin, tree)?;
        self.visit_object_list_handle(object_list, tree)?;
        self.visit_end_handle(end, tree)?;
        Ok(())
    }
    fn visit_object_list_super(
        &mut self,
        handle: ObjectListHandle,
        view_param: ObjectListView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let ObjectListView { key, bind, value, object_opt, object_list } = view_param;
        self.visit_key_handle(key, tree)?;
        self.visit_bind_handle(bind, tree)?;
        self.visit_value_handle(value, tree)?;
        self.visit_object_opt_handle(object_opt, tree)?;
        self.visit_object_list_handle(object_list, tree)?;
        Ok(())
    }
    fn visit_object_opt_super(
        &mut self,
        handle: ObjectOptHandle,
        view_param: CommaHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        self.visit_comma_handle(view_param, tree)?;
        Ok(())
    }
    fn visit_path_super(
        &mut self,
        handle: PathHandle,
        view_param: PathView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let PathView { dot, keys } = view_param;
        self.visit_dot_handle(dot, tree)?;
        self.visit_keys_handle(keys, tree)?;
        Ok(())
    }
    fn visit_r_paren_super(
        &mut self,
        handle: RParenHandle,
        view_param: RParenView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let RParenView { r_paren } = view_param;
        let data = match r_paren.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        r_paren.0,
                        NodeKind::Terminal(r_paren.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_r_paren_terminal(r_paren, data, tree)?;
        Ok(())
    }
    fn visit_section_super(
        &mut self,
        handle: SectionHandle,
        view_param: SectionView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let SectionView { at, keys, section_body } = view_param;
        self.visit_at_handle(at, tree)?;
        self.visit_keys_handle(keys, tree)?;
        self.visit_section_body_handle(section_body, tree)?;
        Ok(())
    }
    fn visit_section_binding_super(
        &mut self,
        handle: SectionBindingHandle,
        view_param: SectionBindingView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let SectionBindingView { begin, eure, end } = view_param;
        self.visit_begin_handle(begin, tree)?;
        self.visit_eure_handle(eure, tree)?;
        self.visit_end_handle(end, tree)?;
        Ok(())
    }
    fn visit_section_body_super(
        &mut self,
        handle: SectionBodyHandle,
        view_param: SectionBodyView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        match view_param {
            SectionBodyView::SectionBodyOpt(item) => {
                self.visit_section_body_opt_handle(item, tree)?;
            }
            SectionBodyView::Begin(item) => {
                self.visit_begin_handle(item, tree)?;
            }
        }
        Ok(())
    }
    fn visit_section_body_list_super(
        &mut self,
        handle: SectionBodyListHandle,
        view_param: SectionBodyListView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let SectionBodyListView { binding, section_body_list } = view_param;
        self.visit_binding_handle(binding, tree)?;
        self.visit_section_body_list_handle(section_body_list, tree)?;
        Ok(())
    }
    fn visit_section_body_opt_super(
        &mut self,
        handle: SectionBodyOptHandle,
        view_param: ValueBindingHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        self.visit_value_binding_handle(view_param, tree)?;
        Ok(())
    }
    fn visit_str_super(
        &mut self,
        handle: StrHandle,
        view_param: StrView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let StrView { str } = view_param;
        let data = match str.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        str.0,
                        NodeKind::Terminal(str.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_str_terminal(str, data, tree)?;
        Ok(())
    }
    fn visit_strings_super(
        &mut self,
        handle: StringsHandle,
        view_param: StringsView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let StringsView { str, strings_list } = view_param;
        self.visit_str_handle(str, tree)?;
        self.visit_strings_list_handle(strings_list, tree)?;
        Ok(())
    }
    fn visit_strings_list_super(
        &mut self,
        handle: StringsListHandle,
        view_param: StringsListView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let StringsListView { r#continue, str, strings_list } = view_param;
        self.visit_continue_handle(r#continue, tree)?;
        self.visit_str_handle(str, tree)?;
        self.visit_strings_list_handle(strings_list, tree)?;
        Ok(())
    }
    fn visit_text_super(
        &mut self,
        handle: TextHandle,
        view_param: TextView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let TextView { text } = view_param;
        let data = match text.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        text.0,
                        NodeKind::Terminal(text.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_text_terminal(text, data, tree)?;
        Ok(())
    }
    fn visit_text_binding_super(
        &mut self,
        handle: TextBindingHandle,
        view_param: TextBindingView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let TextBindingView { text_start, text_binding_opt, text, text_binding_opt_0 } = view_param;
        self.visit_text_start_handle(text_start, tree)?;
        self.visit_text_binding_opt_handle(text_binding_opt, tree)?;
        self.visit_text_handle(text, tree)?;
        self.visit_text_binding_opt_0_handle(text_binding_opt_0, tree)?;
        Ok(())
    }
    fn visit_text_binding_opt_super(
        &mut self,
        handle: TextBindingOptHandle,
        view_param: WsHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        self.visit_ws_handle(view_param, tree)?;
        Ok(())
    }
    fn visit_text_binding_opt_0_super(
        &mut self,
        handle: TextBindingOpt0Handle,
        view_param: GrammarNewlineHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        self.visit_grammar_newline_handle(view_param, tree)?;
        Ok(())
    }
    fn visit_text_start_super(
        &mut self,
        handle: TextStartHandle,
        view_param: TextStartView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let TextStartView { text_start } = view_param;
        let data = match text_start.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        text_start.0,
                        NodeKind::Terminal(text_start.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_text_start_terminal(text_start, data, tree)?;
        Ok(())
    }
    fn visit_true_super(
        &mut self,
        handle: TrueHandle,
        view_param: TrueView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let TrueView { r#true } = view_param;
        let data = match r#true.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        r#true.0,
                        NodeKind::Terminal(r#true.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_true_terminal(r#true, data, tree)?;
        Ok(())
    }
    fn visit_tuple_super(
        &mut self,
        handle: TupleHandle,
        view_param: TupleView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let TupleView { l_paren, tuple_opt, r_paren } = view_param;
        self.visit_l_paren_handle(l_paren, tree)?;
        self.visit_tuple_opt_handle(tuple_opt, tree)?;
        self.visit_r_paren_handle(r_paren, tree)?;
        Ok(())
    }
    fn visit_tuple_elements_super(
        &mut self,
        handle: TupleElementsHandle,
        view_param: TupleElementsView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let TupleElementsView { value, tuple_elements_opt } = view_param;
        self.visit_value_handle(value, tree)?;
        self.visit_tuple_elements_opt_handle(tuple_elements_opt, tree)?;
        Ok(())
    }
    fn visit_tuple_elements_opt_super(
        &mut self,
        handle: TupleElementsOptHandle,
        view_param: TupleElementsTailHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        self.visit_tuple_elements_tail_handle(view_param, tree)?;
        Ok(())
    }
    fn visit_tuple_elements_tail_super(
        &mut self,
        handle: TupleElementsTailHandle,
        view_param: TupleElementsTailView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let TupleElementsTailView { comma, tuple_elements_tail_opt } = view_param;
        self.visit_comma_handle(comma, tree)?;
        self.visit_tuple_elements_tail_opt_handle(tuple_elements_tail_opt, tree)?;
        Ok(())
    }
    fn visit_tuple_elements_tail_opt_super(
        &mut self,
        handle: TupleElementsTailOptHandle,
        view_param: TupleElementsHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        self.visit_tuple_elements_handle(view_param, tree)?;
        Ok(())
    }
    fn visit_tuple_index_super(
        &mut self,
        handle: TupleIndexHandle,
        view_param: TupleIndexView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let TupleIndexView { hash, integer } = view_param;
        let data = match hash.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        hash.0,
                        NodeKind::Terminal(hash.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_hash_terminal(hash, data, tree)?;
        self.visit_integer_handle(integer, tree)?;
        Ok(())
    }
    fn visit_tuple_opt_super(
        &mut self,
        handle: TupleOptHandle,
        view_param: TupleElementsHandle,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        self.visit_tuple_elements_handle(view_param, tree)?;
        Ok(())
    }
    fn visit_value_super(
        &mut self,
        handle: ValueHandle,
        view_param: ValueView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        match view_param {
            ValueView::Object(item) => {
                self.visit_object_handle(item, tree)?;
            }
            ValueView::Array(item) => {
                self.visit_array_handle(item, tree)?;
            }
            ValueView::Tuple(item) => {
                self.visit_tuple_handle(item, tree)?;
            }
            ValueView::Float(item) => {
                self.visit_float_handle(item, tree)?;
            }
            ValueView::Integer(item) => {
                self.visit_integer_handle(item, tree)?;
            }
            ValueView::Boolean(item) => {
                self.visit_boolean_handle(item, tree)?;
            }
            ValueView::Null(item) => {
                self.visit_null_handle(item, tree)?;
            }
            ValueView::Strings(item) => {
                self.visit_strings_handle(item, tree)?;
            }
            ValueView::Hole(item) => {
                self.visit_hole_handle(item, tree)?;
            }
            ValueView::CodeBlock(item) => {
                self.visit_code_block_handle(item, tree)?;
            }
            ValueView::InlineCode(item) => {
                self.visit_inline_code_handle(item, tree)?;
            }
            ValueView::Path(item) => {
                self.visit_path_handle(item, tree)?;
            }
        }
        Ok(())
    }
    fn visit_value_binding_super(
        &mut self,
        handle: ValueBindingHandle,
        view_param: ValueBindingView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let ValueBindingView { bind, value } = view_param;
        self.visit_bind_handle(bind, tree)?;
        self.visit_value_handle(value, tree)?;
        Ok(())
    }
    fn visit_ws_super(
        &mut self,
        handle: WsHandle,
        view_param: WsView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let WsView { ws } = view_param;
        let data = match ws.get_data(tree) {
            Ok(data) => data,
            Err(error) => {
                return self
                    .then_construct_error(
                        None,
                        ws.0,
                        NodeKind::Terminal(ws.kind()),
                        error,
                        tree,
                    );
            }
        };
        self.visit_ws_terminal(ws, data, tree)?;
        Ok(())
    }
    fn visit_root_super(
        &mut self,
        handle: RootHandle,
        view_param: RootView,
        tree: &F,
    ) -> Result<(), V::Error> {
        let _handle = handle;
        let RootView { eure } = view_param;
        self.visit_eure_handle(eure, tree)?;
        Ok(())
    }
    fn visit_new_line_terminal_super(
        &mut self,
        terminal: NewLine,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_whitespace_terminal_super(
        &mut self,
        terminal: Whitespace,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_line_comment_terminal_super(
        &mut self,
        terminal: LineComment,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_block_comment_terminal_super(
        &mut self,
        terminal: BlockComment,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_hash_terminal_super(
        &mut self,
        terminal: Hash,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_float_terminal_super(
        &mut self,
        terminal: Float,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_integer_terminal_super(
        &mut self,
        terminal: Integer,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_true_terminal_super(
        &mut self,
        terminal: True,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_false_terminal_super(
        &mut self,
        terminal: False,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_null_terminal_super(
        &mut self,
        terminal: Null,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_hole_terminal_super(
        &mut self,
        terminal: Hole,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_str_terminal_super(
        &mut self,
        terminal: Str,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_text_terminal_super(
        &mut self,
        terminal: Text,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_inline_code_1_terminal_super(
        &mut self,
        terminal: InlineCode1,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_inline_code_start_2_terminal_super(
        &mut self,
        terminal: InlineCodeStart2,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_code_block_start_3_terminal_super(
        &mut self,
        terminal: CodeBlockStart3,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_code_block_start_4_terminal_super(
        &mut self,
        terminal: CodeBlockStart4,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_code_block_start_5_terminal_super(
        &mut self,
        terminal: CodeBlockStart5,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_code_block_start_6_terminal_super(
        &mut self,
        terminal: CodeBlockStart6,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_code_block_end_3_terminal_super(
        &mut self,
        terminal: CodeBlockEnd3,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_backtick_2_terminal_super(
        &mut self,
        terminal: Backtick2,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_code_block_end_4_terminal_super(
        &mut self,
        terminal: CodeBlockEnd4,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_backtick_3_terminal_super(
        &mut self,
        terminal: Backtick3,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_code_block_end_5_terminal_super(
        &mut self,
        terminal: CodeBlockEnd5,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_backtick_4_terminal_super(
        &mut self,
        terminal: Backtick4,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_code_block_end_6_terminal_super(
        &mut self,
        terminal: CodeBlockEnd6,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_backtick_5_terminal_super(
        &mut self,
        terminal: Backtick5,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_inline_code_end_2_terminal_super(
        &mut self,
        terminal: InlineCodeEnd2,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_backtick_1_terminal_super(
        &mut self,
        terminal: Backtick1,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_no_backtick_terminal_super(
        &mut self,
        terminal: NoBacktick,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_no_backtick_inline_terminal_super(
        &mut self,
        terminal: NoBacktickInline,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_grammar_newline_terminal_super(
        &mut self,
        terminal: GrammarNewline,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_ws_terminal_super(
        &mut self,
        terminal: Ws,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_at_terminal_super(
        &mut self,
        terminal: At,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_dollar_terminal_super(
        &mut self,
        terminal: Dollar,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_dot_terminal_super(
        &mut self,
        terminal: Dot,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_l_brace_terminal_super(
        &mut self,
        terminal: LBrace,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_r_brace_terminal_super(
        &mut self,
        terminal: RBrace,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_l_bracket_terminal_super(
        &mut self,
        terminal: LBracket,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_r_bracket_terminal_super(
        &mut self,
        terminal: RBracket,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_l_paren_terminal_super(
        &mut self,
        terminal: LParen,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_r_paren_terminal_super(
        &mut self,
        terminal: RParen,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_bind_terminal_super(
        &mut self,
        terminal: Bind,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_comma_terminal_super(
        &mut self,
        terminal: Comma,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_esc_terminal_super(
        &mut self,
        terminal: Esc,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_text_start_terminal_super(
        &mut self,
        terminal: TextStart,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_ident_terminal_super(
        &mut self,
        terminal: Ident,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_terminal(terminal.0, terminal.kind(), data, tree)?;
        Ok(())
    }
    fn visit_non_terminal_super(
        &mut self,
        _id: CstNodeId,
        _kind: NonTerminalKind,
        _data: NonTerminalData,
        _tree: &F,
    ) -> Result<(), V::Error> {
        Ok(())
    }
    fn visit_non_terminal_close_super(
        &mut self,
        _id: CstNodeId,
        _kind: NonTerminalKind,
        _data: NonTerminalData,
        _tree: &F,
    ) -> Result<(), V::Error> {
        Ok(())
    }
    fn visit_terminal_super(
        &mut self,
        _id: CstNodeId,
        _kind: TerminalKind,
        _data: TerminalData,
        _tree: &F,
    ) -> Result<(), V::Error> {
        Ok(())
    }
    fn recover_error(
        &mut self,
        node_data: Option<CstNode>,
        id: CstNodeId,
        kind: NodeKind,
        tree: &F,
    ) -> Result<(), V::Error> {
        let Some(node_data) = node_data else {
            return Ok(());
        };
        if node_data.node_kind() == kind {
            for child in tree.children(id) {
                if let Some(node_data) = tree.node_data(child) {
                    self.visit_any(child, node_data, tree)?;
                }
            }
        } else {
            self.visit_any(id, node_data, tree)?;
        }
        Ok(())
    }
    fn visit_any(
        &mut self,
        id: CstNodeId,
        node: CstNode,
        tree: &F,
    ) -> Result<(), V::Error> {
        match node {
            CstNode::NonTerminal { kind, .. } => {
                match kind {
                    NonTerminalKind::Array => {
                        let handle = ArrayHandle(id);
                        self.visit_array_handle(handle, tree)?;
                    }
                    NonTerminalKind::ArrayBegin => {
                        let handle = ArrayBeginHandle(id);
                        self.visit_array_begin_handle(handle, tree)?;
                    }
                    NonTerminalKind::ArrayElements => {
                        let handle = ArrayElementsHandle(id);
                        self.visit_array_elements_handle(handle, tree)?;
                    }
                    NonTerminalKind::ArrayElementsOpt => {
                        let handle = ArrayElementsOptHandle(id);
                        self.visit_array_elements_opt_handle(handle, tree)?;
                    }
                    NonTerminalKind::ArrayElementsTail => {
                        let handle = ArrayElementsTailHandle(id);
                        self.visit_array_elements_tail_handle(handle, tree)?;
                    }
                    NonTerminalKind::ArrayElementsTailOpt => {
                        let handle = ArrayElementsTailOptHandle(id);
                        self.visit_array_elements_tail_opt_handle(handle, tree)?;
                    }
                    NonTerminalKind::ArrayEnd => {
                        let handle = ArrayEndHandle(id);
                        self.visit_array_end_handle(handle, tree)?;
                    }
                    NonTerminalKind::ArrayMarker => {
                        let handle = ArrayMarkerHandle(id);
                        self.visit_array_marker_handle(handle, tree)?;
                    }
                    NonTerminalKind::ArrayMarkerOpt => {
                        let handle = ArrayMarkerOptHandle(id);
                        self.visit_array_marker_opt_handle(handle, tree)?;
                    }
                    NonTerminalKind::ArrayOpt => {
                        let handle = ArrayOptHandle(id);
                        self.visit_array_opt_handle(handle, tree)?;
                    }
                    NonTerminalKind::At => {
                        let handle = AtHandle(id);
                        self.visit_at_handle(handle, tree)?;
                    }
                    NonTerminalKind::Backtick1 => {
                        let handle = Backtick1Handle(id);
                        self.visit_backtick_1_handle(handle, tree)?;
                    }
                    NonTerminalKind::Backtick2 => {
                        let handle = Backtick2Handle(id);
                        self.visit_backtick_2_handle(handle, tree)?;
                    }
                    NonTerminalKind::Backtick3 => {
                        let handle = Backtick3Handle(id);
                        self.visit_backtick_3_handle(handle, tree)?;
                    }
                    NonTerminalKind::Backtick4 => {
                        let handle = Backtick4Handle(id);
                        self.visit_backtick_4_handle(handle, tree)?;
                    }
                    NonTerminalKind::Backtick5 => {
                        let handle = Backtick5Handle(id);
                        self.visit_backtick_5_handle(handle, tree)?;
                    }
                    NonTerminalKind::Begin => {
                        let handle = BeginHandle(id);
                        self.visit_begin_handle(handle, tree)?;
                    }
                    NonTerminalKind::Bind => {
                        let handle = BindHandle(id);
                        self.visit_bind_handle(handle, tree)?;
                    }
                    NonTerminalKind::Binding => {
                        let handle = BindingHandle(id);
                        self.visit_binding_handle(handle, tree)?;
                    }
                    NonTerminalKind::BindingRhs => {
                        let handle = BindingRhsHandle(id);
                        self.visit_binding_rhs_handle(handle, tree)?;
                    }
                    NonTerminalKind::Boolean => {
                        let handle = BooleanHandle(id);
                        self.visit_boolean_handle(handle, tree)?;
                    }
                    NonTerminalKind::CodeBlock => {
                        let handle = CodeBlockHandle(id);
                        self.visit_code_block_handle(handle, tree)?;
                    }
                    NonTerminalKind::CodeBlock3 => {
                        let handle = CodeBlock3Handle(id);
                        self.visit_code_block_3_handle(handle, tree)?;
                    }
                    NonTerminalKind::CodeBlock3List => {
                        let handle = CodeBlock3ListHandle(id);
                        self.visit_code_block_3_list_handle(handle, tree)?;
                    }
                    NonTerminalKind::CodeBlock3ListGroup => {
                        let handle = CodeBlock3ListGroupHandle(id);
                        self.visit_code_block_3_list_group_handle(handle, tree)?;
                    }
                    NonTerminalKind::CodeBlock4 => {
                        let handle = CodeBlock4Handle(id);
                        self.visit_code_block_4_handle(handle, tree)?;
                    }
                    NonTerminalKind::CodeBlock4List => {
                        let handle = CodeBlock4ListHandle(id);
                        self.visit_code_block_4_list_handle(handle, tree)?;
                    }
                    NonTerminalKind::CodeBlock4ListGroup => {
                        let handle = CodeBlock4ListGroupHandle(id);
                        self.visit_code_block_4_list_group_handle(handle, tree)?;
                    }
                    NonTerminalKind::CodeBlock5 => {
                        let handle = CodeBlock5Handle(id);
                        self.visit_code_block_5_handle(handle, tree)?;
                    }
                    NonTerminalKind::CodeBlock5List => {
                        let handle = CodeBlock5ListHandle(id);
                        self.visit_code_block_5_list_handle(handle, tree)?;
                    }
                    NonTerminalKind::CodeBlock5ListGroup => {
                        let handle = CodeBlock5ListGroupHandle(id);
                        self.visit_code_block_5_list_group_handle(handle, tree)?;
                    }
                    NonTerminalKind::CodeBlock6 => {
                        let handle = CodeBlock6Handle(id);
                        self.visit_code_block_6_handle(handle, tree)?;
                    }
                    NonTerminalKind::CodeBlock6List => {
                        let handle = CodeBlock6ListHandle(id);
                        self.visit_code_block_6_list_handle(handle, tree)?;
                    }
                    NonTerminalKind::CodeBlock6ListGroup => {
                        let handle = CodeBlock6ListGroupHandle(id);
                        self.visit_code_block_6_list_group_handle(handle, tree)?;
                    }
                    NonTerminalKind::CodeBlockEnd3 => {
                        let handle = CodeBlockEnd3Handle(id);
                        self.visit_code_block_end_3_handle(handle, tree)?;
                    }
                    NonTerminalKind::CodeBlockEnd4 => {
                        let handle = CodeBlockEnd4Handle(id);
                        self.visit_code_block_end_4_handle(handle, tree)?;
                    }
                    NonTerminalKind::CodeBlockEnd5 => {
                        let handle = CodeBlockEnd5Handle(id);
                        self.visit_code_block_end_5_handle(handle, tree)?;
                    }
                    NonTerminalKind::CodeBlockEnd6 => {
                        let handle = CodeBlockEnd6Handle(id);
                        self.visit_code_block_end_6_handle(handle, tree)?;
                    }
                    NonTerminalKind::CodeBlockStart3 => {
                        let handle = CodeBlockStart3Handle(id);
                        self.visit_code_block_start_3_handle(handle, tree)?;
                    }
                    NonTerminalKind::CodeBlockStart4 => {
                        let handle = CodeBlockStart4Handle(id);
                        self.visit_code_block_start_4_handle(handle, tree)?;
                    }
                    NonTerminalKind::CodeBlockStart5 => {
                        let handle = CodeBlockStart5Handle(id);
                        self.visit_code_block_start_5_handle(handle, tree)?;
                    }
                    NonTerminalKind::CodeBlockStart6 => {
                        let handle = CodeBlockStart6Handle(id);
                        self.visit_code_block_start_6_handle(handle, tree)?;
                    }
                    NonTerminalKind::Comma => {
                        let handle = CommaHandle(id);
                        self.visit_comma_handle(handle, tree)?;
                    }
                    NonTerminalKind::Continue => {
                        let handle = ContinueHandle(id);
                        self.visit_continue_handle(handle, tree)?;
                    }
                    NonTerminalKind::Dot => {
                        let handle = DotHandle(id);
                        self.visit_dot_handle(handle, tree)?;
                    }
                    NonTerminalKind::End => {
                        let handle = EndHandle(id);
                        self.visit_end_handle(handle, tree)?;
                    }
                    NonTerminalKind::Eure => {
                        let handle = EureHandle(id);
                        self.visit_eure_handle(handle, tree)?;
                    }
                    NonTerminalKind::EureList => {
                        let handle = EureBindingsHandle(id);
                        self.visit_eure_bindings_handle(handle, tree)?;
                    }
                    NonTerminalKind::EureList0 => {
                        let handle = EureSectionsHandle(id);
                        self.visit_eure_sections_handle(handle, tree)?;
                    }
                    NonTerminalKind::EureOpt => {
                        let handle = EureOptHandle(id);
                        self.visit_eure_opt_handle(handle, tree)?;
                    }
                    NonTerminalKind::Ext => {
                        let handle = ExtHandle(id);
                        self.visit_ext_handle(handle, tree)?;
                    }
                    NonTerminalKind::ExtensionNameSpace => {
                        let handle = ExtensionNameSpaceHandle(id);
                        self.visit_extension_name_space_handle(handle, tree)?;
                    }
                    NonTerminalKind::False => {
                        let handle = FalseHandle(id);
                        self.visit_false_handle(handle, tree)?;
                    }
                    NonTerminalKind::Float => {
                        let handle = FloatHandle(id);
                        self.visit_float_handle(handle, tree)?;
                    }
                    NonTerminalKind::GrammarNewline => {
                        let handle = GrammarNewlineHandle(id);
                        self.visit_grammar_newline_handle(handle, tree)?;
                    }
                    NonTerminalKind::Hole => {
                        let handle = HoleHandle(id);
                        self.visit_hole_handle(handle, tree)?;
                    }
                    NonTerminalKind::Ident => {
                        let handle = IdentHandle(id);
                        self.visit_ident_handle(handle, tree)?;
                    }
                    NonTerminalKind::InlineCode => {
                        let handle = InlineCodeHandle(id);
                        self.visit_inline_code_handle(handle, tree)?;
                    }
                    NonTerminalKind::InlineCode1 => {
                        let handle = InlineCode1Handle(id);
                        self.visit_inline_code_1_handle(handle, tree)?;
                    }
                    NonTerminalKind::InlineCode2 => {
                        let handle = InlineCode2Handle(id);
                        self.visit_inline_code_2_handle(handle, tree)?;
                    }
                    NonTerminalKind::InlineCode2List => {
                        let handle = InlineCode2ListHandle(id);
                        self.visit_inline_code_2_list_handle(handle, tree)?;
                    }
                    NonTerminalKind::InlineCode2ListGroup => {
                        let handle = InlineCode2ListGroupHandle(id);
                        self.visit_inline_code_2_list_group_handle(handle, tree)?;
                    }
                    NonTerminalKind::InlineCodeEnd2 => {
                        let handle = InlineCodeEnd2Handle(id);
                        self.visit_inline_code_end_2_handle(handle, tree)?;
                    }
                    NonTerminalKind::InlineCodeStart2 => {
                        let handle = InlineCodeStart2Handle(id);
                        self.visit_inline_code_start_2_handle(handle, tree)?;
                    }
                    NonTerminalKind::Integer => {
                        let handle = IntegerHandle(id);
                        self.visit_integer_handle(handle, tree)?;
                    }
                    NonTerminalKind::Key => {
                        let handle = KeyHandle(id);
                        self.visit_key_handle(handle, tree)?;
                    }
                    NonTerminalKind::KeyBase => {
                        let handle = KeyBaseHandle(id);
                        self.visit_key_base_handle(handle, tree)?;
                    }
                    NonTerminalKind::KeyIdent => {
                        let handle = KeyIdentHandle(id);
                        self.visit_key_ident_handle(handle, tree)?;
                    }
                    NonTerminalKind::KeyOpt => {
                        let handle = KeyOptHandle(id);
                        self.visit_key_opt_handle(handle, tree)?;
                    }
                    NonTerminalKind::KeyTuple => {
                        let handle = KeyTupleHandle(id);
                        self.visit_key_tuple_handle(handle, tree)?;
                    }
                    NonTerminalKind::KeyTupleElements => {
                        let handle = KeyTupleElementsHandle(id);
                        self.visit_key_tuple_elements_handle(handle, tree)?;
                    }
                    NonTerminalKind::KeyTupleElementsOpt => {
                        let handle = KeyTupleElementsOptHandle(id);
                        self.visit_key_tuple_elements_opt_handle(handle, tree)?;
                    }
                    NonTerminalKind::KeyTupleElementsTail => {
                        let handle = KeyTupleElementsTailHandle(id);
                        self.visit_key_tuple_elements_tail_handle(handle, tree)?;
                    }
                    NonTerminalKind::KeyTupleElementsTailOpt => {
                        let handle = KeyTupleElementsTailOptHandle(id);
                        self.visit_key_tuple_elements_tail_opt_handle(handle, tree)?;
                    }
                    NonTerminalKind::KeyTupleOpt => {
                        let handle = KeyTupleOptHandle(id);
                        self.visit_key_tuple_opt_handle(handle, tree)?;
                    }
                    NonTerminalKind::KeyValue => {
                        let handle = KeyValueHandle(id);
                        self.visit_key_value_handle(handle, tree)?;
                    }
                    NonTerminalKind::Keys => {
                        let handle = KeysHandle(id);
                        self.visit_keys_handle(handle, tree)?;
                    }
                    NonTerminalKind::KeysList => {
                        let handle = KeysListHandle(id);
                        self.visit_keys_list_handle(handle, tree)?;
                    }
                    NonTerminalKind::LParen => {
                        let handle = LParenHandle(id);
                        self.visit_l_paren_handle(handle, tree)?;
                    }
                    NonTerminalKind::NoBacktick => {
                        let handle = NoBacktickHandle(id);
                        self.visit_no_backtick_handle(handle, tree)?;
                    }
                    NonTerminalKind::NoBacktickInline => {
                        let handle = NoBacktickInlineHandle(id);
                        self.visit_no_backtick_inline_handle(handle, tree)?;
                    }
                    NonTerminalKind::Null => {
                        let handle = NullHandle(id);
                        self.visit_null_handle(handle, tree)?;
                    }
                    NonTerminalKind::Object => {
                        let handle = ObjectHandle(id);
                        self.visit_object_handle(handle, tree)?;
                    }
                    NonTerminalKind::ObjectList => {
                        let handle = ObjectListHandle(id);
                        self.visit_object_list_handle(handle, tree)?;
                    }
                    NonTerminalKind::ObjectOpt => {
                        let handle = ObjectOptHandle(id);
                        self.visit_object_opt_handle(handle, tree)?;
                    }
                    NonTerminalKind::Path => {
                        let handle = PathHandle(id);
                        self.visit_path_handle(handle, tree)?;
                    }
                    NonTerminalKind::RParen => {
                        let handle = RParenHandle(id);
                        self.visit_r_paren_handle(handle, tree)?;
                    }
                    NonTerminalKind::Section => {
                        let handle = SectionHandle(id);
                        self.visit_section_handle(handle, tree)?;
                    }
                    NonTerminalKind::SectionBinding => {
                        let handle = SectionBindingHandle(id);
                        self.visit_section_binding_handle(handle, tree)?;
                    }
                    NonTerminalKind::SectionBody => {
                        let handle = SectionBodyHandle(id);
                        self.visit_section_body_handle(handle, tree)?;
                    }
                    NonTerminalKind::SectionBodyList => {
                        let handle = SectionBodyListHandle(id);
                        self.visit_section_body_list_handle(handle, tree)?;
                    }
                    NonTerminalKind::SectionBodyOpt => {
                        let handle = SectionBodyOptHandle(id);
                        self.visit_section_body_opt_handle(handle, tree)?;
                    }
                    NonTerminalKind::Str => {
                        let handle = StrHandle(id);
                        self.visit_str_handle(handle, tree)?;
                    }
                    NonTerminalKind::Strings => {
                        let handle = StringsHandle(id);
                        self.visit_strings_handle(handle, tree)?;
                    }
                    NonTerminalKind::StringsList => {
                        let handle = StringsListHandle(id);
                        self.visit_strings_list_handle(handle, tree)?;
                    }
                    NonTerminalKind::Text => {
                        let handle = TextHandle(id);
                        self.visit_text_handle(handle, tree)?;
                    }
                    NonTerminalKind::TextBinding => {
                        let handle = TextBindingHandle(id);
                        self.visit_text_binding_handle(handle, tree)?;
                    }
                    NonTerminalKind::TextBindingOpt => {
                        let handle = TextBindingOptHandle(id);
                        self.visit_text_binding_opt_handle(handle, tree)?;
                    }
                    NonTerminalKind::TextBindingOpt0 => {
                        let handle = TextBindingOpt0Handle(id);
                        self.visit_text_binding_opt_0_handle(handle, tree)?;
                    }
                    NonTerminalKind::TextStart => {
                        let handle = TextStartHandle(id);
                        self.visit_text_start_handle(handle, tree)?;
                    }
                    NonTerminalKind::True => {
                        let handle = TrueHandle(id);
                        self.visit_true_handle(handle, tree)?;
                    }
                    NonTerminalKind::Tuple => {
                        let handle = TupleHandle(id);
                        self.visit_tuple_handle(handle, tree)?;
                    }
                    NonTerminalKind::TupleElements => {
                        let handle = TupleElementsHandle(id);
                        self.visit_tuple_elements_handle(handle, tree)?;
                    }
                    NonTerminalKind::TupleElementsOpt => {
                        let handle = TupleElementsOptHandle(id);
                        self.visit_tuple_elements_opt_handle(handle, tree)?;
                    }
                    NonTerminalKind::TupleElementsTail => {
                        let handle = TupleElementsTailHandle(id);
                        self.visit_tuple_elements_tail_handle(handle, tree)?;
                    }
                    NonTerminalKind::TupleElementsTailOpt => {
                        let handle = TupleElementsTailOptHandle(id);
                        self.visit_tuple_elements_tail_opt_handle(handle, tree)?;
                    }
                    NonTerminalKind::TupleIndex => {
                        let handle = TupleIndexHandle(id);
                        self.visit_tuple_index_handle(handle, tree)?;
                    }
                    NonTerminalKind::TupleOpt => {
                        let handle = TupleOptHandle(id);
                        self.visit_tuple_opt_handle(handle, tree)?;
                    }
                    NonTerminalKind::Value => {
                        let handle = ValueHandle(id);
                        self.visit_value_handle(handle, tree)?;
                    }
                    NonTerminalKind::ValueBinding => {
                        let handle = ValueBindingHandle(id);
                        self.visit_value_binding_handle(handle, tree)?;
                    }
                    NonTerminalKind::Ws => {
                        let handle = WsHandle(id);
                        self.visit_ws_handle(handle, tree)?;
                    }
                    NonTerminalKind::Root => {
                        let handle = RootHandle(id);
                        self.visit_root_handle(handle, tree)?;
                    }
                }
            }
            CstNode::Terminal { kind, data } => {
                match kind {
                    TerminalKind::NewLine => {
                        let terminal = NewLine(id);
                        self.visit_new_line_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::Whitespace => {
                        let terminal = Whitespace(id);
                        self.visit_whitespace_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::LineComment => {
                        let terminal = LineComment(id);
                        self.visit_line_comment_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::BlockComment => {
                        let terminal = BlockComment(id);
                        self.visit_block_comment_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::Hash => {
                        let terminal = Hash(id);
                        self.visit_hash_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::Float => {
                        let terminal = Float(id);
                        self.visit_float_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::Integer => {
                        let terminal = Integer(id);
                        self.visit_integer_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::True => {
                        let terminal = True(id);
                        self.visit_true_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::False => {
                        let terminal = False(id);
                        self.visit_false_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::Null => {
                        let terminal = Null(id);
                        self.visit_null_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::Hole => {
                        let terminal = Hole(id);
                        self.visit_hole_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::Str => {
                        let terminal = Str(id);
                        self.visit_str_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::Text => {
                        let terminal = Text(id);
                        self.visit_text_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::InlineCode1 => {
                        let terminal = InlineCode1(id);
                        self.visit_inline_code_1_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::InlineCodeStart2 => {
                        let terminal = InlineCodeStart2(id);
                        self.visit_inline_code_start_2_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::CodeBlockStart3 => {
                        let terminal = CodeBlockStart3(id);
                        self.visit_code_block_start_3_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::CodeBlockStart4 => {
                        let terminal = CodeBlockStart4(id);
                        self.visit_code_block_start_4_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::CodeBlockStart5 => {
                        let terminal = CodeBlockStart5(id);
                        self.visit_code_block_start_5_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::CodeBlockStart6 => {
                        let terminal = CodeBlockStart6(id);
                        self.visit_code_block_start_6_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::CodeBlockEnd3 => {
                        let terminal = CodeBlockEnd3(id);
                        self.visit_code_block_end_3_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::Backtick2 => {
                        let terminal = Backtick2(id);
                        self.visit_backtick_2_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::CodeBlockEnd4 => {
                        let terminal = CodeBlockEnd4(id);
                        self.visit_code_block_end_4_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::Backtick3 => {
                        let terminal = Backtick3(id);
                        self.visit_backtick_3_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::CodeBlockEnd5 => {
                        let terminal = CodeBlockEnd5(id);
                        self.visit_code_block_end_5_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::Backtick4 => {
                        let terminal = Backtick4(id);
                        self.visit_backtick_4_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::CodeBlockEnd6 => {
                        let terminal = CodeBlockEnd6(id);
                        self.visit_code_block_end_6_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::Backtick5 => {
                        let terminal = Backtick5(id);
                        self.visit_backtick_5_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::InlineCodeEnd2 => {
                        let terminal = InlineCodeEnd2(id);
                        self.visit_inline_code_end_2_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::Backtick1 => {
                        let terminal = Backtick1(id);
                        self.visit_backtick_1_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::NoBacktick => {
                        let terminal = NoBacktick(id);
                        self.visit_no_backtick_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::NoBacktickInline => {
                        let terminal = NoBacktickInline(id);
                        self.visit_no_backtick_inline_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::GrammarNewline => {
                        let terminal = GrammarNewline(id);
                        self.visit_grammar_newline_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::Ws => {
                        let terminal = Ws(id);
                        self.visit_ws_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::At => {
                        let terminal = At(id);
                        self.visit_at_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::Dollar => {
                        let terminal = Dollar(id);
                        self.visit_dollar_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::Dot => {
                        let terminal = Dot(id);
                        self.visit_dot_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::LBrace => {
                        let terminal = LBrace(id);
                        self.visit_l_brace_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::RBrace => {
                        let terminal = RBrace(id);
                        self.visit_r_brace_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::LBracket => {
                        let terminal = LBracket(id);
                        self.visit_l_bracket_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::RBracket => {
                        let terminal = RBracket(id);
                        self.visit_r_bracket_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::LParen => {
                        let terminal = LParen(id);
                        self.visit_l_paren_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::RParen => {
                        let terminal = RParen(id);
                        self.visit_r_paren_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::Bind => {
                        let terminal = Bind(id);
                        self.visit_bind_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::Comma => {
                        let terminal = Comma(id);
                        self.visit_comma_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::Esc => {
                        let terminal = Esc(id);
                        self.visit_esc_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::TextStart => {
                        let terminal = TextStart(id);
                        self.visit_text_start_terminal(terminal, data, tree)?;
                    }
                    TerminalKind::Ident => {
                        let terminal = Ident(id);
                        self.visit_ident_terminal(terminal, data, tree)?;
                    }
                }
            }
        }
        Ok(())
    }
}
mod private2 {
    pub trait Sealed {}
}
pub trait NodeVisitor: NodeVisitorSuper<Self::Error> {
    type Error;
    fn visit_node(
        &mut self,
        id: CstNodeId,
        node: CstNode,
        tree: &Cst,
    ) -> Result<(), Self::Error>;
}
pub trait NodeVisitorSuper<E>: private2::Sealed {
    fn visit_node_id(&mut self, id: CstNodeId, tree: &Cst) -> Result<(), E>;
    fn visit_node_super(
        &mut self,
        id: CstNodeId,
        node: CstNode,
        tree: &Cst,
    ) -> Result<(), E>;
}
impl<V: NodeVisitor> private2::Sealed for V {}
impl<V: NodeVisitor> NodeVisitorSuper<V::Error> for V {
    fn visit_node_id(&mut self, id: CstNodeId, tree: &Cst) -> Result<(), V::Error> {
        if let Some(node) = tree.node_data(id) {
            self.visit_node(id, node, tree)
        } else {
            Ok(())
        }
    }
    fn visit_node_super(
        &mut self,
        id: CstNodeId,
        _node: CstNode,
        tree: &Cst,
    ) -> Result<(), V::Error> {
        for child in tree.children(id) {
            if let Some(child_node) = tree.node_data(child) {
                self.visit_node(child, child_node, tree)?;
            }
        }
        Ok(())
    }
}
pub trait BuiltinTerminalVisitor<E, F: CstFacade> {
    fn visit_builtin_new_line_terminal(
        &mut self,
        terminal: NewLine,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_builtin_whitespace_terminal(
        &mut self,
        terminal: Whitespace,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_builtin_line_comment_terminal(
        &mut self,
        terminal: LineComment,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
    fn visit_builtin_block_comment_terminal(
        &mut self,
        terminal: BlockComment,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
}
impl<V: CstVisitor<F>, F: CstFacade> BuiltinTerminalVisitor<V::Error, F> for V {
    fn visit_builtin_new_line_terminal(
        &mut self,
        terminal: NewLine,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_new_line_terminal(terminal, data, tree)
    }
    fn visit_builtin_whitespace_terminal(
        &mut self,
        terminal: Whitespace,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_whitespace_terminal(terminal, data, tree)
    }
    fn visit_builtin_line_comment_terminal(
        &mut self,
        terminal: LineComment,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_line_comment_terminal(terminal, data, tree)
    }
    fn visit_builtin_block_comment_terminal(
        &mut self,
        terminal: BlockComment,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), V::Error> {
        self.visit_block_comment_terminal(terminal, data, tree)
    }
}
