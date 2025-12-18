use crate::node_kind::{NonTerminalKind, TerminalKind};
#[allow(unused_imports)]
use parol_walker::{
    BuiltinTerminalKind, BuiltinTerminalVisitor, CstConstructError, CstFacade, CstNodeId, NodeKind,
    NonTerminalHandle, RecursiveView, TerminalHandle, ViewConstructionError,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArrayHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for ArrayHandle {
    type View = ArrayView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::ArrayBegin),
                NodeKind::NonTerminal(NonTerminalKind::ArrayOpt),
                NodeKind::NonTerminal(NonTerminalKind::ArrayEnd),
            ],
            |[array_begin, array_opt, array_end], visit_ignored| {
                Ok(visit(
                    ArrayView {
                        array_begin: ArrayBeginHandle(array_begin),
                        array_opt: ArrayOptHandle(array_opt),
                        array_end: ArrayEndHandle(array_end),
                    },
                    visit_ignored,
                ))
            },
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for ArrayBeginHandle {
    type View = ArrayBeginView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::LBracket)],
            |[l_bracket], visit_ignored| {
                Ok(visit(
                    ArrayBeginView {
                        l_bracket: LBracket(l_bracket),
                    },
                    visit_ignored,
                ))
            },
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for ArrayElementsHandle {
    type View = ArrayElementsView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Value),
                NodeKind::NonTerminal(NonTerminalKind::ArrayElementsOpt),
            ],
            |[value, array_elements_opt], visit_ignored| {
                Ok(visit(
                    ArrayElementsView {
                        value: ValueHandle(value),
                        array_elements_opt: ArrayElementsOptHandle(array_elements_opt),
                    },
                    visit_ignored,
                ))
            },
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for ArrayElementsOptHandle {
    type View = Option<ArrayElementsTailHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(visit(
            Some(ArrayElementsTailHandle::new_with_visit(
                self.0,
                tree,
                visit_ignored,
            )?),
            visit_ignored,
        )
        .0)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArrayElementsTailHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for ArrayElementsTailHandle {
    type View = ArrayElementsTailView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Comma),
                NodeKind::NonTerminal(NonTerminalKind::ArrayElementsTailOpt),
            ],
            |[comma, array_elements_tail_opt], visit_ignored| {
                Ok(visit(
                    ArrayElementsTailView {
                        comma: CommaHandle(comma),
                        array_elements_tail_opt: ArrayElementsTailOptHandle(
                            array_elements_tail_opt,
                        ),
                    },
                    visit_ignored,
                ))
            },
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for ArrayElementsTailOptHandle {
    type View = Option<ArrayElementsHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(visit(
            Some(ArrayElementsHandle::new_with_visit(
                self.0,
                tree,
                visit_ignored,
            )?),
            visit_ignored,
        )
        .0)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArrayEndHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for ArrayEndHandle {
    type View = ArrayEndView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::RBracket)],
            |[r_bracket], visit_ignored| {
                Ok(visit(
                    ArrayEndView {
                        r_bracket: RBracket(r_bracket),
                    },
                    visit_ignored,
                ))
            },
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for ArrayMarkerHandle {
    type View = ArrayMarkerView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::ArrayBegin),
                NodeKind::NonTerminal(NonTerminalKind::ArrayMarkerOpt),
                NodeKind::NonTerminal(NonTerminalKind::ArrayEnd),
            ],
            |[array_begin, array_marker_opt, array_end], visit_ignored| {
                Ok(visit(
                    ArrayMarkerView {
                        array_begin: ArrayBeginHandle(array_begin),
                        array_marker_opt: ArrayMarkerOptHandle(array_marker_opt),
                        array_end: ArrayEndHandle(array_end),
                    },
                    visit_ignored,
                ))
            },
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for ArrayMarkerOptHandle {
    type View = Option<IntegerHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(visit(
            Some(IntegerHandle::new_with_visit(self.0, tree, visit_ignored)?),
            visit_ignored,
        )
        .0)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArrayOptHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for ArrayOptHandle {
    type View = Option<ArrayElementsHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(visit(
            Some(ArrayElementsHandle::new_with_visit(
                self.0,
                tree,
                visit_ignored,
            )?),
            visit_ignored,
        )
        .0)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AtHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for AtHandle {
    type View = AtView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
pub struct Backtick1Handle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for Backtick1Handle {
    type View = Backtick1View;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Backtick1)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Backtick1
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Backtick1)],
            |[backtick_1], visit_ignored| {
                Ok(visit(
                    Backtick1View {
                        backtick_1: Backtick1(backtick_1),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Backtick1View {
    pub backtick_1: Backtick1,
}
impl Backtick1View {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Backtick2Handle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for Backtick2Handle {
    type View = Backtick2View;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Backtick2)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Backtick2
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Backtick2)],
            |[backtick_2], visit_ignored| {
                Ok(visit(
                    Backtick2View {
                        backtick_2: Backtick2(backtick_2),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Backtick2View {
    pub backtick_2: Backtick2,
}
impl Backtick2View {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Backtick3Handle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for Backtick3Handle {
    type View = Backtick3View;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Backtick3)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Backtick3
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Backtick3)],
            |[backtick_3], visit_ignored| {
                Ok(visit(
                    Backtick3View {
                        backtick_3: Backtick3(backtick_3),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Backtick3View {
    pub backtick_3: Backtick3,
}
impl Backtick3View {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Backtick4Handle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for Backtick4Handle {
    type View = Backtick4View;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Backtick4)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Backtick4
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Backtick4)],
            |[backtick_4], visit_ignored| {
                Ok(visit(
                    Backtick4View {
                        backtick_4: Backtick4(backtick_4),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Backtick4View {
    pub backtick_4: Backtick4,
}
impl Backtick4View {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Backtick5Handle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for Backtick5Handle {
    type View = Backtick5View;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Backtick5)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Backtick5
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Backtick5)],
            |[backtick_5], visit_ignored| {
                Ok(visit(
                    Backtick5View {
                        backtick_5: Backtick5(backtick_5),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Backtick5View {
    pub backtick_5: Backtick5,
}
impl Backtick5View {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BeginHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for BeginHandle {
    type View = BeginView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::LBrace)],
            |[l_brace], visit_ignored| {
                Ok(visit(
                    BeginView {
                        l_brace: LBrace(l_brace),
                    },
                    visit_ignored,
                ))
            },
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for BindHandle {
    type View = BindView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Bind)],
            |[bind], visit_ignored| Ok(visit(BindView { bind: Bind(bind) }, visit_ignored)),
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for BindingHandle {
    type View = BindingView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Keys),
                NodeKind::NonTerminal(NonTerminalKind::BindingRhs),
            ],
            |[keys, binding_rhs], visit_ignored| {
                Ok(visit(
                    BindingView {
                        keys: KeysHandle(keys),
                        binding_rhs: BindingRhsHandle(binding_rhs),
                    },
                    visit_ignored,
                ))
            },
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for BindingRhsHandle {
    type View = BindingRhsView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        let mut children = tree.children(self.0);
        let Some(child) = children.next() else {
            return Err(ViewConstructionError::UnexpectedEndOfChildren { parent: self.0 }.into());
        };
        let Some(child_data) = tree.node_data(child) else {
            return Err(ViewConstructionError::NodeIdNotFound { node: child }.into());
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
                return Err(ViewConstructionError::UnexpectedNodeData {
                    node: child,
                    data: child_data,
                }
                .into());
            }
        };
        let (result, _visit) = visit(variant, visit_ignored);
        if let Some(extra_child) = children.next() {
            return Err(ViewConstructionError::UnexpectedExtraNode { node: extra_child }.into());
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for BooleanHandle {
    type View = BooleanView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        let mut children = tree.children(self.0);
        let Some(child) = children.next() else {
            return Err(ViewConstructionError::UnexpectedEndOfChildren { parent: self.0 }.into());
        };
        let Some(child_data) = tree.node_data(child) else {
            return Err(ViewConstructionError::NodeIdNotFound { node: child }.into());
        };
        let variant = match child_data.node_kind() {
            NodeKind::NonTerminal(NonTerminalKind::True) => BooleanView::True(TrueHandle(child)),
            NodeKind::NonTerminal(NonTerminalKind::False) => BooleanView::False(FalseHandle(child)),
            _ => {
                return Err(ViewConstructionError::UnexpectedNodeData {
                    node: child,
                    data: child_data,
                }
                .into());
            }
        };
        let (result, _visit) = visit(variant, visit_ignored);
        if let Some(extra_child) = children.next() {
            return Err(ViewConstructionError::UnexpectedExtraNode { node: extra_child }.into());
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
pub struct CodeBlockHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for CodeBlockHandle {
    type View = CodeBlockView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        let mut children = tree.children(self.0);
        let Some(child) = children.next() else {
            return Err(ViewConstructionError::UnexpectedEndOfChildren { parent: self.0 }.into());
        };
        let Some(child_data) = tree.node_data(child) else {
            return Err(ViewConstructionError::NodeIdNotFound { node: child }.into());
        };
        let variant = match child_data.node_kind() {
            NodeKind::NonTerminal(NonTerminalKind::CodeBlock3) => {
                CodeBlockView::CodeBlock3(CodeBlock3Handle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::CodeBlock4) => {
                CodeBlockView::CodeBlock4(CodeBlock4Handle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::CodeBlock5) => {
                CodeBlockView::CodeBlock5(CodeBlock5Handle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::CodeBlock6) => {
                CodeBlockView::CodeBlock6(CodeBlock6Handle(child))
            }
            _ => {
                return Err(ViewConstructionError::UnexpectedNodeData {
                    node: child,
                    data: child_data,
                }
                .into());
            }
        };
        let (result, _visit) = visit(variant, visit_ignored);
        if let Some(extra_child) = children.next() {
            return Err(ViewConstructionError::UnexpectedExtraNode { node: extra_child }.into());
        }
        Ok(result)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeBlockView {
    CodeBlock3(CodeBlock3Handle),
    CodeBlock4(CodeBlock4Handle),
    CodeBlock5(CodeBlock5Handle),
    CodeBlock6(CodeBlock6Handle),
}
impl CodeBlockView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodeBlock3Handle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for CodeBlock3Handle {
    type View = CodeBlock3View;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::CodeBlock3)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::CodeBlock3
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::CodeBlockStart3),
                NodeKind::NonTerminal(NonTerminalKind::CodeBlock3List),
                NodeKind::NonTerminal(NonTerminalKind::CodeBlockEnd3),
            ],
            |[code_block_start_3, code_block_3_list, code_block_end_3], visit_ignored| {
                Ok(visit(
                    CodeBlock3View {
                        code_block_start_3: CodeBlockStart3Handle(code_block_start_3),
                        code_block_3_list: CodeBlock3ListHandle(code_block_3_list),
                        code_block_end_3: CodeBlockEnd3Handle(code_block_end_3),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeBlock3View {
    pub code_block_start_3: CodeBlockStart3Handle,
    pub code_block_3_list: CodeBlock3ListHandle,
    pub code_block_end_3: CodeBlockEnd3Handle,
}
impl CodeBlock3View {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodeBlock3ListHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for CodeBlock3ListHandle {
    type View = Option<CodeBlock3ListView>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::CodeBlock3List)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::CodeBlock3List
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::CodeBlock3ListGroup),
                NodeKind::NonTerminal(NonTerminalKind::CodeBlock3List),
            ],
            |[code_block_3_list_group, code_block_3_list], visit_ignored| {
                Ok(visit(
                    Some(CodeBlock3ListView {
                        code_block_3_list_group: CodeBlock3ListGroupHandle(code_block_3_list_group),
                        code_block_3_list: CodeBlock3ListHandle(code_block_3_list),
                    }),
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeBlock3ListView {
    pub code_block_3_list_group: CodeBlock3ListGroupHandle,
    pub code_block_3_list: CodeBlock3ListHandle,
}
impl<F: CstFacade<TerminalKind, NonTerminalKind>> RecursiveView<TerminalKind, NonTerminalKind, F>
    for CodeBlock3ListView
{
    type Item = CodeBlock3ListGroupHandle;
    fn get_all_with_visit<E>(
        &self,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Vec<Self::Item>, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        let mut items = Vec::new();
        let mut current_view = Some(*self);
        while let Some(item) = current_view {
            let Self {
                code_block_3_list_group,
                ..
            } = item;
            items.push(code_block_3_list_group);
            item.code_block_3_list.get_view_with_visit(
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
pub struct CodeBlock3ListGroupHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for CodeBlock3ListGroupHandle {
    type View = CodeBlock3ListGroupView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::CodeBlock3ListGroup)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::CodeBlock3ListGroup
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        let mut children = tree.children(self.0);
        let Some(child) = children.next() else {
            return Err(ViewConstructionError::UnexpectedEndOfChildren { parent: self.0 }.into());
        };
        let Some(child_data) = tree.node_data(child) else {
            return Err(ViewConstructionError::NodeIdNotFound { node: child }.into());
        };
        let variant = match child_data.node_kind() {
            NodeKind::NonTerminal(NonTerminalKind::NoBacktick) => {
                CodeBlock3ListGroupView::NoBacktick(NoBacktickHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::Backtick2) => {
                CodeBlock3ListGroupView::Backtick2(Backtick2Handle(child))
            }
            _ => {
                return Err(ViewConstructionError::UnexpectedNodeData {
                    node: child,
                    data: child_data,
                }
                .into());
            }
        };
        let (result, _visit) = visit(variant, visit_ignored);
        if let Some(extra_child) = children.next() {
            return Err(ViewConstructionError::UnexpectedExtraNode { node: extra_child }.into());
        }
        Ok(result)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeBlock3ListGroupView {
    NoBacktick(NoBacktickHandle),
    Backtick2(Backtick2Handle),
}
impl CodeBlock3ListGroupView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodeBlock4Handle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for CodeBlock4Handle {
    type View = CodeBlock4View;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::CodeBlock4)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::CodeBlock4
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::CodeBlockStart4),
                NodeKind::NonTerminal(NonTerminalKind::CodeBlock4List),
                NodeKind::NonTerminal(NonTerminalKind::CodeBlockEnd4),
            ],
            |[code_block_start_4, code_block_4_list, code_block_end_4], visit_ignored| {
                Ok(visit(
                    CodeBlock4View {
                        code_block_start_4: CodeBlockStart4Handle(code_block_start_4),
                        code_block_4_list: CodeBlock4ListHandle(code_block_4_list),
                        code_block_end_4: CodeBlockEnd4Handle(code_block_end_4),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeBlock4View {
    pub code_block_start_4: CodeBlockStart4Handle,
    pub code_block_4_list: CodeBlock4ListHandle,
    pub code_block_end_4: CodeBlockEnd4Handle,
}
impl CodeBlock4View {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodeBlock4ListHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for CodeBlock4ListHandle {
    type View = Option<CodeBlock4ListView>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::CodeBlock4List)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::CodeBlock4List
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::CodeBlock4ListGroup),
                NodeKind::NonTerminal(NonTerminalKind::CodeBlock4List),
            ],
            |[code_block_4_list_group, code_block_4_list], visit_ignored| {
                Ok(visit(
                    Some(CodeBlock4ListView {
                        code_block_4_list_group: CodeBlock4ListGroupHandle(code_block_4_list_group),
                        code_block_4_list: CodeBlock4ListHandle(code_block_4_list),
                    }),
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeBlock4ListView {
    pub code_block_4_list_group: CodeBlock4ListGroupHandle,
    pub code_block_4_list: CodeBlock4ListHandle,
}
impl<F: CstFacade<TerminalKind, NonTerminalKind>> RecursiveView<TerminalKind, NonTerminalKind, F>
    for CodeBlock4ListView
{
    type Item = CodeBlock4ListGroupHandle;
    fn get_all_with_visit<E>(
        &self,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Vec<Self::Item>, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        let mut items = Vec::new();
        let mut current_view = Some(*self);
        while let Some(item) = current_view {
            let Self {
                code_block_4_list_group,
                ..
            } = item;
            items.push(code_block_4_list_group);
            item.code_block_4_list.get_view_with_visit(
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
pub struct CodeBlock4ListGroupHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for CodeBlock4ListGroupHandle {
    type View = CodeBlock4ListGroupView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::CodeBlock4ListGroup)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::CodeBlock4ListGroup
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        let mut children = tree.children(self.0);
        let Some(child) = children.next() else {
            return Err(ViewConstructionError::UnexpectedEndOfChildren { parent: self.0 }.into());
        };
        let Some(child_data) = tree.node_data(child) else {
            return Err(ViewConstructionError::NodeIdNotFound { node: child }.into());
        };
        let variant = match child_data.node_kind() {
            NodeKind::NonTerminal(NonTerminalKind::NoBacktick) => {
                CodeBlock4ListGroupView::NoBacktick(NoBacktickHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::Backtick3) => {
                CodeBlock4ListGroupView::Backtick3(Backtick3Handle(child))
            }
            _ => {
                return Err(ViewConstructionError::UnexpectedNodeData {
                    node: child,
                    data: child_data,
                }
                .into());
            }
        };
        let (result, _visit) = visit(variant, visit_ignored);
        if let Some(extra_child) = children.next() {
            return Err(ViewConstructionError::UnexpectedExtraNode { node: extra_child }.into());
        }
        Ok(result)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeBlock4ListGroupView {
    NoBacktick(NoBacktickHandle),
    Backtick3(Backtick3Handle),
}
impl CodeBlock4ListGroupView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodeBlock5Handle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for CodeBlock5Handle {
    type View = CodeBlock5View;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::CodeBlock5)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::CodeBlock5
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::CodeBlockStart5),
                NodeKind::NonTerminal(NonTerminalKind::CodeBlock5List),
                NodeKind::NonTerminal(NonTerminalKind::CodeBlockEnd5),
            ],
            |[code_block_start_5, code_block_5_list, code_block_end_5], visit_ignored| {
                Ok(visit(
                    CodeBlock5View {
                        code_block_start_5: CodeBlockStart5Handle(code_block_start_5),
                        code_block_5_list: CodeBlock5ListHandle(code_block_5_list),
                        code_block_end_5: CodeBlockEnd5Handle(code_block_end_5),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeBlock5View {
    pub code_block_start_5: CodeBlockStart5Handle,
    pub code_block_5_list: CodeBlock5ListHandle,
    pub code_block_end_5: CodeBlockEnd5Handle,
}
impl CodeBlock5View {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodeBlock5ListHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for CodeBlock5ListHandle {
    type View = Option<CodeBlock5ListView>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::CodeBlock5List)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::CodeBlock5List
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::CodeBlock5ListGroup),
                NodeKind::NonTerminal(NonTerminalKind::CodeBlock5List),
            ],
            |[code_block_5_list_group, code_block_5_list], visit_ignored| {
                Ok(visit(
                    Some(CodeBlock5ListView {
                        code_block_5_list_group: CodeBlock5ListGroupHandle(code_block_5_list_group),
                        code_block_5_list: CodeBlock5ListHandle(code_block_5_list),
                    }),
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeBlock5ListView {
    pub code_block_5_list_group: CodeBlock5ListGroupHandle,
    pub code_block_5_list: CodeBlock5ListHandle,
}
impl<F: CstFacade<TerminalKind, NonTerminalKind>> RecursiveView<TerminalKind, NonTerminalKind, F>
    for CodeBlock5ListView
{
    type Item = CodeBlock5ListGroupHandle;
    fn get_all_with_visit<E>(
        &self,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Vec<Self::Item>, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        let mut items = Vec::new();
        let mut current_view = Some(*self);
        while let Some(item) = current_view {
            let Self {
                code_block_5_list_group,
                ..
            } = item;
            items.push(code_block_5_list_group);
            item.code_block_5_list.get_view_with_visit(
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
pub struct CodeBlock5ListGroupHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for CodeBlock5ListGroupHandle {
    type View = CodeBlock5ListGroupView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::CodeBlock5ListGroup)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::CodeBlock5ListGroup
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        let mut children = tree.children(self.0);
        let Some(child) = children.next() else {
            return Err(ViewConstructionError::UnexpectedEndOfChildren { parent: self.0 }.into());
        };
        let Some(child_data) = tree.node_data(child) else {
            return Err(ViewConstructionError::NodeIdNotFound { node: child }.into());
        };
        let variant = match child_data.node_kind() {
            NodeKind::NonTerminal(NonTerminalKind::NoBacktick) => {
                CodeBlock5ListGroupView::NoBacktick(NoBacktickHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::Backtick4) => {
                CodeBlock5ListGroupView::Backtick4(Backtick4Handle(child))
            }
            _ => {
                return Err(ViewConstructionError::UnexpectedNodeData {
                    node: child,
                    data: child_data,
                }
                .into());
            }
        };
        let (result, _visit) = visit(variant, visit_ignored);
        if let Some(extra_child) = children.next() {
            return Err(ViewConstructionError::UnexpectedExtraNode { node: extra_child }.into());
        }
        Ok(result)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeBlock5ListGroupView {
    NoBacktick(NoBacktickHandle),
    Backtick4(Backtick4Handle),
}
impl CodeBlock5ListGroupView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodeBlock6Handle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for CodeBlock6Handle {
    type View = CodeBlock6View;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::CodeBlock6)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::CodeBlock6
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::CodeBlockStart6),
                NodeKind::NonTerminal(NonTerminalKind::CodeBlock6List),
                NodeKind::NonTerminal(NonTerminalKind::CodeBlockEnd6),
            ],
            |[code_block_start_6, code_block_6_list, code_block_end_6], visit_ignored| {
                Ok(visit(
                    CodeBlock6View {
                        code_block_start_6: CodeBlockStart6Handle(code_block_start_6),
                        code_block_6_list: CodeBlock6ListHandle(code_block_6_list),
                        code_block_end_6: CodeBlockEnd6Handle(code_block_end_6),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeBlock6View {
    pub code_block_start_6: CodeBlockStart6Handle,
    pub code_block_6_list: CodeBlock6ListHandle,
    pub code_block_end_6: CodeBlockEnd6Handle,
}
impl CodeBlock6View {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodeBlock6ListHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for CodeBlock6ListHandle {
    type View = Option<CodeBlock6ListView>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::CodeBlock6List)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::CodeBlock6List
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::CodeBlock6ListGroup),
                NodeKind::NonTerminal(NonTerminalKind::CodeBlock6List),
            ],
            |[code_block_6_list_group, code_block_6_list], visit_ignored| {
                Ok(visit(
                    Some(CodeBlock6ListView {
                        code_block_6_list_group: CodeBlock6ListGroupHandle(code_block_6_list_group),
                        code_block_6_list: CodeBlock6ListHandle(code_block_6_list),
                    }),
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeBlock6ListView {
    pub code_block_6_list_group: CodeBlock6ListGroupHandle,
    pub code_block_6_list: CodeBlock6ListHandle,
}
impl<F: CstFacade<TerminalKind, NonTerminalKind>> RecursiveView<TerminalKind, NonTerminalKind, F>
    for CodeBlock6ListView
{
    type Item = CodeBlock6ListGroupHandle;
    fn get_all_with_visit<E>(
        &self,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Vec<Self::Item>, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        let mut items = Vec::new();
        let mut current_view = Some(*self);
        while let Some(item) = current_view {
            let Self {
                code_block_6_list_group,
                ..
            } = item;
            items.push(code_block_6_list_group);
            item.code_block_6_list.get_view_with_visit(
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
pub struct CodeBlock6ListGroupHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for CodeBlock6ListGroupHandle {
    type View = CodeBlock6ListGroupView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::CodeBlock6ListGroup)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::CodeBlock6ListGroup
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        let mut children = tree.children(self.0);
        let Some(child) = children.next() else {
            return Err(ViewConstructionError::UnexpectedEndOfChildren { parent: self.0 }.into());
        };
        let Some(child_data) = tree.node_data(child) else {
            return Err(ViewConstructionError::NodeIdNotFound { node: child }.into());
        };
        let variant = match child_data.node_kind() {
            NodeKind::NonTerminal(NonTerminalKind::NoBacktick) => {
                CodeBlock6ListGroupView::NoBacktick(NoBacktickHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::Backtick5) => {
                CodeBlock6ListGroupView::Backtick5(Backtick5Handle(child))
            }
            _ => {
                return Err(ViewConstructionError::UnexpectedNodeData {
                    node: child,
                    data: child_data,
                }
                .into());
            }
        };
        let (result, _visit) = visit(variant, visit_ignored);
        if let Some(extra_child) = children.next() {
            return Err(ViewConstructionError::UnexpectedExtraNode { node: extra_child }.into());
        }
        Ok(result)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeBlock6ListGroupView {
    NoBacktick(NoBacktickHandle),
    Backtick5(Backtick5Handle),
}
impl CodeBlock6ListGroupView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodeBlockEnd3Handle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for CodeBlockEnd3Handle {
    type View = CodeBlockEnd3View;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::CodeBlockEnd3)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::CodeBlockEnd3
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::CodeBlockEnd3)],
            |[code_block_end_3], visit_ignored| {
                Ok(visit(
                    CodeBlockEnd3View {
                        code_block_end_3: CodeBlockEnd3(code_block_end_3),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeBlockEnd3View {
    pub code_block_end_3: CodeBlockEnd3,
}
impl CodeBlockEnd3View {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodeBlockEnd4Handle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for CodeBlockEnd4Handle {
    type View = CodeBlockEnd4View;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::CodeBlockEnd4)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::CodeBlockEnd4
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::CodeBlockEnd4)],
            |[code_block_end_4], visit_ignored| {
                Ok(visit(
                    CodeBlockEnd4View {
                        code_block_end_4: CodeBlockEnd4(code_block_end_4),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeBlockEnd4View {
    pub code_block_end_4: CodeBlockEnd4,
}
impl CodeBlockEnd4View {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodeBlockEnd5Handle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for CodeBlockEnd5Handle {
    type View = CodeBlockEnd5View;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::CodeBlockEnd5)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::CodeBlockEnd5
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::CodeBlockEnd5)],
            |[code_block_end_5], visit_ignored| {
                Ok(visit(
                    CodeBlockEnd5View {
                        code_block_end_5: CodeBlockEnd5(code_block_end_5),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeBlockEnd5View {
    pub code_block_end_5: CodeBlockEnd5,
}
impl CodeBlockEnd5View {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodeBlockEnd6Handle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for CodeBlockEnd6Handle {
    type View = CodeBlockEnd6View;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::CodeBlockEnd6)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::CodeBlockEnd6
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::CodeBlockEnd6)],
            |[code_block_end_6], visit_ignored| {
                Ok(visit(
                    CodeBlockEnd6View {
                        code_block_end_6: CodeBlockEnd6(code_block_end_6),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeBlockEnd6View {
    pub code_block_end_6: CodeBlockEnd6,
}
impl CodeBlockEnd6View {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodeBlockStart3Handle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for CodeBlockStart3Handle {
    type View = CodeBlockStart3View;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::CodeBlockStart3)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::CodeBlockStart3
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::CodeBlockStart3)],
            |[code_block_start_3], visit_ignored| {
                Ok(visit(
                    CodeBlockStart3View {
                        code_block_start_3: CodeBlockStart3(code_block_start_3),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeBlockStart3View {
    pub code_block_start_3: CodeBlockStart3,
}
impl CodeBlockStart3View {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodeBlockStart4Handle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for CodeBlockStart4Handle {
    type View = CodeBlockStart4View;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::CodeBlockStart4)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::CodeBlockStart4
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::CodeBlockStart4)],
            |[code_block_start_4], visit_ignored| {
                Ok(visit(
                    CodeBlockStart4View {
                        code_block_start_4: CodeBlockStart4(code_block_start_4),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeBlockStart4View {
    pub code_block_start_4: CodeBlockStart4,
}
impl CodeBlockStart4View {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodeBlockStart5Handle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for CodeBlockStart5Handle {
    type View = CodeBlockStart5View;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::CodeBlockStart5)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::CodeBlockStart5
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::CodeBlockStart5)],
            |[code_block_start_5], visit_ignored| {
                Ok(visit(
                    CodeBlockStart5View {
                        code_block_start_5: CodeBlockStart5(code_block_start_5),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeBlockStart5View {
    pub code_block_start_5: CodeBlockStart5,
}
impl CodeBlockStart5View {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodeBlockStart6Handle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for CodeBlockStart6Handle {
    type View = CodeBlockStart6View;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::CodeBlockStart6)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::CodeBlockStart6
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::CodeBlockStart6)],
            |[code_block_start_6], visit_ignored| {
                Ok(visit(
                    CodeBlockStart6View {
                        code_block_start_6: CodeBlockStart6(code_block_start_6),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeBlockStart6View {
    pub code_block_start_6: CodeBlockStart6,
}
impl CodeBlockStart6View {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommaHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for CommaHandle {
    type View = CommaView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Comma)],
            |[comma], visit_ignored| {
                Ok(visit(
                    CommaView {
                        comma: Comma(comma),
                    },
                    visit_ignored,
                ))
            },
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for ContinueHandle {
    type View = ContinueView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Esc)],
            |[esc], visit_ignored| Ok(visit(ContinueView { esc: Esc(esc) }, visit_ignored)),
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
pub struct DotHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for DotHandle {
    type View = DotView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for EndHandle {
    type View = EndView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::RBrace)],
            |[r_brace], visit_ignored| {
                Ok(visit(
                    EndView {
                        r_brace: RBrace(r_brace),
                    },
                    visit_ignored,
                ))
            },
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for EureHandle {
    type View = EureView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::EureOpt),
                NodeKind::NonTerminal(NonTerminalKind::EureList),
                NodeKind::NonTerminal(NonTerminalKind::EureList0),
            ],
            |[eure_opt, eure_bindings, eure_sections], visit_ignored| {
                Ok(visit(
                    EureView {
                        eure_opt: EureOptHandle(eure_opt),
                        eure_bindings: EureBindingsHandle(eure_bindings),
                        eure_sections: EureSectionsHandle(eure_sections),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EureView {
    pub eure_opt: EureOptHandle,
    pub eure_bindings: EureBindingsHandle,
    pub eure_sections: EureSectionsHandle,
}
impl EureView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EureBindingsHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for EureBindingsHandle {
    type View = Option<EureBindingsView>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Binding),
                NodeKind::NonTerminal(NonTerminalKind::EureList),
            ],
            |[binding, eure_bindings], visit_ignored| {
                Ok(visit(
                    Some(EureBindingsView {
                        binding: BindingHandle(binding),
                        eure_bindings: EureBindingsHandle(eure_bindings),
                    }),
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EureBindingsView {
    pub binding: BindingHandle,
    pub eure_bindings: EureBindingsHandle,
}
impl<F: CstFacade<TerminalKind, NonTerminalKind>> RecursiveView<TerminalKind, NonTerminalKind, F>
    for EureBindingsView
{
    type Item = BindingHandle;
    fn get_all_with_visit<E>(
        &self,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Vec<Self::Item>, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        let mut items = Vec::new();
        let mut current_view = Some(*self);
        while let Some(item) = current_view {
            let Self { binding, .. } = item;
            items.push(binding);
            item.eure_bindings.get_view_with_visit(
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for EureSectionsHandle {
    type View = Option<EureSectionsView>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Section),
                NodeKind::NonTerminal(NonTerminalKind::EureList0),
            ],
            |[section, eure_sections], visit_ignored| {
                Ok(visit(
                    Some(EureSectionsView {
                        section: SectionHandle(section),
                        eure_sections: EureSectionsHandle(eure_sections),
                    }),
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EureSectionsView {
    pub section: SectionHandle,
    pub eure_sections: EureSectionsHandle,
}
impl<F: CstFacade<TerminalKind, NonTerminalKind>> RecursiveView<TerminalKind, NonTerminalKind, F>
    for EureSectionsView
{
    type Item = SectionHandle;
    fn get_all_with_visit<E>(
        &self,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Vec<Self::Item>, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        let mut items = Vec::new();
        let mut current_view = Some(*self);
        while let Some(item) = current_view {
            let Self { section, .. } = item;
            items.push(section);
            item.eure_sections.get_view_with_visit(
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
pub struct EureOptHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for EureOptHandle {
    type View = Option<ValueBindingHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::EureOpt)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::EureOpt
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(visit(
            Some(ValueBindingHandle::new_with_visit(
                self.0,
                tree,
                visit_ignored,
            )?),
            visit_ignored,
        )
        .0)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExtHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for ExtHandle {
    type View = ExtView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Dollar)],
            |[dollar], visit_ignored| {
                Ok(visit(
                    ExtView {
                        dollar: Dollar(dollar),
                    },
                    visit_ignored,
                ))
            },
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for ExtensionNameSpaceHandle {
    type View = ExtensionNameSpaceView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Ext),
                NodeKind::NonTerminal(NonTerminalKind::KeyIdent),
            ],
            |[ext, key_ident], visit_ignored| {
                Ok(visit(
                    ExtensionNameSpaceView {
                        ext: ExtHandle(ext),
                        key_ident: KeyIdentHandle(key_ident),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtensionNameSpaceView {
    pub ext: ExtHandle,
    pub key_ident: KeyIdentHandle,
}
impl ExtensionNameSpaceView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FalseHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for FalseHandle {
    type View = FalseView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::False)],
            |[r#false], visit_ignored| {
                Ok(visit(
                    FalseView {
                        r#false: False(r#false),
                    },
                    visit_ignored,
                ))
            },
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
pub struct FloatHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for FloatHandle {
    type View = FloatView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::Float)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::Float
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Float)],
            |[float], visit_ignored| {
                Ok(visit(
                    FloatView {
                        float: Float(float),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatView {
    pub float: Float,
}
impl FloatView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GrammarNewlineHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for GrammarNewlineHandle {
    type View = GrammarNewlineView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::GrammarNewline)],
            |[grammar_newline], visit_ignored| {
                Ok(visit(
                    GrammarNewlineView {
                        grammar_newline: GrammarNewline(grammar_newline),
                    },
                    visit_ignored,
                ))
            },
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for HoleHandle {
    type View = HoleView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Hole)],
            |[hole], visit_ignored| Ok(visit(HoleView { hole: Hole(hole) }, visit_ignored)),
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for IdentHandle {
    type View = IdentView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Ident)],
            |[ident], visit_ignored| {
                Ok(visit(
                    IdentView {
                        ident: Ident(ident),
                    },
                    visit_ignored,
                ))
            },
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
pub struct InlineCodeHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for InlineCodeHandle {
    type View = InlineCodeView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::InlineCode)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::InlineCode
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        let mut children = tree.children(self.0);
        let Some(child) = children.next() else {
            return Err(ViewConstructionError::UnexpectedEndOfChildren { parent: self.0 }.into());
        };
        let Some(child_data) = tree.node_data(child) else {
            return Err(ViewConstructionError::NodeIdNotFound { node: child }.into());
        };
        let variant = match child_data.node_kind() {
            NodeKind::NonTerminal(NonTerminalKind::InlineCode2) => {
                InlineCodeView::InlineCode2(InlineCode2Handle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::InlineCode1) => {
                InlineCodeView::InlineCode1(InlineCode1Handle(child))
            }
            _ => {
                return Err(ViewConstructionError::UnexpectedNodeData {
                    node: child,
                    data: child_data,
                }
                .into());
            }
        };
        let (result, _visit) = visit(variant, visit_ignored);
        if let Some(extra_child) = children.next() {
            return Err(ViewConstructionError::UnexpectedExtraNode { node: extra_child }.into());
        }
        Ok(result)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineCodeView {
    InlineCode2(InlineCode2Handle),
    InlineCode1(InlineCode1Handle),
}
impl InlineCodeView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InlineCode1Handle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for InlineCode1Handle {
    type View = InlineCode1View;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::InlineCode1)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::InlineCode1
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::InlineCode1)],
            |[inline_code_1], visit_ignored| {
                Ok(visit(
                    InlineCode1View {
                        inline_code_1: InlineCode1(inline_code_1),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InlineCode1View {
    pub inline_code_1: InlineCode1,
}
impl InlineCode1View {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InlineCode2Handle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for InlineCode2Handle {
    type View = InlineCode2View;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::InlineCode2)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::InlineCode2
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::InlineCodeStart2),
                NodeKind::NonTerminal(NonTerminalKind::InlineCode2List),
                NodeKind::NonTerminal(NonTerminalKind::InlineCodeEnd2),
            ],
            |[inline_code_start_2, inline_code_2_list, inline_code_end_2], visit_ignored| {
                Ok(visit(
                    InlineCode2View {
                        inline_code_start_2: InlineCodeStart2Handle(inline_code_start_2),
                        inline_code_2_list: InlineCode2ListHandle(inline_code_2_list),
                        inline_code_end_2: InlineCodeEnd2Handle(inline_code_end_2),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InlineCode2View {
    pub inline_code_start_2: InlineCodeStart2Handle,
    pub inline_code_2_list: InlineCode2ListHandle,
    pub inline_code_end_2: InlineCodeEnd2Handle,
}
impl InlineCode2View {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InlineCode2ListHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for InlineCode2ListHandle {
    type View = Option<InlineCode2ListView>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::InlineCode2List)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::InlineCode2List
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::InlineCode2ListGroup),
                NodeKind::NonTerminal(NonTerminalKind::InlineCode2List),
            ],
            |[inline_code_2_list_group, inline_code_2_list], visit_ignored| {
                Ok(visit(
                    Some(InlineCode2ListView {
                        inline_code_2_list_group: InlineCode2ListGroupHandle(
                            inline_code_2_list_group,
                        ),
                        inline_code_2_list: InlineCode2ListHandle(inline_code_2_list),
                    }),
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InlineCode2ListView {
    pub inline_code_2_list_group: InlineCode2ListGroupHandle,
    pub inline_code_2_list: InlineCode2ListHandle,
}
impl<F: CstFacade<TerminalKind, NonTerminalKind>> RecursiveView<TerminalKind, NonTerminalKind, F>
    for InlineCode2ListView
{
    type Item = InlineCode2ListGroupHandle;
    fn get_all_with_visit<E>(
        &self,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Vec<Self::Item>, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        let mut items = Vec::new();
        let mut current_view = Some(*self);
        while let Some(item) = current_view {
            let Self {
                inline_code_2_list_group,
                ..
            } = item;
            items.push(inline_code_2_list_group);
            item.inline_code_2_list.get_view_with_visit(
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
pub struct InlineCode2ListGroupHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for InlineCode2ListGroupHandle {
    type View = InlineCode2ListGroupView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::InlineCode2ListGroup)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::InlineCode2ListGroup
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        let mut children = tree.children(self.0);
        let Some(child) = children.next() else {
            return Err(ViewConstructionError::UnexpectedEndOfChildren { parent: self.0 }.into());
        };
        let Some(child_data) = tree.node_data(child) else {
            return Err(ViewConstructionError::NodeIdNotFound { node: child }.into());
        };
        let variant = match child_data.node_kind() {
            NodeKind::NonTerminal(NonTerminalKind::NoBacktickInline) => {
                InlineCode2ListGroupView::NoBacktickInline(NoBacktickInlineHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::Backtick1) => {
                InlineCode2ListGroupView::Backtick1(Backtick1Handle(child))
            }
            _ => {
                return Err(ViewConstructionError::UnexpectedNodeData {
                    node: child,
                    data: child_data,
                }
                .into());
            }
        };
        let (result, _visit) = visit(variant, visit_ignored);
        if let Some(extra_child) = children.next() {
            return Err(ViewConstructionError::UnexpectedExtraNode { node: extra_child }.into());
        }
        Ok(result)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineCode2ListGroupView {
    NoBacktickInline(NoBacktickInlineHandle),
    Backtick1(Backtick1Handle),
}
impl InlineCode2ListGroupView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InlineCodeEnd2Handle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for InlineCodeEnd2Handle {
    type View = InlineCodeEnd2View;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::InlineCodeEnd2)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::InlineCodeEnd2
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::InlineCodeEnd2)],
            |[inline_code_end_2], visit_ignored| {
                Ok(visit(
                    InlineCodeEnd2View {
                        inline_code_end_2: InlineCodeEnd2(inline_code_end_2),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InlineCodeEnd2View {
    pub inline_code_end_2: InlineCodeEnd2,
}
impl InlineCodeEnd2View {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InlineCodeStart2Handle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for InlineCodeStart2Handle {
    type View = InlineCodeStart2View;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::InlineCodeStart2)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::InlineCodeStart2
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::InlineCodeStart2)],
            |[inline_code_start_2], visit_ignored| {
                Ok(visit(
                    InlineCodeStart2View {
                        inline_code_start_2: InlineCodeStart2(inline_code_start_2),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InlineCodeStart2View {
    pub inline_code_start_2: InlineCodeStart2,
}
impl InlineCodeStart2View {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IntegerHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for IntegerHandle {
    type View = IntegerView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Integer)],
            |[integer], visit_ignored| {
                Ok(visit(
                    IntegerView {
                        integer: Integer(integer),
                    },
                    visit_ignored,
                ))
            },
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for KeyHandle {
    type View = KeyView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::KeyBase),
                NodeKind::NonTerminal(NonTerminalKind::KeyOpt),
            ],
            |[key_base, key_opt], visit_ignored| {
                Ok(visit(
                    KeyView {
                        key_base: KeyBaseHandle(key_base),
                        key_opt: KeyOptHandle(key_opt),
                    },
                    visit_ignored,
                ))
            },
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for KeyBaseHandle {
    type View = KeyBaseView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        let mut children = tree.children(self.0);
        let Some(child) = children.next() else {
            return Err(ViewConstructionError::UnexpectedEndOfChildren { parent: self.0 }.into());
        };
        let Some(child_data) = tree.node_data(child) else {
            return Err(ViewConstructionError::NodeIdNotFound { node: child }.into());
        };
        let variant = match child_data.node_kind() {
            NodeKind::NonTerminal(NonTerminalKind::KeyIdent) => {
                KeyBaseView::KeyIdent(KeyIdentHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::ExtensionNameSpace) => {
                KeyBaseView::ExtensionNameSpace(ExtensionNameSpaceHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::Str) => KeyBaseView::Str(StrHandle(child)),
            NodeKind::NonTerminal(NonTerminalKind::Integer) => {
                KeyBaseView::Integer(IntegerHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::KeyTuple) => {
                KeyBaseView::KeyTuple(KeyTupleHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::TupleIndex) => {
                KeyBaseView::TupleIndex(TupleIndexHandle(child))
            }
            _ => {
                return Err(ViewConstructionError::UnexpectedNodeData {
                    node: child,
                    data: child_data,
                }
                .into());
            }
        };
        let (result, _visit) = visit(variant, visit_ignored);
        if let Some(extra_child) = children.next() {
            return Err(ViewConstructionError::UnexpectedExtraNode { node: extra_child }.into());
        }
        Ok(result)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyBaseView {
    KeyIdent(KeyIdentHandle),
    ExtensionNameSpace(ExtensionNameSpaceHandle),
    Str(StrHandle),
    Integer(IntegerHandle),
    KeyTuple(KeyTupleHandle),
    TupleIndex(TupleIndexHandle),
}
impl KeyBaseView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyIdentHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for KeyIdentHandle {
    type View = KeyIdentView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::KeyIdent)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::KeyIdent
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        let mut children = tree.children(self.0);
        let Some(child) = children.next() else {
            return Err(ViewConstructionError::UnexpectedEndOfChildren { parent: self.0 }.into());
        };
        let Some(child_data) = tree.node_data(child) else {
            return Err(ViewConstructionError::NodeIdNotFound { node: child }.into());
        };
        let variant = match child_data.node_kind() {
            NodeKind::NonTerminal(NonTerminalKind::Ident) => {
                KeyIdentView::Ident(IdentHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::True) => KeyIdentView::True(TrueHandle(child)),
            NodeKind::NonTerminal(NonTerminalKind::False) => {
                KeyIdentView::False(FalseHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::Null) => KeyIdentView::Null(NullHandle(child)),
            _ => {
                return Err(ViewConstructionError::UnexpectedNodeData {
                    node: child,
                    data: child_data,
                }
                .into());
            }
        };
        let (result, _visit) = visit(variant, visit_ignored);
        if let Some(extra_child) = children.next() {
            return Err(ViewConstructionError::UnexpectedExtraNode { node: extra_child }.into());
        }
        Ok(result)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyIdentView {
    Ident(IdentHandle),
    True(TrueHandle),
    False(FalseHandle),
    Null(NullHandle),
}
impl KeyIdentView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyOptHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for KeyOptHandle {
    type View = Option<ArrayMarkerHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(visit(
            Some(ArrayMarkerHandle::new_with_visit(
                self.0,
                tree,
                visit_ignored,
            )?),
            visit_ignored,
        )
        .0)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyTupleHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for KeyTupleHandle {
    type View = KeyTupleView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::KeyTuple)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::KeyTuple
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::LParen),
                NodeKind::NonTerminal(NonTerminalKind::KeyTupleOpt),
                NodeKind::NonTerminal(NonTerminalKind::RParen),
            ],
            |[l_paren, key_tuple_opt, r_paren], visit_ignored| {
                Ok(visit(
                    KeyTupleView {
                        l_paren: LParenHandle(l_paren),
                        key_tuple_opt: KeyTupleOptHandle(key_tuple_opt),
                        r_paren: RParenHandle(r_paren),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyTupleView {
    pub l_paren: LParenHandle,
    pub key_tuple_opt: KeyTupleOptHandle,
    pub r_paren: RParenHandle,
}
impl KeyTupleView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyTupleElementsHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for KeyTupleElementsHandle {
    type View = KeyTupleElementsView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::KeyTupleElements)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::KeyTupleElements
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::KeyValue),
                NodeKind::NonTerminal(NonTerminalKind::KeyTupleElementsOpt),
            ],
            |[key_value, key_tuple_elements_opt], visit_ignored| {
                Ok(visit(
                    KeyTupleElementsView {
                        key_value: KeyValueHandle(key_value),
                        key_tuple_elements_opt: KeyTupleElementsOptHandle(key_tuple_elements_opt),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyTupleElementsView {
    pub key_value: KeyValueHandle,
    pub key_tuple_elements_opt: KeyTupleElementsOptHandle,
}
impl KeyTupleElementsView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyTupleElementsOptHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for KeyTupleElementsOptHandle {
    type View = Option<KeyTupleElementsTailHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::KeyTupleElementsOpt)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::KeyTupleElementsOpt
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(visit(
            Some(KeyTupleElementsTailHandle::new_with_visit(
                self.0,
                tree,
                visit_ignored,
            )?),
            visit_ignored,
        )
        .0)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyTupleElementsTailHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for KeyTupleElementsTailHandle {
    type View = KeyTupleElementsTailView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::KeyTupleElementsTail)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::KeyTupleElementsTail
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Comma),
                NodeKind::NonTerminal(NonTerminalKind::KeyTupleElementsTailOpt),
            ],
            |[comma, key_tuple_elements_tail_opt], visit_ignored| {
                Ok(visit(
                    KeyTupleElementsTailView {
                        comma: CommaHandle(comma),
                        key_tuple_elements_tail_opt: KeyTupleElementsTailOptHandle(
                            key_tuple_elements_tail_opt,
                        ),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyTupleElementsTailView {
    pub comma: CommaHandle,
    pub key_tuple_elements_tail_opt: KeyTupleElementsTailOptHandle,
}
impl KeyTupleElementsTailView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyTupleElementsTailOptHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for KeyTupleElementsTailOptHandle {
    type View = Option<KeyTupleElementsHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(
                NonTerminalKind::KeyTupleElementsTailOpt,
            )],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::KeyTupleElementsTailOpt
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(visit(
            Some(KeyTupleElementsHandle::new_with_visit(
                self.0,
                tree,
                visit_ignored,
            )?),
            visit_ignored,
        )
        .0)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyTupleOptHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for KeyTupleOptHandle {
    type View = Option<KeyTupleElementsHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::KeyTupleOpt)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::KeyTupleOpt
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(visit(
            Some(KeyTupleElementsHandle::new_with_visit(
                self.0,
                tree,
                visit_ignored,
            )?),
            visit_ignored,
        )
        .0)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyValueHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for KeyValueHandle {
    type View = KeyValueView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::KeyValue)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::KeyValue
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        let mut children = tree.children(self.0);
        let Some(child) = children.next() else {
            return Err(ViewConstructionError::UnexpectedEndOfChildren { parent: self.0 }.into());
        };
        let Some(child_data) = tree.node_data(child) else {
            return Err(ViewConstructionError::NodeIdNotFound { node: child }.into());
        };
        let variant = match child_data.node_kind() {
            NodeKind::NonTerminal(NonTerminalKind::Integer) => {
                KeyValueView::Integer(IntegerHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::Boolean) => {
                KeyValueView::Boolean(BooleanHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::Str) => KeyValueView::Str(StrHandle(child)),
            NodeKind::NonTerminal(NonTerminalKind::KeyTuple) => {
                KeyValueView::KeyTuple(KeyTupleHandle(child))
            }
            _ => {
                return Err(ViewConstructionError::UnexpectedNodeData {
                    node: child,
                    data: child_data,
                }
                .into());
            }
        };
        let (result, _visit) = visit(variant, visit_ignored);
        if let Some(extra_child) = children.next() {
            return Err(ViewConstructionError::UnexpectedExtraNode { node: extra_child }.into());
        }
        Ok(result)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyValueView {
    Integer(IntegerHandle),
    Boolean(BooleanHandle),
    Str(StrHandle),
    KeyTuple(KeyTupleHandle),
}
impl KeyValueView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeysHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for KeysHandle {
    type View = KeysView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Key),
                NodeKind::NonTerminal(NonTerminalKind::KeysList),
            ],
            |[key, keys_list], visit_ignored| {
                Ok(visit(
                    KeysView {
                        key: KeyHandle(key),
                        keys_list: KeysListHandle(keys_list),
                    },
                    visit_ignored,
                ))
            },
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for KeysListHandle {
    type View = Option<KeysListView>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
            |[dot, key, keys_list], visit_ignored| {
                Ok(visit(
                    Some(KeysListView {
                        dot: DotHandle(dot),
                        key: KeyHandle(key),
                        keys_list: KeysListHandle(keys_list),
                    }),
                    visit_ignored,
                ))
            },
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
impl<F: CstFacade<TerminalKind, NonTerminalKind>> RecursiveView<TerminalKind, NonTerminalKind, F>
    for KeysListView
{
    type Item = KeysListItem;
    fn get_all_with_visit<E>(
        &self,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Vec<Self::Item>, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        let mut items = Vec::new();
        let mut current_view = Some(*self);
        while let Some(item) = current_view {
            let Self { dot, key, .. } = item;
            items.push(KeysListItem { dot, key });
            item.keys_list.get_view_with_visit(
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for LParenHandle {
    type View = LParenView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::LParen)],
            |[l_paren], visit_ignored| {
                Ok(visit(
                    LParenView {
                        l_paren: LParen(l_paren),
                    },
                    visit_ignored,
                ))
            },
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
pub struct MapBindHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for MapBindHandle {
    type View = MapBindView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::MapBind)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::MapBind
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::MapBind)],
            |[map_bind], visit_ignored| {
                Ok(visit(
                    MapBindView {
                        map_bind: MapBind(map_bind),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapBindView {
    pub map_bind: MapBind,
}
impl MapBindView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoBacktickHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for NoBacktickHandle {
    type View = NoBacktickView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::NoBacktick)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::NoBacktick
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::NoBacktick)],
            |[no_backtick], visit_ignored| {
                Ok(visit(
                    NoBacktickView {
                        no_backtick: NoBacktick(no_backtick),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoBacktickView {
    pub no_backtick: NoBacktick,
}
impl NoBacktickView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoBacktickInlineHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for NoBacktickInlineHandle {
    type View = NoBacktickInlineView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::NoBacktickInline)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::NoBacktickInline
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::NoBacktickInline)],
            |[no_backtick_inline], visit_ignored| {
                Ok(visit(
                    NoBacktickInlineView {
                        no_backtick_inline: NoBacktickInline(no_backtick_inline),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoBacktickInlineView {
    pub no_backtick_inline: NoBacktickInline,
}
impl NoBacktickInlineView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NullHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for NullHandle {
    type View = NullView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Null)],
            |[null], visit_ignored| Ok(visit(NullView { null: Null(null) }, visit_ignored)),
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for ObjectHandle {
    type View = ObjectView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Begin),
                NodeKind::NonTerminal(NonTerminalKind::ObjectOpt),
                NodeKind::NonTerminal(NonTerminalKind::ObjectList),
                NodeKind::NonTerminal(NonTerminalKind::End),
            ],
            |[begin, object_opt, object_list, end], visit_ignored| {
                Ok(visit(
                    ObjectView {
                        begin: BeginHandle(begin),
                        object_opt: ObjectOptHandle(object_opt),
                        object_list: ObjectListHandle(object_list),
                        end: EndHandle(end),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectView {
    pub begin: BeginHandle,
    pub object_opt: ObjectOptHandle,
    pub object_list: ObjectListHandle,
    pub end: EndHandle,
}
impl ObjectView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectListHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for ObjectListHandle {
    type View = Option<ObjectListView>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Keys),
                NodeKind::NonTerminal(NonTerminalKind::MapBind),
                NodeKind::NonTerminal(NonTerminalKind::Value),
                NodeKind::NonTerminal(NonTerminalKind::ObjectOpt0),
                NodeKind::NonTerminal(NonTerminalKind::ObjectList),
            ],
            |[keys, map_bind, value, object_opt_0, object_list], visit_ignored| {
                Ok(visit(
                    Some(ObjectListView {
                        keys: KeysHandle(keys),
                        map_bind: MapBindHandle(map_bind),
                        value: ValueHandle(value),
                        object_opt_0: ObjectOpt0Handle(object_opt_0),
                        object_list: ObjectListHandle(object_list),
                    }),
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectListView {
    pub keys: KeysHandle,
    pub map_bind: MapBindHandle,
    pub value: ValueHandle,
    pub object_opt_0: ObjectOpt0Handle,
    pub object_list: ObjectListHandle,
}
impl<F: CstFacade<TerminalKind, NonTerminalKind>> RecursiveView<TerminalKind, NonTerminalKind, F>
    for ObjectListView
{
    type Item = ObjectListItem;
    fn get_all_with_visit<E>(
        &self,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Vec<Self::Item>, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        let mut items = Vec::new();
        let mut current_view = Some(*self);
        while let Some(item) = current_view {
            let Self {
                keys,
                map_bind,
                value,
                object_opt_0,
                ..
            } = item;
            items.push(ObjectListItem {
                keys,
                map_bind,
                value,
                object_opt_0,
            });
            item.object_list.get_view_with_visit(
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
    pub keys: KeysHandle,
    pub map_bind: MapBindHandle,
    pub value: ValueHandle,
    pub object_opt_0: ObjectOpt0Handle,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectOptHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for ObjectOptHandle {
    type View = Option<ObjectOptView>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::ValueBinding),
                NodeKind::NonTerminal(NonTerminalKind::ObjectOpt1),
            ],
            |[value_binding, object_opt_1], visit_ignored| {
                Ok(visit(
                    Some(ObjectOptView {
                        value_binding: ValueBindingHandle(value_binding),
                        object_opt_1: ObjectOpt1Handle(object_opt_1),
                    }),
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectOptView {
    pub value_binding: ValueBindingHandle,
    pub object_opt_1: ObjectOpt1Handle,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectOpt0Handle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for ObjectOpt0Handle {
    type View = Option<CommaHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::ObjectOpt0)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::ObjectOpt0
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(visit(
            Some(CommaHandle::new_with_visit(self.0, tree, visit_ignored)?),
            visit_ignored,
        )
        .0)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectOpt1Handle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for ObjectOpt1Handle {
    type View = Option<CommaHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::ObjectOpt1)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::ObjectOpt1
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(visit(
            Some(CommaHandle::new_with_visit(self.0, tree, visit_ignored)?),
            visit_ignored,
        )
        .0)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RParenHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for RParenHandle {
    type View = RParenView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::RParen)],
            |[r_paren], visit_ignored| {
                Ok(visit(
                    RParenView {
                        r_paren: RParen(r_paren),
                    },
                    visit_ignored,
                ))
            },
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for SectionHandle {
    type View = SectionView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::At),
                NodeKind::NonTerminal(NonTerminalKind::Keys),
                NodeKind::NonTerminal(NonTerminalKind::SectionBody),
            ],
            |[at, keys, section_body], visit_ignored| {
                Ok(visit(
                    SectionView {
                        at: AtHandle(at),
                        keys: KeysHandle(keys),
                        section_body: SectionBodyHandle(section_body),
                    },
                    visit_ignored,
                ))
            },
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for SectionBindingHandle {
    type View = SectionBindingView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Begin),
                NodeKind::NonTerminal(NonTerminalKind::Eure),
                NodeKind::NonTerminal(NonTerminalKind::End),
            ],
            |[begin, eure, end], visit_ignored| {
                Ok(visit(
                    SectionBindingView {
                        begin: BeginHandle(begin),
                        eure: EureHandle(eure),
                        end: EndHandle(end),
                    },
                    visit_ignored,
                ))
            },
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for SectionBodyHandle {
    type View = SectionBodyView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        let mut children = tree.children(self.0);
        let Some(child) = children.next() else {
            return Err(ViewConstructionError::UnexpectedEndOfChildren { parent: self.0 }.into());
        };
        let Some(child_data) = tree.node_data(child) else {
            return Err(ViewConstructionError::NodeIdNotFound { node: child }.into());
        };
        match child_data.node_kind() {
            NodeKind::NonTerminal(NonTerminalKind::SectionBodyOpt) => tree.collect_nodes(
                self.0,
                [
                    NodeKind::NonTerminal(NonTerminalKind::SectionBodyOpt),
                    NodeKind::NonTerminal(NonTerminalKind::SectionBodyList),
                ],
                |[section_body_opt, section_body_list], visit_ignored| {
                    Ok(visit(
                        SectionBodyView::Alt0(SectionBodyAlt0 {
                            section_body_opt: SectionBodyOptHandle(section_body_opt),
                            section_body_list: SectionBodyListHandle(section_body_list),
                        }),
                        visit_ignored,
                    ))
                },
                visit_ignored,
            ),
            NodeKind::NonTerminal(NonTerminalKind::Begin) => tree.collect_nodes(
                self.0,
                [
                    NodeKind::NonTerminal(NonTerminalKind::Begin),
                    NodeKind::NonTerminal(NonTerminalKind::Eure),
                    NodeKind::NonTerminal(NonTerminalKind::End),
                ],
                |[begin, eure, end], visit_ignored| {
                    Ok(visit(
                        SectionBodyView::Alt1(SectionBodyAlt1 {
                            begin: BeginHandle(begin),
                            eure: EureHandle(eure),
                            end: EndHandle(end),
                        }),
                        visit_ignored,
                    ))
                },
                visit_ignored,
            ),
            _ => Err(ViewConstructionError::UnexpectedNodeData {
                node: child,
                data: child_data,
            }
            .into()),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionBodyView {
    Alt0(SectionBodyAlt0),
    Alt1(SectionBodyAlt1),
}
impl SectionBodyView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionBodyAlt0 {
    pub section_body_opt: SectionBodyOptHandle,
    pub section_body_list: SectionBodyListHandle,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionBodyAlt1 {
    pub begin: BeginHandle,
    pub eure: EureHandle,
    pub end: EndHandle,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SectionBodyListHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for SectionBodyListHandle {
    type View = Option<SectionBodyListView>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Binding),
                NodeKind::NonTerminal(NonTerminalKind::SectionBodyList),
            ],
            |[binding, section_body_list], visit_ignored| {
                Ok(visit(
                    Some(SectionBodyListView {
                        binding: BindingHandle(binding),
                        section_body_list: SectionBodyListHandle(section_body_list),
                    }),
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionBodyListView {
    pub binding: BindingHandle,
    pub section_body_list: SectionBodyListHandle,
}
impl<F: CstFacade<TerminalKind, NonTerminalKind>> RecursiveView<TerminalKind, NonTerminalKind, F>
    for SectionBodyListView
{
    type Item = BindingHandle;
    fn get_all_with_visit<E>(
        &self,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Vec<Self::Item>, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        let mut items = Vec::new();
        let mut current_view = Some(*self);
        while let Some(item) = current_view {
            let Self { binding, .. } = item;
            items.push(binding);
            item.section_body_list.get_view_with_visit(
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
pub struct SectionBodyOptHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for SectionBodyOptHandle {
    type View = Option<ValueBindingHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::SectionBodyOpt)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::SectionBodyOpt
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(visit(
            Some(ValueBindingHandle::new_with_visit(
                self.0,
                tree,
                visit_ignored,
            )?),
            visit_ignored,
        )
        .0)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StrHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for StrHandle {
    type View = StrView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for StringsHandle {
    type View = StringsView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Str),
                NodeKind::NonTerminal(NonTerminalKind::StringsList),
            ],
            |[str, strings_list], visit_ignored| {
                Ok(visit(
                    StringsView {
                        str: StrHandle(str),
                        strings_list: StringsListHandle(strings_list),
                    },
                    visit_ignored,
                ))
            },
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for StringsListHandle {
    type View = Option<StringsListView>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
            |[r#continue, str, strings_list], visit_ignored| {
                Ok(visit(
                    Some(StringsListView {
                        r#continue: ContinueHandle(r#continue),
                        str: StrHandle(str),
                        strings_list: StringsListHandle(strings_list),
                    }),
                    visit_ignored,
                ))
            },
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
impl<F: CstFacade<TerminalKind, NonTerminalKind>> RecursiveView<TerminalKind, NonTerminalKind, F>
    for StringsListView
{
    type Item = StringsListItem;
    fn get_all_with_visit<E>(
        &self,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Vec<Self::Item>, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        let mut items = Vec::new();
        let mut current_view = Some(*self);
        while let Some(item) = current_view {
            let Self {
                r#continue, str, ..
            } = item;
            items.push(StringsListItem { r#continue, str });
            item.strings_list.get_view_with_visit(
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for TextHandle {
    type View = TextView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::Text)],
            |[text], visit_ignored| Ok(visit(TextView { text: Text(text) }, visit_ignored)),
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for TextBindingHandle {
    type View = TextBindingView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::TextStart),
                NodeKind::NonTerminal(NonTerminalKind::TextBindingOpt),
                NodeKind::NonTerminal(NonTerminalKind::Text),
                NodeKind::NonTerminal(NonTerminalKind::TextBindingOpt0),
            ],
            |[text_start, text_binding_opt, text, text_binding_opt_0], visit_ignored| {
                Ok(visit(
                    TextBindingView {
                        text_start: TextStartHandle(text_start),
                        text_binding_opt: TextBindingOptHandle(text_binding_opt),
                        text: TextHandle(text),
                        text_binding_opt_0: TextBindingOpt0Handle(text_binding_opt_0),
                    },
                    visit_ignored,
                ))
            },
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for TextBindingOptHandle {
    type View = Option<WsHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(visit(
            Some(WsHandle::new_with_visit(self.0, tree, visit_ignored)?),
            visit_ignored,
        )
        .0)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextBindingOpt0Handle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for TextBindingOpt0Handle {
    type View = Option<GrammarNewlineHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(visit(
            Some(GrammarNewlineHandle::new_with_visit(
                self.0,
                tree,
                visit_ignored,
            )?),
            visit_ignored,
        )
        .0)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextStartHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for TextStartHandle {
    type View = TextStartView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::TextStart)],
            |[text_start], visit_ignored| {
                Ok(visit(
                    TextStartView {
                        text_start: TextStart(text_start),
                    },
                    visit_ignored,
                ))
            },
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for TrueHandle {
    type View = TrueView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::Terminal(TerminalKind::True)],
            |[r#true], visit_ignored| {
                Ok(visit(
                    TrueView {
                        r#true: True(r#true),
                    },
                    visit_ignored,
                ))
            },
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for TupleHandle {
    type View = TupleView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::LParen),
                NodeKind::NonTerminal(NonTerminalKind::TupleOpt),
                NodeKind::NonTerminal(NonTerminalKind::RParen),
            ],
            |[l_paren, tuple_opt, r_paren], visit_ignored| {
                Ok(visit(
                    TupleView {
                        l_paren: LParenHandle(l_paren),
                        tuple_opt: TupleOptHandle(tuple_opt),
                        r_paren: RParenHandle(r_paren),
                    },
                    visit_ignored,
                ))
            },
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for TupleElementsHandle {
    type View = TupleElementsView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Value),
                NodeKind::NonTerminal(NonTerminalKind::TupleElementsOpt),
            ],
            |[value, tuple_elements_opt], visit_ignored| {
                Ok(visit(
                    TupleElementsView {
                        value: ValueHandle(value),
                        tuple_elements_opt: TupleElementsOptHandle(tuple_elements_opt),
                    },
                    visit_ignored,
                ))
            },
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for TupleElementsOptHandle {
    type View = Option<TupleElementsTailHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(visit(
            Some(TupleElementsTailHandle::new_with_visit(
                self.0,
                tree,
                visit_ignored,
            )?),
            visit_ignored,
        )
        .0)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TupleElementsTailHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for TupleElementsTailHandle {
    type View = TupleElementsTailView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Comma),
                NodeKind::NonTerminal(NonTerminalKind::TupleElementsTailOpt),
            ],
            |[comma, tuple_elements_tail_opt], visit_ignored| {
                Ok(visit(
                    TupleElementsTailView {
                        comma: CommaHandle(comma),
                        tuple_elements_tail_opt: TupleElementsTailOptHandle(
                            tuple_elements_tail_opt,
                        ),
                    },
                    visit_ignored,
                ))
            },
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for TupleElementsTailOptHandle {
    type View = Option<TupleElementsHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(visit(
            Some(TupleElementsHandle::new_with_visit(
                self.0,
                tree,
                visit_ignored,
            )?),
            visit_ignored,
        )
        .0)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TupleIndexHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for TupleIndexHandle {
    type View = TupleIndexView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            index,
            [NodeKind::NonTerminal(NonTerminalKind::TupleIndex)],
            |[index], visit| Ok((Self(index), visit)),
            visit_ignored,
        )
    }
    fn kind(&self) -> NonTerminalKind {
        NonTerminalKind::TupleIndex
    }
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::Terminal(TerminalKind::Hash),
                NodeKind::NonTerminal(NonTerminalKind::Integer),
            ],
            |[hash, integer], visit_ignored| {
                Ok(visit(
                    TupleIndexView {
                        hash: Hash(hash),
                        integer: IntegerHandle(integer),
                    },
                    visit_ignored,
                ))
            },
            visit_ignored,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TupleIndexView {
    pub hash: Hash,
    pub integer: IntegerHandle,
}
impl TupleIndexView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TupleOptHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for TupleOptHandle {
    type View = Option<TupleElementsHandle>;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        if tree.has_no_children(self.0) {
            return Ok(visit(None, visit_ignored).0);
        }
        Ok(visit(
            Some(TupleElementsHandle::new_with_visit(
                self.0,
                tree,
                visit_ignored,
            )?),
            visit_ignored,
        )
        .0)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ValueHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for ValueHandle {
    type View = ValueView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        let mut children = tree.children(self.0);
        let Some(child) = children.next() else {
            return Err(ViewConstructionError::UnexpectedEndOfChildren { parent: self.0 }.into());
        };
        let Some(child_data) = tree.node_data(child) else {
            return Err(ViewConstructionError::NodeIdNotFound { node: child }.into());
        };
        let variant = match child_data.node_kind() {
            NodeKind::NonTerminal(NonTerminalKind::Object) => {
                ValueView::Object(ObjectHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::Array) => ValueView::Array(ArrayHandle(child)),
            NodeKind::NonTerminal(NonTerminalKind::Tuple) => ValueView::Tuple(TupleHandle(child)),
            NodeKind::NonTerminal(NonTerminalKind::Float) => ValueView::Float(FloatHandle(child)),
            NodeKind::NonTerminal(NonTerminalKind::Integer) => {
                ValueView::Integer(IntegerHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::Boolean) => {
                ValueView::Boolean(BooleanHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::Null) => ValueView::Null(NullHandle(child)),
            NodeKind::NonTerminal(NonTerminalKind::Strings) => {
                ValueView::Strings(StringsHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::Hole) => ValueView::Hole(HoleHandle(child)),
            NodeKind::NonTerminal(NonTerminalKind::CodeBlock) => {
                ValueView::CodeBlock(CodeBlockHandle(child))
            }
            NodeKind::NonTerminal(NonTerminalKind::InlineCode) => {
                ValueView::InlineCode(InlineCodeHandle(child))
            }
            _ => {
                return Err(ViewConstructionError::UnexpectedNodeData {
                    node: child,
                    data: child_data,
                }
                .into());
            }
        };
        let (result, _visit) = visit(variant, visit_ignored);
        if let Some(extra_child) = children.next() {
            return Err(ViewConstructionError::UnexpectedExtraNode { node: extra_child }.into());
        }
        Ok(result)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueView {
    Object(ObjectHandle),
    Array(ArrayHandle),
    Tuple(TupleHandle),
    Float(FloatHandle),
    Integer(IntegerHandle),
    Boolean(BooleanHandle),
    Null(NullHandle),
    Strings(StringsHandle),
    Hole(HoleHandle),
    CodeBlock(CodeBlockHandle),
    InlineCode(InlineCodeHandle),
}
impl ValueView {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ValueBindingHandle(pub(crate) super::tree::CstNodeId);
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for ValueBindingHandle {
    type View = ValueBindingView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [
                NodeKind::NonTerminal(NonTerminalKind::Bind),
                NodeKind::NonTerminal(NonTerminalKind::Value),
            ],
            |[bind, value], visit_ignored| {
                Ok(visit(
                    ValueBindingView {
                        bind: BindHandle(bind),
                        value: ValueHandle(value),
                    },
                    visit_ignored,
                ))
            },
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for WsHandle {
    type View = WsView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
impl NonTerminalHandle<TerminalKind, NonTerminalKind> for RootHandle {
    type View = RootView;
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn new_with_visit<F: CstFacade<TerminalKind, NonTerminalKind>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
    ) -> Result<Self, CstConstructError<TerminalKind, NonTerminalKind, E>> {
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
    fn get_view_with_visit<
        'v,
        F: CstFacade<TerminalKind, NonTerminalKind>,
        V: BuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, F>,
        O,
        E,
    >(
        &self,
        tree: &F,
        mut visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        tree.collect_nodes(
            self.0,
            [NodeKind::NonTerminal(NonTerminalKind::Eure)],
            |[eure], visit_ignored| {
                Ok(visit(
                    RootView {
                        eure: EureHandle(eure),
                    },
                    visit_ignored,
                ))
            },
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
pub struct NewLine(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for NewLine {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::NewLine
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Whitespace(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for Whitespace {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Whitespace
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LineComment(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for LineComment {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::LineComment
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockComment(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for BlockComment {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::BlockComment
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hash(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for Hash {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Hash
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MapBind(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for MapBind {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::MapBind
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Integer(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for Integer {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Integer
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Float(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for Float {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Float
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct True(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for True {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::True
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct False(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for False {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::False
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Null(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for Null {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Null
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hole(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for Hole {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Hole
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Str(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for Str {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Str
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Text(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for Text {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Text
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InlineCode1(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for InlineCode1 {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::InlineCode1
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InlineCodeStart2(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for InlineCodeStart2 {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::InlineCodeStart2
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CodeBlockStart3(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for CodeBlockStart3 {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::CodeBlockStart3
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CodeBlockStart4(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for CodeBlockStart4 {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::CodeBlockStart4
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CodeBlockStart5(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for CodeBlockStart5 {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::CodeBlockStart5
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CodeBlockStart6(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for CodeBlockStart6 {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::CodeBlockStart6
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CodeBlockEnd3(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for CodeBlockEnd3 {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::CodeBlockEnd3
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Backtick2(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for Backtick2 {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Backtick2
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CodeBlockEnd4(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for CodeBlockEnd4 {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::CodeBlockEnd4
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Backtick3(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for Backtick3 {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Backtick3
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CodeBlockEnd5(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for CodeBlockEnd5 {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::CodeBlockEnd5
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Backtick4(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for Backtick4 {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Backtick4
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CodeBlockEnd6(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for CodeBlockEnd6 {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::CodeBlockEnd6
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Backtick5(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for Backtick5 {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Backtick5
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InlineCodeEnd2(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for InlineCodeEnd2 {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::InlineCodeEnd2
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Backtick1(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for Backtick1 {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Backtick1
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoBacktick(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for NoBacktick {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::NoBacktick
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoBacktickInline(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for NoBacktickInline {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::NoBacktickInline
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GrammarNewline(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for GrammarNewline {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::GrammarNewline
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ws(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for Ws {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Ws
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct At(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for At {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::At
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Dollar(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for Dollar {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Dollar
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Dot(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for Dot {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Dot
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LBrace(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for LBrace {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::LBrace
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RBrace(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for RBrace {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::RBrace
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LBracket(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for LBracket {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::LBracket
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RBracket(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for RBracket {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::RBracket
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LParen(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for LParen {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::LParen
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RParen(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for RParen {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::RParen
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bind(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for Bind {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Bind
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Comma(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for Comma {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Comma
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Esc(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for Esc {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Esc
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextStart(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for TextStart {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::TextStart
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ident(pub(crate) CstNodeId);
impl TerminalHandle<TerminalKind> for Ident {
    fn node_id(&self) -> CstNodeId {
        self.0
    }
    fn kind(&self) -> TerminalKind {
        TerminalKind::Ident
    }
}
