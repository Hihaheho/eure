#[cfg(test)]
mod tests {
    use super::super::value_visitor::*;
    use eure_value::value::Value;

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
    fn test_document_to_value() {
        // Test the document_to_value function with an empty document
        let visitor = ValueVisitor::new("");
        let doc = visitor.into_document();
        let value = document_to_value(doc);
        
        // Should produce an empty map
        match value {
            Value::Map(map) => {
                assert!(map.0.is_empty(), "Expected empty map");
            }
            _ => panic!("Expected Map value"),
        }
    }
}

// Note: The previous tests were testing the old implementation with Values struct
// and private methods. Those tests are no longer applicable with the new
// EureDocument-based implementation. New integration tests should be written
// to test the ValueVisitor through the public API using actual parsing.