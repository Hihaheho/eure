use eure_tree::document::{DocumentKey, NodeValue};
use eure_tree::value_visitor::ValueVisitor;
use eure_value::identifier::Identifier;
use std::str::FromStr;

#[test]
fn test_section_with_value_assignment() {
    // Test how @ section = value is parsed
    let input = r#"
@ section1 = "direct value"
@ section2.nested = 123
@ section3.$extension = true
"#;

    let tree = eure_parol::parse(input).expect("Parse should succeed");
    let mut visitor = ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor)
        .expect("Visit should succeed");

    let document = visitor.into_document();
    let root = document.get_root();

    println!("Document structure:");

    if let NodeValue::Map { entries, .. } = &root.content {
        println!("\nTop-level keys: {} entries", entries.len());
        for (key, _) in entries {
            match key {
                DocumentKey::Ident(ident) => println!("  - Ident: {}", ident),
                DocumentKey::MetaExtension(ident) => println!("  - MetaExtension: {}", ident),
                DocumentKey::Value(val) => println!("  - Value: {:?}", val),
                DocumentKey::TupleIndex(idx) => println!("  - TupleIndex: {}", idx),
            }
        }

        // These should create sections, but the value assignment might not work
        // as the grammar shows SectionBody can be "Bind Value" but the visitor
        // might not handle it properly
    }
}

#[test]
fn test_actual_failing_case() {
    // This is what's in the failing test
    let input = r#"
@ $types.User
name.$type = .string
age.$type = .number

@
users.$array = .$types.User
"#;

    let tree = eure_parol::parse(input).expect("Parse should succeed");
    let mut visitor = ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor)
        .expect("Visit should succeed");

    let document = visitor.into_document();
    let root = document.get_root();

    // Navigate the document structure
    if let NodeValue::Map { entries, .. } = &root.content {
        println!("Found {} root entries", entries.len());

        // Look for users field
        let users_key = DocumentKey::Ident(Identifier::from_str("users").unwrap());
        if let Some(users_node_id) = entries
            .iter()
            .find(|(k, _)| k == &users_key)
            .map(|(_, id)| id)
        {
            let users_node = document.get_node(*users_node_id);
            println!(
                "Found users node with extensions: {:?}",
                users_node.extensions.keys().collect::<Vec<_>>()
            );

            // Check if $array extension has the correct value
            if let Some(array_node_id) = users_node
                .extensions
                .get(&Identifier::from_str("array").unwrap())
            {
                let array_node = document.get_node(*array_node_id);
                match &array_node.content {
                    NodeValue::Path { value, .. } => {
                        println!("SUCCESS: users.$array is a Path: {:?}", value);
                    }
                    _ => {
                        println!(
                            "FAIL: users.$array is not a Path but: {:?}",
                            array_node.content
                        );
                    }
                }
            }
        }
    }
}
