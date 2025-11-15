use crate::prelude_internal::*;

pub struct DocumentConstructor<'d> {
    document: &'d mut EureDocument,
    stack: Vec<NodeId>,
    path: Vec<PathSegment>,
}

impl<'d> DocumentConstructor<'d> {
    pub fn current_node_id(&self) -> NodeId {
        self.stack.last().copied().unwrap()
    }

    pub fn current_node(&self) -> &Node {
        self.document.get_node(self.current_node_id())
    }
}

impl<'d> DocumentConstructor<'d> {
    /// Prepare a node at the current path.
    pub fn prepare_node(
        &mut self,
        path_iter: impl Iterator<Item = PathSegment>,
        new: NodeValue,
    ) -> Result<(), InsertErrorKind> {
        for segment in path_iter {
            let node_id = self
                .document
                .add_child_by_segment(segment.clone(), self.current_node_id())?;
            self.path.push(segment);
            self.stack.push(node_id);
        }
        self.document.get_node_mut(self.current_node_id()).content = new;
        Ok(())
    }
}
