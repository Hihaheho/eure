#[cfg(test)]
mod tests {
    use super::super::value_visitor::*;

    #[test]
    fn test_value_visitor_new() {
        let input = "test";
        let visitor = ValueVisitor::new(input);
        // Simply verify that we can create a new visitor
        let _doc = visitor.into_document();
    }

    #[test]
    fn test_error_types() {
        // Test that error types can be constructed
        let _err1 = ValueVisitorError::InvalidIdentifier("bad_ident".to_string());
        let _err2 = ValueVisitorError::InvalidInteger("not_a_number".to_string());
    }

    #[test]
    fn test_empty_document() {
        // Test creating an empty document
        let visitor = ValueVisitor::new("");
        let doc = visitor.into_document();
        
        // The document should have a root node that is an empty map
        let root = doc.get_root();
        match &root.content {
            crate::document::NodeContent::Map { entries, .. } => {
                assert!(entries.is_empty(), "Expected empty map at root");
            }
            _ => panic!("Expected Map node at root"),
        }
    }
}

// Note: The previous tests were testing the old implementation with Values struct
// and private methods. Those tests are no longer applicable with the new
// EureDocument-based implementation. New integration tests should be written
// to test the ValueVisitor through the public API using actual parsing.