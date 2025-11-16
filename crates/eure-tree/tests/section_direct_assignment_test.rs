use eure_tree::document::{DocumentKey, NodeValue};
use eure_tree::value_visitor::ValueVisitor;
use eure_value::identifier::Identifier;
use std::str::FromStr;

#[test]
fn test_simple_section_direct_assignment() {
    // Test simple direct assignment: @ key = "value"
    let input = r#"
@ section1 = "direct value"
@ section2 = 123
@ section3 = true
@ section4 = null
"#;

    let tree = eure_parol::parse(input).expect("Parse should succeed");
    let mut visitor = ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor)
        .expect("Visit should succeed");

    let document = visitor.into_document();
    let root = document.root();

    if let NodeValue::Map { entries, .. } = &root.content {
        assert_eq!(entries.len(), 4, "Should have 4 sections");

        // Check section1
        let section1_key = DocumentKey::Ident(Identifier::from_str("section1").unwrap());
        let section1_node = entries
            .iter()
            .find(|(k, _)| k == &section1_key)
            .map(|(_, id)| document.node(*id))
            .expect("section1 should exist");

        match &section1_node.content {
            NodeValue::String { value, .. } => {
                assert_eq!(value, "direct value");
            }
            _ => panic!("section1 should be a string"),
        }

        // Check section2
        let section2_key = DocumentKey::Ident(Identifier::from_str("section2").unwrap());
        let section2_node = entries
            .iter()
            .find(|(k, _)| k == &section2_key)
            .map(|(_, id)| document.node(*id))
            .expect("section2 should exist");

        match &section2_node.content {
            NodeValue::I64 { value, .. } => {
                assert_eq!(*value, 123);
            }
            _ => panic!("section2 should be an integer"),
        }

        // Check section3
        let section3_key = DocumentKey::Ident(Identifier::from_str("section3").unwrap());
        let section3_node = entries
            .iter()
            .find(|(k, _)| k == &section3_key)
            .map(|(_, id)| document.node(*id))
            .expect("section3 should exist");

        match &section3_node.content {
            NodeValue::Bool { value, .. } => {
                assert!(value);
            }
            _ => panic!("section3 should be a boolean"),
        }

        // Check section4
        let section4_key = DocumentKey::Ident(Identifier::from_str("section4").unwrap());
        let section4_node = entries
            .iter()
            .find(|(k, _)| k == &section4_key)
            .map(|(_, id)| document.node(*id))
            .expect("section4 should exist");

        match &section4_node.content {
            NodeValue::Null { .. } => {
                // OK
            }
            _ => panic!("section4 should be null"),
        }
    } else {
        panic!("Root should be a map");
    }
}

#[test]
fn test_nested_path_direct_assignment() {
    // Test nested path direct assignment: @ path.to.field = value
    let input = r#"
@ config.database.host = "localhost"
@ config.database.port = 5432
@ config.server.enabled = true
"#;

    let tree = eure_parol::parse(input).expect("Parse should succeed");
    let mut visitor = ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor)
        .expect("Visit should succeed");

    let document = visitor.into_document();
    let root = document.root();

    if let NodeValue::Map { entries, .. } = &root.content {
        // Navigate to config
        let config_key = DocumentKey::Ident(Identifier::from_str("config").unwrap());
        let config_node = entries
            .iter()
            .find(|(k, _)| k == &config_key)
            .map(|(_, id)| document.node(*id))
            .expect("config should exist");

        if let NodeValue::Map {
            entries: config_entries,
            ..
        } = &config_node.content
        {
            // Check database
            let database_key = DocumentKey::Ident(Identifier::from_str("database").unwrap());
            let database_node = config_entries
                .iter()
                .find(|(k, _)| k == &database_key)
                .map(|(_, id)| document.node(*id))
                .expect("database should exist");

            if let NodeValue::Map {
                entries: db_entries,
                ..
            } = &database_node.content
            {
                // Check host
                let host_key = DocumentKey::Ident(Identifier::from_str("host").unwrap());
                let host_node = db_entries
                    .iter()
                    .find(|(k, _)| k == &host_key)
                    .map(|(_, id)| document.node(*id))
                    .expect("host should exist");

                match &host_node.content {
                    NodeValue::String { value, .. } => {
                        assert_eq!(value, "localhost");
                    }
                    _ => panic!("host should be a string"),
                }

                // Check port
                let port_key = DocumentKey::Ident(Identifier::from_str("port").unwrap());
                let port_node = db_entries
                    .iter()
                    .find(|(k, _)| k == &port_key)
                    .map(|(_, id)| document.node(*id))
                    .expect("port should exist");

                match &port_node.content {
                    NodeValue::I64 { value, .. } => {
                        assert_eq!(*value, 5432);
                    }
                    _ => panic!("port should be an integer"),
                }
            }

            // Check server
            let server_key = DocumentKey::Ident(Identifier::from_str("server").unwrap());
            let server_node = config_entries
                .iter()
                .find(|(k, _)| k == &server_key)
                .map(|(_, id)| document.node(*id))
                .expect("server should exist");

            if let NodeValue::Map {
                entries: server_entries,
                ..
            } = &server_node.content
            {
                let enabled_key = DocumentKey::Ident(Identifier::from_str("enabled").unwrap());
                let enabled_node = server_entries
                    .iter()
                    .find(|(k, _)| k == &enabled_key)
                    .map(|(_, id)| document.node(*id))
                    .expect("enabled should exist");

                match &enabled_node.content {
                    NodeValue::Bool { value, .. } => {
                        assert!(value);
                    }
                    _ => panic!("enabled should be a boolean"),
                }
            }
        }
    }
}

#[test]
fn test_array_index_direct_assignment() {
    // Test array index direct assignment: @ items[0] = value
    let input = r#"
items = []
@ items[0] = "first"
@ items[1] = "second"
@ items[2] = "third"
"#;

    let tree = eure_parol::parse(input).expect("Parse should succeed");
    let mut visitor = ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor)
        .expect("Visit should succeed");

    let document = visitor.into_document();
    let root = document.root();

    if let NodeValue::Map { entries, .. } = &root.content {
        let items_key = DocumentKey::Ident(Identifier::from_str("items").unwrap());
        let items_node = entries
            .iter()
            .find(|(k, _)| k == &items_key)
            .map(|(_, id)| id)
            .map(|id| document.node(*id))
            .expect("items should exist");

        if let NodeValue::Array {
            children: elements, ..
        } = &items_node.content
        {
            assert_eq!(elements.len(), 3, "Should have 3 items");

            // Check first item
            let first = document.node(elements[0]);
            match &first.content {
                NodeValue::String { value, .. } => {
                    assert_eq!(value, "first");
                }
                _ => panic!("First item should be a string"),
            }

            // Check second item
            let second = document.node(elements[1]);
            match &second.content {
                NodeValue::String { value, .. } => {
                    assert_eq!(value, "second");
                }
                _ => panic!("Second item should be a string"),
            }

            // Check third item
            let third = document.node(elements[2]);
            match &third.content {
                NodeValue::String { value, .. } => {
                    assert_eq!(value, "third");
                }
                _ => panic!("Third item should be a string"),
            }
        } else {
            panic!("items should be an array");
        }
    }
}

#[test]
fn test_array_append_direct_assignment() {
    // Test array append direct assignment: @ items[] = value
    let input = r#"
items = ["existing"]
@ items[] = "appended1"
@ items[] = "appended2"
"#;

    let tree = eure_parol::parse(input).expect("Parse should succeed");
    let mut visitor = ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor)
        .expect("Visit should succeed");

    let document = visitor.into_document();
    let root = document.root();

    if let NodeValue::Map { entries, .. } = &root.content {
        let items_key = DocumentKey::Ident(Identifier::from_str("items").unwrap());
        let items_node = entries
            .iter()
            .find(|(k, _)| k == &items_key)
            .map(|(_, id)| id)
            .map(|id| document.node(*id))
            .expect("items should exist");

        if let NodeValue::Array {
            children: elements, ..
        } = &items_node.content
        {
            assert_eq!(elements.len(), 3, "Should have 3 items total");

            // Check existing item
            let first = document.node(elements[0]);
            match &first.content {
                NodeValue::String { value, .. } => {
                    assert_eq!(value, "existing");
                }
                _ => panic!("First item should be a string"),
            }

            // Check appended items
            let second = document.node(elements[1]);
            match &second.content {
                NodeValue::String { value, .. } => {
                    assert_eq!(value, "appended1");
                }
                _ => panic!("Second item should be a string"),
            }

            let third = document.node(elements[2]);
            match &third.content {
                NodeValue::String { value, .. } => {
                    assert_eq!(value, "appended2");
                }
                _ => panic!("Third item should be a string"),
            }
        } else {
            panic!("items should be an array");
        }
    }
}

#[test]
fn test_extension_direct_assignment() {
    // Test extension direct assignment: @ field.$extension = value
    let input = r#"
@ user.$type = "Person"
@ user.$variant = "admin"
@ config.$$internal = true
"#;

    let tree = eure_parol::parse(input).expect("Parse should succeed");
    let mut visitor = ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor)
        .expect("Visit should succeed");

    let document = visitor.into_document();
    let root = document.root();

    if let NodeValue::Map { entries, .. } = &root.content {
        // Check user extensions
        let user_key = DocumentKey::Ident(Identifier::from_str("user").unwrap());
        let user_node = entries
            .iter()
            .find(|(k, _)| k == &user_key)
            .map(|(_, id)| id)
            .map(|id| document.node(*id))
            .expect("user should exist");

        // Check $type extension
        let type_ext = user_node
            .extensions
            .get(&Identifier::from_str("type").unwrap())
            .map(|id| document.node(*id))
            .expect("$type extension should exist");

        match &type_ext.content {
            NodeValue::String { value, .. } => {
                assert_eq!(value, "Person");
            }
            _ => panic!("$type should be a string"),
        }

        // Check $variant extension
        let variant_ext = user_node
            .extensions
            .get(&Identifier::from_str("variant").unwrap())
            .map(|id| document.node(*id))
            .expect("$variant extension should exist");

        match &variant_ext.content {
            NodeValue::String { value, .. } => {
                assert_eq!(value, "admin");
            }
            _ => panic!("$variant should be a string"),
        }

        // Check config meta-extension
        // Meta extensions are stored as regular entries with DocumentKey::MetaExtension
        let config_key = DocumentKey::Ident(Identifier::from_str("config").unwrap());
        let config_node = entries
            .iter()
            .find(|(k, _)| k == &config_key)
            .map(|(_, id)| id)
            .map(|id| document.node(*id))
            .expect("config should exist");

        // Meta-extensions are stored as entries in the Map, not in extensions
        if let NodeValue::Map {
            entries: config_entries,
            ..
        } = &config_node.content
        {
            let internal_key =
                DocumentKey::MetaExtension(Identifier::from_str("internal").unwrap());
            let internal_meta = config_entries
                .iter()
                .find(|(k, _)| k == &internal_key)
                .map(|(_, id)| document.node(*id))
                .expect("$$internal meta-extension should exist");

            match &internal_meta.content {
                NodeValue::Bool { value, .. } => {
                    assert!(value);
                }
                _ => panic!("$$internal should be a boolean"),
            }
        }
    }
}

#[test]
fn test_complex_value_direct_assignment() {
    // Test assignment of complex values (objects, arrays)
    let input = r#"
@ data.object = {key = "value", num = 42}
@ data.array = [1, 2, 3]
@ data.tuple = ("a", 2, true)
"#;

    let tree = eure_parol::parse(input).expect("Parse should succeed");
    let mut visitor = ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor)
        .expect("Visit should succeed");

    let document = visitor.into_document();
    let root = document.root();

    if let NodeValue::Map { entries, .. } = &root.content {
        let data_key = DocumentKey::Ident(Identifier::from_str("data").unwrap());
        let data_node = entries
            .iter()
            .find(|(k, _)| k == &data_key)
            .map(|(_, id)| id)
            .map(|id| document.node(*id))
            .expect("data should exist");

        if let NodeValue::Map {
            entries: data_entries,
            ..
        } = &data_node.content
        {
            // Check object
            let object_key = DocumentKey::Ident(Identifier::from_str("object").unwrap());
            let object_node = data_entries
                .iter()
                .find(|(k, _)| k == &object_key)
                .map(|(_, id)| id)
                .map(|id| document.node(*id))
                .expect("object should exist");

            if let NodeValue::Map {
                entries: obj_entries,
                ..
            } = &object_node.content
            {
                assert_eq!(obj_entries.len(), 2);

                let key_key = DocumentKey::Ident(Identifier::from_str("key").unwrap());
                let key_node = obj_entries
                    .iter()
                    .find(|(k, _)| k == &key_key)
                    .map(|(_, id)| id)
                    .map(|id| document.node(*id))
                    .expect("key should exist");

                match &key_node.content {
                    NodeValue::String { value, .. } => {
                        assert_eq!(value, "value");
                    }
                    _ => panic!("key should be a string"),
                }
            }

            // Check array
            let array_key = DocumentKey::Ident(Identifier::from_str("array").unwrap());
            let array_node = data_entries
                .iter()
                .find(|(k, _)| k == &array_key)
                .map(|(_, id)| id)
                .map(|id| document.node(*id))
                .expect("array should exist");

            if let NodeValue::Array {
                children: elements, ..
            } = &array_node.content
            {
                assert_eq!(elements.len(), 3);
            }

            // Check tuple
            let tuple_key = DocumentKey::Ident(Identifier::from_str("tuple").unwrap());
            let tuple_node = data_entries
                .iter()
                .find(|(k, _)| k == &tuple_key)
                .map(|(_, id)| id)
                .map(|id| document.node(*id))
                .expect("tuple should exist");

            if let NodeValue::Tuple {
                children: elements, ..
            } = &tuple_node.content
            {
                assert_eq!(elements.len(), 3);
            }
        }
    }
}

#[test]
fn test_mixed_section_styles() {
    // Test mixing direct assignment with traditional section syntax
    let input = r#"
@ config.name = "MyApp"

@ config.database {
    host = "localhost"
    port = 5432
}

@ config.cache = true
"#;

    let tree = eure_parol::parse(input).expect("Parse should succeed");
    let mut visitor = ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor)
        .expect("Visit should succeed");

    let document = visitor.into_document();
    let root = document.root();

    if let NodeValue::Map { entries, .. } = &root.content {
        let config_key = DocumentKey::Ident(Identifier::from_str("config").unwrap());
        let config_node = entries
            .iter()
            .find(|(k, _)| k == &config_key)
            .map(|(_, id)| id)
            .map(|id| document.node(*id))
            .expect("config should exist");

        if let NodeValue::Map {
            entries: config_entries,
            ..
        } = &config_node.content
        {
            // Check name (direct assignment)
            let name_key = DocumentKey::Ident(Identifier::from_str("name").unwrap());
            assert!(
                config_entries.iter().any(|(k, _)| k == &name_key),
                "name should exist"
            );

            // Check database (traditional section)
            let database_key = DocumentKey::Ident(Identifier::from_str("database").unwrap());
            let database_node = config_entries
                .iter()
                .find(|(k, _)| k == &database_key)
                .map(|(_, id)| id)
                .map(|id| document.node(*id))
                .expect("database should exist");

            if let NodeValue::Map {
                entries: db_entries,
                ..
            } = &database_node.content
            {
                assert!(
                    db_entries
                        .iter()
                        .any(|(k, _)| k
                            == &DocumentKey::Ident(Identifier::from_str("host").unwrap()))
                );
                assert!(
                    db_entries
                        .iter()
                        .any(|(k, _)| k
                            == &DocumentKey::Ident(Identifier::from_str("port").unwrap()))
                );
            }

            // Check cache (direct assignment after traditional section)
            let cache_key = DocumentKey::Ident(Identifier::from_str("cache").unwrap());
            assert!(
                config_entries.iter().any(|(k, _)| k == &cache_key),
                "cache should exist"
            );
        }
    }
}
