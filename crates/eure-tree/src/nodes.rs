#![allow(unused_variables)]
use super::tree::{
    TerminalHandle, NonTerminalHandle, RecursiveView, CstNodeId, ViewConstructionError,
    CstFacade,
};
use super::visitor::BuiltinTerminalVisitor;
use crate::CstConstructError;
use super::node_kind::{TerminalKind, NonTerminalKind, NodeKind};
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArrayHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for ArrayHandle {
    type View = ArrayView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Array)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Array
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::ArrayBegin),
                NodeKind::NonTerminal(NonTerminalKind::ArrayOpt),
                NodeKind::NonTerminal(NonTerminalKind::ArrayEnd),
            ],
            |[array_begin, array_opt, array_end], visit_ignored| Ok(
                visit(
                    ArrayView {
                        array_begin: ArrayBeginHandle(array_begin),
                        array_opt: ArrayOptHandle(array_opt),
                        array_end: ArrayEndHandle(array_end),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArrayView {
    pub array_begin: ArrayBeginHandle,
    pub array_opt: ArrayOptHandle,
    pub array_end: ArrayEndHandle,
}
impl ArrayView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArrayBeginHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for ArrayBeginHandle {
    type View = ArrayBeginView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::ArrayBegin)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::ArrayBegin
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::LBracket)],
            |[l_bracket], visit_ignored| Ok(
                visit(
                    ArrayBeginView {
                        l_bracket: LBracket(l_bracket),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArrayBeginView {
    pub l_bracket: LBracket,
}
impl ArrayBeginView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArrayElementsHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for ArrayElementsHandle {
    type View = ArrayElementsView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::ArrayElements)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::ArrayElements
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Value),
                NodeKind::NonTerminal(NonTerminalKind::ArrayElementsOpt),
            ],
            |[value, array_elements_opt], visit_ignored| Ok(
                visit(
                    ArrayElementsView {
                        value: ValueHandle(value),
                        array_elements_opt: ArrayElementsOptHandle(array_elements_opt),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArrayElementsView {
    pub value: ValueHandle,
    pub array_elements_opt: ArrayElementsOptHandle,
}
impl ArrayElementsView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArrayElementsOptHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for ArrayElementsOptHandle {
    type View = Option<ArrayElementsTailHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::ArrayElementsOpt)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::ArrayElementsOpt
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(
            visit(
                    Some(
                        ArrayElementsTailHandle::new_with_visit(
                            self.0,
                            tree,
                            visit_ignored,
                        )?,
                    ),
                    visit_ignored,
                )
                .0,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArrayElementsTailHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for ArrayElementsTailHandle {
    type View = ArrayElementsTailView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::ArrayElementsTail)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::ArrayElementsTail
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Comma),
                NodeKind::NonTerminal(NonTerminalKind::ArrayElementsTailOpt),
            ],
            |[comma, array_elements_tail_opt], visit_ignored| Ok(
                visit(
                    ArrayElementsTailView {
                        comma: CommaHandle(comma),
                        array_elements_tail_opt: ArrayElementsTailOptHandle(
                            array_elements_tail_opt,
                        ),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArrayElementsTailView {
    pub comma: CommaHandle,
    pub array_elements_tail_opt: ArrayElementsTailOptHandle,
}
impl ArrayElementsTailView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArrayElementsTailOptHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for ArrayElementsTailOptHandle {
    type View = Option<ArrayElementsHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::ArrayElementsTailOpt)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::ArrayElementsTailOpt
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(
            visit(
                    Some(
                        ArrayElementsHandle::new_with_visit(self.0, tree, visit_ignored)?,
                    ),
                    visit_ignored,
                )
                .0,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArrayEndHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for ArrayEndHandle {
    type View = ArrayEndView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::ArrayEnd)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::ArrayEnd
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::RBracket)],
            |[r_bracket], visit_ignored| Ok(
                visit(
                    ArrayEndView {
                        r_bracket: RBracket(r_bracket),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArrayEndView {
    pub r_bracket: RBracket,
}
impl ArrayEndView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArrayMarkerHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for ArrayMarkerHandle {
    type View = ArrayMarkerView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::ArrayMarker)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::ArrayMarker
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::ArrayBegin),
                NodeKind::NonTerminal(NonTerminalKind::ArrayMarkerOpt),
                NodeKind::NonTerminal(NonTerminalKind::ArrayEnd),
            ],
            |[array_begin, array_marker_opt, array_end], visit_ignored| Ok(
                visit(
                    ArrayMarkerView {
                        array_begin: ArrayBeginHandle(array_begin),
                        array_marker_opt: ArrayMarkerOptHandle(array_marker_opt),
                        array_end: ArrayEndHandle(array_end),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArrayMarkerView {
    pub array_begin: ArrayBeginHandle,
    pub array_marker_opt: ArrayMarkerOptHandle,
    pub array_end: ArrayEndHandle,
}
impl ArrayMarkerView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArrayMarkerOptHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for ArrayMarkerOptHandle {
    type View = Option<IntegerHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::ArrayMarkerOpt)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::ArrayMarkerOpt
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(
            visit(
                    Some(IntegerHandle::new_with_visit(self.0, tree, visit_ignored)?),
                    visit_ignored,
                )
                .0,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArrayOptHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for ArrayOptHandle {
    type View = Option<ArrayElementsHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::ArrayOpt)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::ArrayOpt
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(
            visit(
                    Some(
                        ArrayElementsHandle::new_with_visit(self.0, tree, visit_ignored)?,
                    ),
                    visit_ignored,
                )
                .0,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AtHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for AtHandle {
    type View = AtView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::At)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::At
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::At)],
            |[at], visit_ignored| Ok(visit(AtView { at: At(at) }, visit_ignored)),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtView {
    pub at: At,
}
impl AtView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BeginHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for BeginHandle {
    type View = BeginView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Begin)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Begin
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::LBrace)],
            |[l_brace], visit_ignored| Ok(
                visit(
                    BeginView {
                        l_brace: LBrace(l_brace),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BeginView {
    pub l_brace: LBrace,
}
impl BeginView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BindHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for BindHandle {
    type View = BindView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Bind)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Bind
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Bind)],
            |[bind], visit_ignored| Ok(
                visit(BindView { bind: Bind(bind) }, visit_ignored),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindView {
    pub bind: Bind,
}
impl BindView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BindingHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for BindingHandle {
    type View = BindingView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Binding)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Binding
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Keys),
                NodeKind::NonTerminal(NonTerminalKind::BindingRhs),
            ],
            |[keys, binding_rhs], visit_ignored| Ok(
                visit(
                    BindingView {
                        keys: KeysHandle(keys),
                        binding_rhs: BindingRhsHandle(binding_rhs),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindingView {
    pub keys: KeysHandle,
    pub binding_rhs: BindingRhsHandle,
}
impl BindingView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BindingRhsHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for BindingRhsHandle {
    type View = BindingRhsView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::BindingRhs)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::BindingRhs
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        let mut children = tree.children(self.0);
        let Some(child) = children.next() else {
            return Err(ViewConstructionError::UnexpectedEndOfChildren {
                parent: self.0,
            });
        };
        let Some(child_data) = tree.node_data(child) else {
            return Err(ViewConstructionError::NodeIdNotFound {
                node: child,
            });
        };
        let variant = match child_data.node_kind() {
            NodeKind::NonTerminal(NonTerminalKind::ValueBinding) => {
                BindingRhsView::ValueBinding(ValueBindingHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::SectionBinding) => {
                BindingRhsView::SectionBinding(SectionBindingHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::TextBinding) => {
                BindingRhsView::TextBinding(TextBindingHandle(child))
            }
            _ => {
                return Err(ViewConstructionError::UnexpectedNode {
                    node: child,
                    data: child_data,
                    expected_kind: child_data.node_kind(),
                });
            }
        };
        let (result, _visit) = visit(variant, visit_ignored);
        if let Some(child) = children.next() {
            return Err(ViewConstructionError::UnexpectedExtraNode {
                node: child,
            });
        }
        Ok(result)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingRhsView {
    ValueBinding(ValueBindingHandle),
    SectionBinding(SectionBindingHandle),
    TextBinding(TextBindingHandle),
}
impl BindingRhsView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BooleanHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for BooleanHandle {
    type View = BooleanView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Boolean)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Boolean
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        let mut children = tree.children(self.0);
        let Some(child) = children.next() else {
            return Err(ViewConstructionError::UnexpectedEndOfChildren {
                parent: self.0,
            });
        };
        let Some(child_data) = tree.node_data(child) else {
            return Err(ViewConstructionError::NodeIdNotFound {
                node: child,
            });
        };
        let variant = match child_data.node_kind() {
            NodeKind::NonTerminal(NonTerminalKind::True) => {
                BooleanView::True(TrueHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::False) => {
                BooleanView::False(FalseHandle(child))
            }
            _ => {
                return Err(ViewConstructionError::UnexpectedNode {
                    node: child,
                    data: child_data,
                    expected_kind: child_data.node_kind(),
                });
            }
        };
        let (result, _visit) = visit(variant, visit_ignored);
        if let Some(child) = children.next() {
            return Err(ViewConstructionError::UnexpectedExtraNode {
                node: child,
            });
        }
        Ok(result)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanView {
    True(TrueHandle),
    False(FalseHandle),
}
impl BooleanView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodeHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for CodeHandle {
    type View = CodeView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Code)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Code
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Code)],
            |[code], visit_ignored| Ok(
                visit(CodeView { code: Code(code) }, visit_ignored),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeView {
    pub code: Code,
}
impl CodeView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodeBlockHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for CodeBlockHandle {
    type View = CodeBlockView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::CodeBlock)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::CodeBlock
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::CodeBlock)],
            |[code_block], visit_ignored| Ok(
                visit(
                    CodeBlockView {
                        code_block: CodeBlock(code_block),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeBlockView {
    pub code_block: CodeBlock,
}
impl CodeBlockView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommaHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for CommaHandle {
    type View = CommaView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Comma)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Comma
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Comma)],
            |[comma], visit_ignored| Ok(
                visit(CommaView { comma: Comma(comma) }, visit_ignored),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommaView {
    pub comma: Comma,
}
impl CommaView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContinueHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for ContinueHandle {
    type View = ContinueView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Continue)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Continue
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Esc)],
            |[esc], visit_ignored| Ok(
                visit(ContinueView { esc: Esc(esc) }, visit_ignored),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContinueView {
    pub esc: Esc,
}
impl ContinueView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DirectBindHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for DirectBindHandle {
    type View = DirectBindView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::DirectBind)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::DirectBind
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Bind),
                NodeKind::NonTerminal(NonTerminalKind::Value),
            ],
            |[bind, value], visit_ignored| Ok(
                visit(
                    DirectBindView {
                        bind: BindHandle(bind),
                        value: ValueHandle(value),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectBindView {
    pub bind: BindHandle,
    pub value: ValueHandle,
}
impl DirectBindView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DotHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for DotHandle {
    type View = DotView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Dot)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Dot
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Dot)],
            |[dot], visit_ignored| Ok(visit(DotView { dot: Dot(dot) }, visit_ignored)),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DotView {
    pub dot: Dot,
}
impl DotView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EndHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for EndHandle {
    type View = EndView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::End)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::End
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::RBrace)],
            |[r_brace], visit_ignored| Ok(
                visit(
                    EndView {
                        r_brace: RBrace(r_brace),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndView {
    pub r_brace: RBrace,
}
impl EndView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EureHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for EureHandle {
    type View = EureView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Eure)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Eure
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::EureList),
                NodeKind::NonTerminal(NonTerminalKind::EureList0),
            ],
            |[eure_bindings, eure_sections], visit_ignored| Ok(
                visit(
                    EureView {
                        eure_bindings: EureBindingsHandle(eure_bindings),
                        eure_sections: EureSectionsHandle(eure_sections),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EureView {
    pub eure_bindings: EureBindingsHandle,
    pub eure_sections: EureSectionsHandle,
}
impl EureView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EureBindingsHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for EureBindingsHandle {
    type View = Option<EureBindingsView>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::EureList)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::EureList
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Binding),
                NodeKind::NonTerminal(NonTerminalKind::EureList),
            ],
            |[binding, eure_bindings], visit_ignored| Ok(
                visit(
                    Some(EureBindingsView {
                        binding: BindingHandle(binding),
                        eure_bindings: EureBindingsHandle(eure_bindings),
                    }),
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EureBindingsView {
    pub binding: BindingHandle,
    pub eure_bindings: EureBindingsHandle,
}
impl<F: CstFacade> RecursiveView<F> for EureBindingsView {
    type Item = BindingHandle;
    fn get_all_with_visit<E>(
        &self,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Vec<Self::Item>, CstConstructError<E>> {
        let mut items = Vec::new();
        let mut current_view = Some(*self);
        while let Some(item) = current_view {
            let Self { binding, .. } = item;
            items.push(binding);
            item.eure_bindings
                .get_view_with_visit(
                    tree,
                    |view, visit_ignored| {
                        current_view = view;
                        ((), visit_ignored)
                    },
                    visit_ignored,
                )?;
        }
        Ok(items)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EureSectionsHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for EureSectionsHandle {
    type View = Option<EureSectionsView>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::EureList0)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::EureList0
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Section),
                NodeKind::NonTerminal(NonTerminalKind::EureList0),
            ],
            |[section, eure_sections], visit_ignored| Ok(
                visit(
                    Some(EureSectionsView {
                        section: SectionHandle(section),
                        eure_sections: EureSectionsHandle(eure_sections),
                    }),
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EureSectionsView {
    pub section: SectionHandle,
    pub eure_sections: EureSectionsHandle,
}
impl<F: CstFacade> RecursiveView<F> for EureSectionsView {
    type Item = SectionHandle;
    fn get_all_with_visit<E>(
        &self,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Vec<Self::Item>, CstConstructError<E>> {
        let mut items = Vec::new();
        let mut current_view = Some(*self);
        while let Some(item) = current_view {
            let Self { section, .. } = item;
            items.push(section);
            item.eure_sections
                .get_view_with_visit(
                    tree,
                    |view, visit_ignored| {
                        current_view = view;
                        ((), visit_ignored)
                    },
                    visit_ignored,
                )?;
        }
        Ok(items)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExtHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for ExtHandle {
    type View = ExtView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Ext)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Ext
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Dollar)],
            |[dollar], visit_ignored| Ok(
                visit(ExtView { dollar: Dollar(dollar) }, visit_ignored),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtView {
    pub dollar: Dollar,
}
impl ExtView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExtensionNameSpaceHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for ExtensionNameSpaceHandle {
    type View = ExtensionNameSpaceView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::ExtensionNameSpace)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::ExtensionNameSpace
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Ext),
                NodeKind::NonTerminal(NonTerminalKind::Ident),
            ],
            |[ext, ident], visit_ignored| Ok(
                visit(
                    ExtensionNameSpaceView {
                        ext: ExtHandle(ext),
                        ident: IdentHandle(ident),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtensionNameSpaceView {
    pub ext: ExtHandle,
    pub ident: IdentHandle,
}
impl ExtensionNameSpaceView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FalseHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for FalseHandle {
    type View = FalseView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::False)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::False
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::False)],
            |[r#false], visit_ignored| Ok(
                visit(
                    FalseView {
                        r#false: False(r#false),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FalseView {
    pub r#false: False,
}
impl FalseView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GrammarNewlineHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for GrammarNewlineHandle {
    type View = GrammarNewlineView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::GrammarNewline)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::GrammarNewline
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::GrammarNewline)],
            |[grammar_newline], visit_ignored| Ok(
                visit(
                    GrammarNewlineView {
                        grammar_newline: GrammarNewline(grammar_newline),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrammarNewlineView {
    pub grammar_newline: GrammarNewline,
}
impl GrammarNewlineView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HoleHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for HoleHandle {
    type View = HoleView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Hole)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Hole
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Hole)],
            |[hole], visit_ignored| Ok(
                visit(HoleView { hole: Hole(hole) }, visit_ignored),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HoleView {
    pub hole: Hole,
}
impl HoleView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdentHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for IdentHandle {
    type View = IdentView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Ident)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Ident
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Ident)],
            |[ident], visit_ignored| Ok(
                visit(IdentView { ident: Ident(ident) }, visit_ignored),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdentView {
    pub ident: Ident,
}
impl IdentView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IntegerHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for IntegerHandle {
    type View = IntegerView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Integer)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Integer
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Integer)],
            |[integer], visit_ignored| Ok(
                visit(
                    IntegerView {
                        integer: Integer(integer),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntegerView {
    pub integer: Integer,
}
impl IntegerView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for KeyHandle {
    type View = KeyView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Key)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Key
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::KeyBase),
                NodeKind::NonTerminal(NonTerminalKind::KeyOpt),
            ],
            |[key_base, key_opt], visit_ignored| Ok(
                visit(
                    KeyView {
                        key_base: KeyBaseHandle(key_base),
                        key_opt: KeyOptHandle(key_opt),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyView {
    pub key_base: KeyBaseHandle,
    pub key_opt: KeyOptHandle,
}
impl KeyView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyBaseHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for KeyBaseHandle {
    type View = KeyBaseView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::KeyBase)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::KeyBase
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        let mut children = tree.children(self.0);
        let Some(child) = children.next() else {
            return Err(ViewConstructionError::UnexpectedEndOfChildren {
                parent: self.0,
            });
        };
        let Some(child_data) = tree.node_data(child) else {
            return Err(ViewConstructionError::NodeIdNotFound {
                node: child,
            });
        };
        let variant = match child_data.node_kind() {
            NodeKind::NonTerminal(NonTerminalKind::Ident) => {
                KeyBaseView::Ident(IdentHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::ExtensionNameSpace) => {
                KeyBaseView::ExtensionNameSpace(ExtensionNameSpaceHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::Str) => {
                KeyBaseView::Str(StrHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::Integer) => {
                KeyBaseView::Integer(IntegerHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::MetaExtKey) => {
                KeyBaseView::MetaExtKey(MetaExtKeyHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::Null) => {
                KeyBaseView::Null(NullHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::True) => {
                KeyBaseView::True(TrueHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::False) => {
                KeyBaseView::False(FalseHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::Hole) => {
                KeyBaseView::Hole(HoleHandle(child))
            }
            _ => {
                return Err(ViewConstructionError::UnexpectedNode {
                    node: child,
                    data: child_data,
                    expected_kind: child_data.node_kind(),
                });
            }
        };
        let (result, _visit) = visit(variant, visit_ignored);
        if let Some(child) = children.next() {
            return Err(ViewConstructionError::UnexpectedExtraNode {
                node: child,
            });
        }
        Ok(result)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyBaseView {
    Ident(IdentHandle),
    ExtensionNameSpace(ExtensionNameSpaceHandle),
    Str(StrHandle),
    Integer(IntegerHandle),
    MetaExtKey(MetaExtKeyHandle),
    Null(NullHandle),
    True(TrueHandle),
    False(FalseHandle),
    Hole(HoleHandle),
}
impl KeyBaseView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyOptHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for KeyOptHandle {
    type View = Option<ArrayMarkerHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::KeyOpt)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::KeyOpt
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(
            visit(
                    Some(
                        ArrayMarkerHandle::new_with_visit(self.0, tree, visit_ignored)?,
                    ),
                    visit_ignored,
                )
                .0,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeysHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for KeysHandle {
    type View = KeysView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Keys)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Keys
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Key),
                NodeKind::NonTerminal(NonTerminalKind::KeysList),
            ],
            |[key, keys_list], visit_ignored| Ok(
                visit(
                    KeysView {
                        key: KeyHandle(key),
                        keys_list: KeysListHandle(keys_list),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeysView {
    pub key: KeyHandle,
    pub keys_list: KeysListHandle,
}
impl KeysView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeysListHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for KeysListHandle {
    type View = Option<KeysListView>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::KeysList)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::KeysList
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Dot),
                NodeKind::NonTerminal(NonTerminalKind::Key),
                NodeKind::NonTerminal(NonTerminalKind::KeysList),
            ],
            |[dot, key, keys_list], visit_ignored| Ok(
                visit(
                    Some(KeysListView {
                        dot: DotHandle(dot),
                        key: KeyHandle(key),
                        keys_list: KeysListHandle(keys_list),
                    }),
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeysListView {
    pub dot: DotHandle,
    pub key: KeyHandle,
    pub keys_list: KeysListHandle,
}
impl<F: CstFacade> RecursiveView<F> for KeysListView {
    type Item = KeysListItem;
    fn get_all_with_visit<E>(
        &self,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Vec<Self::Item>, CstConstructError<E>> {
        let mut items = Vec::new();
        let mut current_view = Some(*self);
        while let Some(item) = current_view {
            let Self { dot, key, .. } = item;
            items.push(KeysListItem { dot, key });
            item.keys_list
                .get_view_with_visit(
                    tree,
                    |view, visit_ignored| {
                        current_view = view;
                        ((), visit_ignored)
                    },
                    visit_ignored,
                )?;
        }
        Ok(items)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeysListItem {
    pub dot: DotHandle,
    pub key: KeyHandle,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LParenHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for LParenHandle {
    type View = LParenView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::LParen)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::LParen
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::LParen)],
            |[l_paren], visit_ignored| Ok(
                visit(
                    LParenView {
                        l_paren: LParen(l_paren),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LParenView {
    pub l_paren: LParen,
}
impl LParenView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MetaExtHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for MetaExtHandle {
    type View = MetaExtView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::MetaExt)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::MetaExt
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::DollarDollar)],
            |[dollar_dollar], visit_ignored| Ok(
                visit(
                    MetaExtView {
                        dollar_dollar: DollarDollar(dollar_dollar),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MetaExtView {
    pub dollar_dollar: DollarDollar,
}
impl MetaExtView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MetaExtKeyHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for MetaExtKeyHandle {
    type View = MetaExtKeyView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::MetaExtKey)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::MetaExtKey
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::MetaExt),
                NodeKind::NonTerminal(NonTerminalKind::Ident),
            ],
            |[meta_ext, ident], visit_ignored| Ok(
                visit(
                    MetaExtKeyView {
                        meta_ext: MetaExtHandle(meta_ext),
                        ident: IdentHandle(ident),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MetaExtKeyView {
    pub meta_ext: MetaExtHandle,
    pub ident: IdentHandle,
}
impl MetaExtKeyView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NamedCodeHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for NamedCodeHandle {
    type View = NamedCodeView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::NamedCode)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::NamedCode
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::NamedCode)],
            |[named_code], visit_ignored| Ok(
                visit(
                    NamedCodeView {
                        named_code: NamedCode(named_code),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NamedCodeView {
    pub named_code: NamedCode,
}
impl NamedCodeView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NullHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for NullHandle {
    type View = NullView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Null)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Null
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Null)],
            |[null], visit_ignored| Ok(
                visit(NullView { null: Null(null) }, visit_ignored),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NullView {
    pub null: Null,
}
impl NullView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for ObjectHandle {
    type View = ObjectView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Object)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Object
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Begin),
                NodeKind::NonTerminal(NonTerminalKind::ObjectList),
                NodeKind::NonTerminal(NonTerminalKind::End),
            ],
            |[begin, object_list, end], visit_ignored| Ok(
                visit(
                    ObjectView {
                        begin: BeginHandle(begin),
                        object_list: ObjectListHandle(object_list),
                        end: EndHandle(end),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectView {
    pub begin: BeginHandle,
    pub object_list: ObjectListHandle,
    pub end: EndHandle,
}
impl ObjectView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectListHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for ObjectListHandle {
    type View = Option<ObjectListView>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::ObjectList)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::ObjectList
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Key),
                NodeKind::NonTerminal(NonTerminalKind::Bind),
                NodeKind::NonTerminal(NonTerminalKind::Value),
                NodeKind::NonTerminal(NonTerminalKind::ObjectOpt),
                NodeKind::NonTerminal(NonTerminalKind::ObjectList),
            ],
            |[key, bind, value, object_opt, object_list], visit_ignored| Ok(
                visit(
                    Some(ObjectListView {
                        key: KeyHandle(key),
                        bind: BindHandle(bind),
                        value: ValueHandle(value),
                        object_opt: ObjectOptHandle(object_opt),
                        object_list: ObjectListHandle(object_list),
                    }),
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectListView {
    pub key: KeyHandle,
    pub bind: BindHandle,
    pub value: ValueHandle,
    pub object_opt: ObjectOptHandle,
    pub object_list: ObjectListHandle,
}
impl<F: CstFacade> RecursiveView<F> for ObjectListView {
    type Item = ObjectListItem;
    fn get_all_with_visit<E>(
        &self,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Vec<Self::Item>, CstConstructError<E>> {
        let mut items = Vec::new();
        let mut current_view = Some(*self);
        while let Some(item) = current_view {
            let Self { key, bind, value, object_opt, .. } = item;
            items
                .push(ObjectListItem {
                    key,
                    bind,
                    value,
                    object_opt,
                });
            item.object_list
                .get_view_with_visit(
                    tree,
                    |view, visit_ignored| {
                        current_view = view;
                        ((), visit_ignored)
                    },
                    visit_ignored,
                )?;
        }
        Ok(items)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectListItem {
    pub key: KeyHandle,
    pub bind: BindHandle,
    pub value: ValueHandle,
    pub object_opt: ObjectOptHandle,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectOptHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for ObjectOptHandle {
    type View = Option<CommaHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::ObjectOpt)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::ObjectOpt
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(
            visit(
                    Some(CommaHandle::new_with_visit(self.0, tree, visit_ignored)?),
                    visit_ignored,
                )
                .0,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PathHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for PathHandle {
    type View = PathView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Path)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Path
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Dot),
                NodeKind::NonTerminal(NonTerminalKind::Keys),
            ],
            |[dot, keys], visit_ignored| Ok(
                visit(
                    PathView {
                        dot: DotHandle(dot),
                        keys: KeysHandle(keys),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathView {
    pub dot: DotHandle,
    pub keys: KeysHandle,
}
impl PathView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RParenHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for RParenHandle {
    type View = RParenView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::RParen)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::RParen
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::RParen)],
            |[r_paren], visit_ignored| Ok(
                visit(
                    RParenView {
                        r_paren: RParen(r_paren),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RParenView {
    pub r_paren: RParen,
}
impl RParenView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SectionHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for SectionHandle {
    type View = SectionView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Section)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Section
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::At),
                NodeKind::NonTerminal(NonTerminalKind::Keys),
                NodeKind::NonTerminal(NonTerminalKind::SectionBody),
            ],
            |[at, keys, section_body], visit_ignored| Ok(
                visit(
                    SectionView {
                        at: AtHandle(at),
                        keys: KeysHandle(keys),
                        section_body: SectionBodyHandle(section_body),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionView {
    pub at: AtHandle,
    pub keys: KeysHandle,
    pub section_body: SectionBodyHandle,
}
impl SectionView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SectionBindingHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for SectionBindingHandle {
    type View = SectionBindingView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::SectionBinding)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::SectionBinding
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Begin),
                NodeKind::NonTerminal(NonTerminalKind::Eure),
                NodeKind::NonTerminal(NonTerminalKind::End),
            ],
            |[begin, eure, end], visit_ignored| Ok(
                visit(
                    SectionBindingView {
                        begin: BeginHandle(begin),
                        eure: EureHandle(eure),
                        end: EndHandle(end),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionBindingView {
    pub begin: BeginHandle,
    pub eure: EureHandle,
    pub end: EndHandle,
}
impl SectionBindingView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SectionBodyHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for SectionBodyHandle {
    type View = SectionBodyView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::SectionBody)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::SectionBody
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        let mut children = tree.children(self.0);
        let Some(child) = children.next() else {
            return Err(ViewConstructionError::UnexpectedEndOfChildren {
                parent: self.0,
            });
        };
        let Some(child_data) = tree.node_data(child) else {
            return Err(ViewConstructionError::NodeIdNotFound {
                node: child,
            });
        };
        let variant = match child_data.node_kind() {
            NodeKind::NonTerminal(NonTerminalKind::SectionBodyList) => {
                SectionBodyView::SectionBodyList(SectionBodyListHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::SectionBinding) => {
                SectionBodyView::SectionBinding(SectionBindingHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::DirectBind) => {
                SectionBodyView::DirectBind(DirectBindHandle(child))
            }
            _ => {
                return Err(ViewConstructionError::UnexpectedNode {
                    node: child,
                    data: child_data,
                    expected_kind: child_data.node_kind(),
                });
            }
        };
        let (result, _visit) = visit(variant, visit_ignored);
        if let Some(child) = children.next() {
            return Err(ViewConstructionError::UnexpectedExtraNode {
                node: child,
            });
        }
        Ok(result)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionBodyView {
    SectionBodyList(SectionBodyListHandle),
    SectionBinding(SectionBindingHandle),
    DirectBind(DirectBindHandle),
}
impl SectionBodyView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SectionBodyListHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for SectionBodyListHandle {
    type View = Option<SectionBodyListView>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::SectionBodyList)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::SectionBodyList
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Binding),
                NodeKind::NonTerminal(NonTerminalKind::SectionBodyList),
            ],
            |[binding, section_body_list], visit_ignored| Ok(
                visit(
                    Some(SectionBodyListView {
                        binding: BindingHandle(binding),
                        section_body_list: SectionBodyListHandle(section_body_list),
                    }),
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionBodyListView {
    pub binding: BindingHandle,
    pub section_body_list: SectionBodyListHandle,
}
impl<F: CstFacade> RecursiveView<F> for SectionBodyListView {
    type Item = BindingHandle;
    fn get_all_with_visit<E>(
        &self,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Vec<Self::Item>, CstConstructError<E>> {
        let mut items = Vec::new();
        let mut current_view = Some(*self);
        while let Some(item) = current_view {
            let Self { binding, .. } = item;
            items.push(binding);
            item.section_body_list
                .get_view_with_visit(
                    tree,
                    |view, visit_ignored| {
                        current_view = view;
                        ((), visit_ignored)
                    },
                    visit_ignored,
                )?;
        }
        Ok(items)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StrHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for StrHandle {
    type View = StrView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Str)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Str
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Str)],
            |[str], visit_ignored| Ok(visit(StrView { str: Str(str) }, visit_ignored)),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrView {
    pub str: Str,
}
impl StrView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StringsHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for StringsHandle {
    type View = StringsView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Strings)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Strings
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Str),
                NodeKind::NonTerminal(NonTerminalKind::StringsList),
            ],
            |[str, strings_list], visit_ignored| Ok(
                visit(
                    StringsView {
                        str: StrHandle(str),
                        strings_list: StringsListHandle(strings_list),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringsView {
    pub str: StrHandle,
    pub strings_list: StringsListHandle,
}
impl StringsView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StringsListHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for StringsListHandle {
    type View = Option<StringsListView>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::StringsList)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::StringsList
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Continue),
                NodeKind::NonTerminal(NonTerminalKind::Str),
                NodeKind::NonTerminal(NonTerminalKind::StringsList),
            ],
            |[r#continue, str, strings_list], visit_ignored| Ok(
                visit(
                    Some(StringsListView {
                        r#continue: ContinueHandle(r#continue),
                        str: StrHandle(str),
                        strings_list: StringsListHandle(strings_list),
                    }),
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringsListView {
    pub r#continue: ContinueHandle,
    pub str: StrHandle,
    pub strings_list: StringsListHandle,
}
impl<F: CstFacade> RecursiveView<F> for StringsListView {
    type Item = StringsListItem;
    fn get_all_with_visit<E>(
        &self,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Vec<Self::Item>, CstConstructError<E>> {
        let mut items = Vec::new();
        let mut current_view = Some(*self);
        while let Some(item) = current_view {
            let Self { r#continue, str, .. } = item;
            items.push(StringsListItem { r#continue, str });
            item.strings_list
                .get_view_with_visit(
                    tree,
                    |view, visit_ignored| {
                        current_view = view;
                        ((), visit_ignored)
                    },
                    visit_ignored,
                )?;
        }
        Ok(items)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringsListItem {
    pub r#continue: ContinueHandle,
    pub str: StrHandle,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for TextHandle {
    type View = TextView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Text)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Text
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Text)],
            |[text], visit_ignored| Ok(
                visit(TextView { text: Text(text) }, visit_ignored),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextView {
    pub text: Text,
}
impl TextView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextBindingHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for TextBindingHandle {
    type View = TextBindingView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::TextBinding)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::TextBinding
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::TextStart),
                NodeKind::NonTerminal(NonTerminalKind::TextBindingOpt),
                NodeKind::NonTerminal(NonTerminalKind::Text),
                NodeKind::NonTerminal(NonTerminalKind::TextBindingOpt0),
            ],
            |[text_start, text_binding_opt, text, text_binding_opt_0], visit_ignored| Ok(
                visit(
                    TextBindingView {
                        text_start: TextStartHandle(text_start),
                        text_binding_opt: TextBindingOptHandle(text_binding_opt),
                        text: TextHandle(text),
                        text_binding_opt_0: TextBindingOpt0Handle(text_binding_opt_0),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextBindingView {
    pub text_start: TextStartHandle,
    pub text_binding_opt: TextBindingOptHandle,
    pub text: TextHandle,
    pub text_binding_opt_0: TextBindingOpt0Handle,
}
impl TextBindingView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextBindingOptHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for TextBindingOptHandle {
    type View = Option<WsHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::TextBindingOpt)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::TextBindingOpt
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(
            visit(
                    Some(WsHandle::new_with_visit(self.0, tree, visit_ignored)?),
                    visit_ignored,
                )
                .0,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextBindingOpt0Handle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for TextBindingOpt0Handle {
    type View = Option<GrammarNewlineHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::TextBindingOpt0)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::TextBindingOpt0
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(
            visit(
                    Some(
                        GrammarNewlineHandle::new_with_visit(
                            self.0,
                            tree,
                            visit_ignored,
                        )?,
                    ),
                    visit_ignored,
                )
                .0,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextStartHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for TextStartHandle {
    type View = TextStartView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::TextStart)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::TextStart
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::TextStart)],
            |[text_start], visit_ignored| Ok(
                visit(
                    TextStartView {
                        text_start: TextStart(text_start),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextStartView {
    pub text_start: TextStart,
}
impl TextStartView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrueHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for TrueHandle {
    type View = TrueView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::True)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::True
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::True)],
            |[r#true], visit_ignored| Ok(
                visit(TrueView { r#true: True(r#true) }, visit_ignored),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrueView {
    pub r#true: True,
}
impl TrueView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TupleHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for TupleHandle {
    type View = TupleView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Tuple)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Tuple
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::LParen),
                NodeKind::NonTerminal(NonTerminalKind::TupleOpt),
                NodeKind::NonTerminal(NonTerminalKind::RParen),
            ],
            |[l_paren, tuple_opt, r_paren], visit_ignored| Ok(
                visit(
                    TupleView {
                        l_paren: LParenHandle(l_paren),
                        tuple_opt: TupleOptHandle(tuple_opt),
                        r_paren: RParenHandle(r_paren),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TupleView {
    pub l_paren: LParenHandle,
    pub tuple_opt: TupleOptHandle,
    pub r_paren: RParenHandle,
}
impl TupleView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TupleElementsHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for TupleElementsHandle {
    type View = TupleElementsView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::TupleElements)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::TupleElements
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Value),
                NodeKind::NonTerminal(NonTerminalKind::TupleElementsOpt),
            ],
            |[value, tuple_elements_opt], visit_ignored| Ok(
                visit(
                    TupleElementsView {
                        value: ValueHandle(value),
                        tuple_elements_opt: TupleElementsOptHandle(tuple_elements_opt),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TupleElementsView {
    pub value: ValueHandle,
    pub tuple_elements_opt: TupleElementsOptHandle,
}
impl TupleElementsView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TupleElementsOptHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for TupleElementsOptHandle {
    type View = Option<TupleElementsTailHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::TupleElementsOpt)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::TupleElementsOpt
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(
            visit(
                    Some(
                        TupleElementsTailHandle::new_with_visit(
                            self.0,
                            tree,
                            visit_ignored,
                        )?,
                    ),
                    visit_ignored,
                )
                .0,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TupleElementsTailHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for TupleElementsTailHandle {
    type View = TupleElementsTailView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::TupleElementsTail)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::TupleElementsTail
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Comma),
                NodeKind::NonTerminal(NonTerminalKind::TupleElementsTailOpt),
            ],
            |[comma, tuple_elements_tail_opt], visit_ignored| Ok(
                visit(
                    TupleElementsTailView {
                        comma: CommaHandle(comma),
                        tuple_elements_tail_opt: TupleElementsTailOptHandle(
                            tuple_elements_tail_opt,
                        ),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TupleElementsTailView {
    pub comma: CommaHandle,
    pub tuple_elements_tail_opt: TupleElementsTailOptHandle,
}
impl TupleElementsTailView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TupleElementsTailOptHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for TupleElementsTailOptHandle {
    type View = Option<TupleElementsHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::TupleElementsTailOpt)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::TupleElementsTailOpt
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(
            visit(
                    Some(
                        TupleElementsHandle::new_with_visit(self.0, tree, visit_ignored)?,
                    ),
                    visit_ignored,
                )
                .0,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TupleOptHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for TupleOptHandle {
    type View = Option<TupleElementsHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::TupleOpt)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::TupleOpt
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(
            visit(
                    Some(
                        TupleElementsHandle::new_with_visit(self.0, tree, visit_ignored)?,
                    ),
                    visit_ignored,
                )
                .0,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ValueHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for ValueHandle {
    type View = ValueView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Value)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Value
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        let mut children = tree.children(self.0);
        let Some(child) = children.next() else {
            return Err(ViewConstructionError::UnexpectedEndOfChildren {
                parent: self.0,
            });
        };
        let Some(child_data) = tree.node_data(child) else {
            return Err(ViewConstructionError::NodeIdNotFound {
                node: child,
            });
        };
        let variant = match child_data.node_kind() {
            NodeKind::NonTerminal(NonTerminalKind::Object) => {
                ValueView::Object(ObjectHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::Array) => {
                ValueView::Array(ArrayHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::Tuple) => {
                ValueView::Tuple(TupleHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::Integer) => {
                ValueView::Integer(IntegerHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::Boolean) => {
                ValueView::Boolean(BooleanHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::Null) => {
                ValueView::Null(NullHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::Strings) => {
                ValueView::Strings(StringsHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::Hole) => {
                ValueView::Hole(HoleHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::CodeBlock) => {
                ValueView::CodeBlock(CodeBlockHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::NamedCode) => {
                ValueView::NamedCode(NamedCodeHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::Code) => {
                ValueView::Code(CodeHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::Path) => {
                ValueView::Path(PathHandle(child))
            }
            _ => {
                return Err(ViewConstructionError::UnexpectedNode {
                    node: child,
                    data: child_data,
                    expected_kind: child_data.node_kind(),
                });
            }
        };
        let (result, _visit) = visit(variant, visit_ignored);
        if let Some(child) = children.next() {
            return Err(ViewConstructionError::UnexpectedExtraNode {
                node: child,
            });
        }
        Ok(result)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueView {
    Object(ObjectHandle),
    Array(ArrayHandle),
    Tuple(TupleHandle),
    Integer(IntegerHandle),
    Boolean(BooleanHandle),
    Null(NullHandle),
    Strings(StringsHandle),
    Hole(HoleHandle),
    CodeBlock(CodeBlockHandle),
    NamedCode(NamedCodeHandle),
    Code(CodeHandle),
    Path(PathHandle),
}
impl ValueView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ValueBindingHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for ValueBindingHandle {
    type View = ValueBindingView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::ValueBinding)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::ValueBinding
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Bind),
                NodeKind::NonTerminal(NonTerminalKind::Value),
            ],
            |[bind, value], visit_ignored| Ok(
                visit(
                    ValueBindingView {
                        bind: BindHandle(bind),
                        value: ValueHandle(value),
                    },
                    visit_ignored,
                ),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueBindingView {
    pub bind: BindHandle,
    pub value: ValueHandle,
}
impl ValueBindingView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WsHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for WsHandle {
    type View = WsView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Ws)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Ws
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Ws)],
            |[ws], visit_ignored| Ok(visit(WsView { ws: Ws(ws) }, visit_ignored)),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WsView {
    pub ws: Ws,
}
impl WsView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RootHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle for RootHandle {
    type View = RootView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Root)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Root
    }
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::NonTerminal(NonTerminalKind::Eure)],
            |[eure], visit_ignored| Ok(
                visit(RootView { eure: EureHandle(eure) }, visit_ignored),
            ),
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RootView {
    pub eure: EureHandle,
}
impl RootView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NewLine(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for NewLine {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::NewLine
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Whitespace(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for Whitespace {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Whitespace
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LineComment(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for LineComment {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::LineComment
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockComment(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for BlockComment {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::BlockComment
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Integer(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for Integer {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Integer
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct True(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for True {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::True
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct False(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for False {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::False
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Null(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for Null {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Null
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hole(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for Hole {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Hole
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Str(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for Str {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Str
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Text(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for Text {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Text
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CodeBlock(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for CodeBlock {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::CodeBlock
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NamedCode(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for NamedCode {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::NamedCode
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Code(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for Code {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Code
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GrammarNewline(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for GrammarNewline {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::GrammarNewline
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ws(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for Ws {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Ws
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct At(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for At {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::At
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DollarDollar(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for DollarDollar {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::DollarDollar
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Dollar(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for Dollar {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Dollar
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Dot(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for Dot {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Dot
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LBrace(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for LBrace {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::LBrace
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RBrace(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for RBrace {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::RBrace
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LBracket(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for LBracket {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::LBracket
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RBracket(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for RBracket {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::RBracket
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LParen(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for LParen {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::LParen
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RParen(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for RParen {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::RParen
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bind(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for Bind {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Bind
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Comma(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for Comma {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Comma
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Esc(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for Esc {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Esc
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextStart(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for TextStart {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::TextStart
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ident(pub(crate) super::tree::CstNodeId);
impl TerminalHandle for Ident {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Ident
    }
}
