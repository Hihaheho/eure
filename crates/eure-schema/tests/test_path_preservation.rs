use eure_tree::value_visitor::ValueVisitor;
use eure_value::value::{KeyCmpValue, Path, PathSegment, Value};

#[test]
fn test_path_segments_preserved() {
    // Test that PathSegment types are preserved
    let input = r#"
test1 = .regular.path
test2 = .$types.User
test3 = .$$meta.extension
"#;

    let tree = eure_parol::parse(input).expect("Parse should succeed");
    let mut visitor = ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor)
        .expect("Visit should succeed");
    let document = visitor.into_document();
    let doc = document.to_value();

    println!("Document: {doc:#?}");

    if let Value::Map(map) = doc {
        // Check test1 - regular path
        if let Some(Value::Path(Path(segments))) =
            map.0.get(&KeyCmpValue::String("test1".to_string()))
        {
            assert_eq!(segments.len(), 2);
            assert!(matches!(&segments[0], PathSegment::Ident(id) if id.to_string() == "regular"));
            assert!(matches!(&segments[1], PathSegment::Ident(id) if id.to_string() == "path"));
            println!("test1 segments correct");
        } else {
            panic!("test1 should be a Path");
        }

        // Check test2 - extension path
        if let Some(Value::Path(Path(segments))) =
            map.0.get(&KeyCmpValue::String("test2".to_string()))
        {
            assert_eq!(segments.len(), 2);
            assert!(
                matches!(&segments[0], PathSegment::Extension(id) if id.to_string() == "types")
            );
            assert!(matches!(&segments[1], PathSegment::Ident(id) if id.to_string() == "User"));
            println!("test2 segments correct");
        } else {
            panic!("test2 should be a Path");
        }

        // Check test3 - meta extension path
        if let Some(Value::Path(Path(segments))) =
            map.0.get(&KeyCmpValue::String("test3".to_string()))
        {
            assert_eq!(segments.len(), 2);
            assert!(matches!(&segments[0], PathSegment::MetaExt(id) if id.to_string() == "meta"));
            assert!(
                matches!(&segments[1], PathSegment::Ident(id) if id.to_string() == "extension")
            );
            println!("test3 segments correct");
        } else {
            panic!("test3 should be a Path");
        }
    }
}

#[test]
fn test_nested_path_preservation() {
    // Test paths in nested structures
    let input = r#"
nested.field = .$types.User
"#;

    let tree = eure_parol::parse(input).expect("Parse should succeed");
    let mut visitor = ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor)
        .expect("Visit should succeed");
    let document = visitor.into_document();
    let doc = document.to_value();

    println!("\nNested document: {doc:#?}");

    if let Value::Map(map) = doc {
        if let Some(Value::Map(nested_map)) = map.0.get(&KeyCmpValue::String("nested".to_string()))
        {
            if let Some(value) = nested_map.0.get(&KeyCmpValue::String("field".to_string())) {
                match value {
                    Value::Path(Path(segments)) => {
                        println!("Found Path with segments: {segments:?}");
                        assert_eq!(segments.len(), 2);
                        assert!(
                            matches!(&segments[0], PathSegment::Extension(id) if id.to_string() == "types")
                        );
                        assert!(
                            matches!(&segments[1], PathSegment::Ident(id) if id.to_string() == "User")
                        );
                    }
                    _ => panic!("nested.field should be a Path, but got: {value:?}"),
                }
            } else {
                panic!("nested.field not found");
            }
        } else {
            panic!("nested not found or not a Map");
        }
    }
}
