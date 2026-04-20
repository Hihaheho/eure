//! Compatibility re-exports for schema codegen metadata types.

pub use eure_schema::{CodegenDefaults, FieldCodegen, RecordCodegen, RootCodegen, UnionCodegen};

#[cfg(test)]
mod tests {
    use super::*;
    use eure_document::document::node::NodeValue;
    use eure_document::document::{EureDocument, NodeId};

    fn create_empty_map_node(doc: &mut EureDocument) -> NodeId {
        let root_id = doc.get_root_id();
        doc.node_mut(root_id).content = NodeValue::Map(Default::default());
        root_id
    }

    #[test]
    fn test_root_codegen_empty() {
        let mut doc = EureDocument::new();
        let node_id = create_empty_map_node(&mut doc);

        let result: RootCodegen = doc.parse(node_id).unwrap();
        assert!(result.type_name.is_none());
    }

    #[test]
    fn test_codegen_defaults_empty() {
        let mut doc = EureDocument::new();
        let node_id = create_empty_map_node(&mut doc);

        let result: CodegenDefaults = doc.parse(node_id).unwrap();
        assert!(result.derive.is_none());
        assert!(result.inline_derive.is_none());
        assert!(result.variant_type_derive.is_none());
        assert!(result.ext_types_field_prefix.is_none());
        assert!(result.ext_types_type_prefix.is_none());
        assert!(result.document_node_id_field.is_none());
    }

    #[test]
    fn test_union_codegen_empty() {
        let mut doc = EureDocument::new();
        let node_id = create_empty_map_node(&mut doc);

        let result: UnionCodegen = doc.parse(node_id).unwrap();
        assert!(result.type_name.is_none());
        assert!(result.derive.is_none());
        assert!(result.inline_derive.is_none());
        assert!(result.variant_types.is_none());
        assert!(result.variant_types_suffix.is_none());
        assert!(result.variant_type_derive.is_none());
    }

    #[test]
    fn test_record_codegen_empty() {
        let mut doc = EureDocument::new();
        let node_id = create_empty_map_node(&mut doc);

        let result: RecordCodegen = doc.parse(node_id).unwrap();
        assert!(result.type_name.is_none());
        assert!(result.derive.is_none());
        assert!(result.inline_derive.is_none());
    }

    #[test]
    fn test_field_codegen_empty() {
        let mut doc = EureDocument::new();
        let node_id = create_empty_map_node(&mut doc);

        let result: FieldCodegen = doc.parse(node_id).unwrap();
        assert!(result.name.is_none());
    }
}
