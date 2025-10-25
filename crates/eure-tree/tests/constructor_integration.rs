use eure_tree::{
    CstNode, constructors::*, node_kind::TerminalKind, tree::ConcreteSyntaxTree,
    value_visitor::ValueVisitor,
};
use eure_value::value::{KeyCmpValue, Map, Value as EureValue};

/// Test that builds a complete CST from scratch using constructors
/// This demonstrates the full constructor API by building: answer = 42
#[test]
fn test_constructor_complete_tree() {
    // Build the tree using constructors
    let answer_token = terminals::ident("answer");
    let equals_token = terminals::bind();
    let forty_two_token = terminals::integer("42");

    // Build value nodes
    let integer_node = IntegerConstructor::builder()
        .integer(forty_two_token)
        .build()
        .build();

    let value_node = ValueConstructor::Integer(integer_node).build();

    // Build key components
    let ident_node = IdentConstructor::builder()
        .ident(answer_token)
        .build()
        .build();

    let key_base = KeyBaseConstructor::Ident(ident_node).build();
    let key_opt = KeyOptConstructor::builder().build().build();
    let key_node = KeyConstructor::builder()
        .key_base(key_base)
        .key_opt(key_opt)
        .build()
        .build();

    // Build keys
    let keys_list = KeysListConstructor::empty();
    let keys = KeysConstructor::builder()
        .key(key_node)
        .keys_list(keys_list)
        .build()
        .build();

    // Build binding
    let bind_node = BindConstructor::builder()
        .bind(equals_token)
        .build()
        .build();

    let value_binding = ValueBindingConstructor::builder()
        .bind(bind_node)
        .value(value_node)
        .build()
        .build();

    let binding_rhs = BindingRhsConstructor::ValueBinding(value_binding).build();
    let binding = BindingConstructor::builder()
        .keys(keys)
        .binding_rhs(binding_rhs)
        .build()
        .build();

    // Build document structure
    let eure_bindings = EureBindingsConstructor::builder()
        .binding(binding)
        .eure_bindings(EureBindingsConstructor::empty())
        .build()
        .build();

    let eure_sections = EureSectionsConstructor::empty();
    let eure = EureConstructor::builder()
        .eure_bindings(eure_bindings)
        .eure_sections(eure_sections)
        .build()
        .build();

    let root = RootConstructor::builder().eure(eure).build().build();

    // Create tree
    let mut tree = ConcreteSyntaxTree::new(CstNode::Terminal {
        kind: TerminalKind::Whitespace,
        data: eure_tree::tree::TerminalData::Dynamic(eure_tree::tree::DynamicTokenId(0)),
    });
    tree.insert_dynamic_terminal("");

    // Apply the root builder
    let root_id = root.into_builder().apply(&mut tree);
    tree.set_root(root_id);

    // Assert tree structure
    assert!(tree.node_data(root_id).is_some(), "Root node should exist");

    // Count nodes - we expect at least:
    // 1 Root, 1 Eure, 1 EureBindings, 1 Binding, 1 Keys, 1 Key, 1 KeyBase, 1 KeyOpt,
    // 1 Ident, 1 BindingRhs, 1 ValueBinding, 1 Value, 1 Integer
    // Plus terminals: answer, =, 42
    let mut node_count = 0;
    for i in 0..100 {
        if tree.node_data(eure_tree::tree::CstNodeId(i)).is_some() {
            node_count += 1;
        } else {
            break;
        }
    }

    assert!(
        node_count >= 16,
        "Expected at least 16 nodes, got {node_count}"
    );

    // Visit the tree to extract the value
    let mut visitor = ValueVisitor::new("");
    tree.visit_from_root(&mut visitor)
        .expect("Failed to visit tree");

    // Get the document value
    let doc = visitor.into_document();
    let document = eure_tree::value_visitor::document_to_value(doc);

    // Verify the value
    match document {
        EureValue::Map(Map(map)) => {
            assert_eq!(map.len(), 1, "Expected one binding");
            let answer_key = KeyCmpValue::String("answer".to_string());
            let answer_value = map.get(&answer_key).expect("Missing 'answer' key");
            match answer_value {
                EureValue::I64(42) => (), // Success!
                _ => panic!("Expected I64(42), got {answer_value:?}"),
            }
        }
        _ => panic!("Expected Map value, got {document:?}"),
    }
}

/// Test building an array: [1, 2, 3]
#[test]
fn test_constructor_array() {
    // Create terminals
    let lbracket = terminals::l_bracket();
    let one_token = terminals::integer("1");
    let comma1 = terminals::comma();
    let two_token = terminals::integer("2");
    let comma2 = terminals::comma();
    let three_token = terminals::integer("3");
    let rbracket = terminals::r_bracket();

    // Build integer values
    let one_int = IntegerConstructor::builder()
        .integer(one_token)
        .build()
        .build();
    let one_value = ValueConstructor::Integer(one_int).build();

    let two_int = IntegerConstructor::builder()
        .integer(two_token)
        .build()
        .build();
    let two_value = ValueConstructor::Integer(two_int).build();

    let three_int = IntegerConstructor::builder()
        .integer(three_token)
        .build()
        .build();
    let three_value = ValueConstructor::Integer(three_int).build();

    // Build array elements recursively
    // ArrayElements: Value [ ArrayElementsTail ]
    // ArrayElementsTail: Comma [ ArrayElements ]

    // Third element: just the value with no tail
    let elements3 = ArrayElementsConstructor::builder()
        .value(three_value)
        .array_elements_opt(ArrayElementsOptConstructor::builder().build().build())
        .build()
        .build();

    // Second element tail: comma + third element
    let comma2_node = CommaConstructor::builder().comma(comma2).build().build();

    let tail2 = ArrayElementsTailConstructor::builder()
        .comma(comma2_node)
        .array_elements_tail_opt(
            ArrayElementsTailOptConstructor::builder()
                .array_elements(elements3)
                .build()
                .build(),
        )
        .build()
        .build();

    // Second element: value + tail
    let elements2 = ArrayElementsConstructor::builder()
        .value(two_value)
        .array_elements_opt(
            ArrayElementsOptConstructor::builder()
                .array_elements_tail(tail2)
                .build()
                .build(),
        )
        .build()
        .build();

    // First element tail: comma + second element
    let comma1_node = CommaConstructor::builder().comma(comma1).build().build();

    let tail1 = ArrayElementsTailConstructor::builder()
        .comma(comma1_node)
        .array_elements_tail_opt(
            ArrayElementsTailOptConstructor::builder()
                .array_elements(elements2)
                .build()
                .build(),
        )
        .build()
        .build();

    // First element: value + tail
    let array_elements = ArrayElementsConstructor::builder()
        .value(one_value)
        .array_elements_opt(
            ArrayElementsOptConstructor::builder()
                .array_elements_tail(tail1)
                .build()
                .build(),
        )
        .build()
        .build();

    // Build array
    let array_begin = ArrayBeginConstructor::builder()
        .l_bracket(lbracket)
        .build()
        .build();

    let array_end = ArrayEndConstructor::builder()
        .r_bracket(rbracket)
        .build()
        .build();

    let array = ArrayConstructor::builder()
        .array_begin(array_begin)
        .array_opt(
            ArrayOptConstructor::builder()
                .array_elements(array_elements)
                .build()
                .build(),
        )
        .array_end(array_end)
        .build()
        .build();

    let value = ValueConstructor::Array(array).build();

    // Build minimal document structure
    let key = terminals::ident("arr");
    let key_opt = KeyOptConstructor::builder().build().build();
    let key_base =
        KeyBaseConstructor::Ident(IdentConstructor::builder().ident(key).build().build()).build();
    let key_node = KeyConstructor::builder()
        .key_base(key_base)
        .key_opt(key_opt)
        .build()
        .build();
    let keys_list = KeysListConstructor::empty();
    let keys = KeysConstructor::builder()
        .key(key_node)
        .keys_list(keys_list)
        .build()
        .build();
    let equals = terminals::bind();
    let bind_node = BindConstructor::builder().bind(equals).build().build();

    let value_binding = ValueBindingConstructor::builder()
        .bind(bind_node)
        .value(value)
        .build()
        .build();

    let binding_rhs = BindingRhsConstructor::ValueBinding(value_binding).build();
    let binding = BindingConstructor::builder()
        .keys(keys)
        .binding_rhs(binding_rhs)
        .build()
        .build();
    let eure_bindings = EureBindingsConstructor::builder()
        .binding(binding)
        .eure_bindings(EureBindingsConstructor::empty())
        .build()
        .build();
    let eure_sections = EureSectionsConstructor::empty();
    let eure = EureConstructor::builder()
        .eure_bindings(eure_bindings)
        .eure_sections(eure_sections)
        .build()
        .build();
    let root = RootConstructor::builder().eure(eure).build().build();

    // Create tree
    let mut tree = ConcreteSyntaxTree::new(CstNode::Terminal {
        kind: TerminalKind::Whitespace,
        data: eure_tree::tree::TerminalData::Dynamic(eure_tree::tree::DynamicTokenId(0)),
    });
    tree.insert_dynamic_terminal("");

    let root_id = root.into_builder().apply(&mut tree);
    tree.set_root(root_id);

    // Visit the tree to extract the value
    let mut visitor = ValueVisitor::new("");
    tree.visit_from_root(&mut visitor)
        .expect("Failed to visit tree");

    // Get the document value
    let doc = visitor.into_document();
    let document = eure_tree::value_visitor::document_to_value(doc);

    // Verify the value
    match document {
        EureValue::Map(Map(map)) => {
            assert_eq!(map.len(), 1, "Expected one binding");
            let arr_key = KeyCmpValue::String("arr".to_string());
            let arr_value = map.get(&arr_key).expect("Missing 'arr' key");
            match arr_value {
                EureValue::Array(arr) => {
                    assert_eq!(arr.0.len(), 3, "Expected array with 3 elements");
                    assert_eq!(arr.0[0], EureValue::I64(1));
                    assert_eq!(arr.0[1], EureValue::I64(2));
                    assert_eq!(arr.0[2], EureValue::I64(3));
                }
                _ => panic!("Expected Array value, got {arr_value:?}"),
            }
        }
        _ => panic!("Expected Map value, got {document:?}"),
    }
}

/// Test all value types
#[test]
fn test_constructor_all_value_types() {
    // Test each value type can be constructed
    #[allow(clippy::type_complexity)]
    let test_cases: Vec<(&str, Box<dyn Fn() -> ValueNode>)> = vec![
        (
            "null",
            Box::new(|| {
                let null_token = terminals::null();
                let null_node = NullConstructor::builder().null(null_token).build().build();
                ValueConstructor::Null(null_node).build()
            }),
        ),
        (
            "true",
            Box::new(|| {
                let true_token = terminals::r#true();
                let true_node = TrueConstructor::builder()
                    .r#true(true_token)
                    .build()
                    .build();
                let boolean_node = BooleanConstructor::True(true_node).build();
                ValueConstructor::Boolean(boolean_node).build()
            }),
        ),
        (
            "false",
            Box::new(|| {
                let false_token = terminals::r#false();
                let false_node = FalseConstructor::builder()
                    .r#false(false_token)
                    .build()
                    .build();
                let boolean_node = BooleanConstructor::False(false_node).build();
                ValueConstructor::Boolean(boolean_node).build()
            }),
        ),
        (
            "\"hello\"",
            Box::new(|| {
                let str_token = terminals::str("\"hello\"");
                let str_node = StrConstructor::builder().str(str_token).build().build();
                // Strings: Str { Continue Str }
                let strings_list = StringsListConstructor::empty();
                let strings_node = StringsConstructor::builder()
                    .str(str_node)
                    .strings_list(strings_list)
                    .build()
                    .build();
                ValueConstructor::Strings(strings_node).build()
            }),
        ),
        (
            "42",
            Box::new(|| {
                let int_token = terminals::integer("42");
                let int_node = IntegerConstructor::builder()
                    .integer(int_token)
                    .build()
                    .build();
                ValueConstructor::Integer(int_node).build()
            }),
        ),
    ];

    for (expected, constructor) in test_cases {
        // Build value
        let value = constructor();

        // Build minimal document
        let key = terminals::ident("val");
        let key_opt = KeyOptConstructor::builder().build().build();
        let key_base =
            KeyBaseConstructor::Ident(IdentConstructor::builder().ident(key).build().build())
                .build();
        let key_node = KeyConstructor::builder()
            .key_base(key_base)
            .key_opt(key_opt)
            .build()
            .build();
        let keys_list = KeysListConstructor::empty();
        let keys = KeysConstructor::builder()
            .key(key_node)
            .keys_list(keys_list)
            .build()
            .build();
        let equals = terminals::bind();
        let bind_node = BindConstructor::builder().bind(equals).build().build();

        let value_binding = ValueBindingConstructor::builder()
            .bind(bind_node)
            .value(value)
            .build()
            .build();

        let binding_rhs = BindingRhsConstructor::ValueBinding(value_binding).build();
        let binding = BindingConstructor::builder()
            .keys(keys)
            .binding_rhs(binding_rhs)
            .build()
            .build();
        let eure_bindings = EureBindingsConstructor::builder()
            .binding(binding)
            .eure_bindings(EureBindingsConstructor::empty())
            .build()
            .build();
        let eure_sections = EureSectionsConstructor::empty();
        let eure = EureConstructor::builder()
            .eure_bindings(eure_bindings)
            .eure_sections(eure_sections)
            .build()
            .build();
        let root = RootConstructor::builder().eure(eure).build().build();

        // Create tree
        let mut tree = ConcreteSyntaxTree::new(CstNode::Terminal {
            kind: TerminalKind::Whitespace,
            data: eure_tree::tree::TerminalData::Dynamic(eure_tree::tree::DynamicTokenId(0)),
        });
        tree.insert_dynamic_terminal("");

        let root_id = root.into_builder().apply(&mut tree);
        tree.set_root(root_id);

        // Visit the tree to extract the value
        let mut visitor = ValueVisitor::new("");
        tree.visit_from_root(&mut visitor)
            .expect("Failed to visit tree");

        // Get the document value
        let doc = visitor.into_document();
        let document = eure_tree::value_visitor::document_to_value(doc);

        // Verify the value
        match document {
            EureValue::Map(Map(map)) => {
                assert_eq!(map.len(), 1, "Expected one binding");
                let val_key = KeyCmpValue::String("val".to_string());
                let val_value = map.get(&val_key).expect("Missing 'val' key");

                // Check the value based on expected
                match (expected, val_value) {
                    ("null", EureValue::Null) => (), // Success!
                    ("true", EureValue::Bool(true)) => (),
                    ("false", EureValue::Bool(false)) => (),
                    ("\"hello\"", EureValue::String(s)) if s == "hello" => (),
                    ("42", EureValue::I64(42)) => (),
                    _ => panic!("Expected {expected} to produce matching value, got {val_value:?}"),
                }
            }
            _ => panic!("Expected Map value, got {document:?}"),
        }
    }
}

/// Test building nested objects
#[test]
fn test_constructor_nested_object() {
    // Build: obj = { name = "Alice", data = { age = 30 } }
    let lbrace1 = terminals::l_brace();
    let name_key = terminals::ident("name");
    let eq1 = terminals::bind();
    let alice = terminals::str("\"Alice\"");
    let comma = terminals::comma();
    let data_key = terminals::ident("data");
    let eq2 = terminals::bind();
    let lbrace2 = terminals::l_brace();
    let age_key = terminals::ident("age");
    let eq3 = terminals::bind();
    let thirty = terminals::integer("30");
    let rbrace2 = terminals::r_brace();
    let rbrace1 = terminals::r_brace();

    // Build inner object: { age = 30 }
    let age_ident = IdentConstructor::builder().ident(age_key).build().build();
    let age_key_base = KeyBaseConstructor::Ident(age_ident).build();
    let age_key_opt = KeyOptConstructor::builder().build().build();
    let age_key_node = KeyConstructor::builder()
        .key_base(age_key_base)
        .key_opt(age_key_opt)
        .build()
        .build();

    let thirty_int = IntegerConstructor::builder()
        .integer(thirty)
        .build()
        .build();
    let thirty_value = ValueConstructor::Integer(thirty_int).build();

    // Build inner object: { age = 30 }
    let inner_begin = BeginConstructor::builder().l_brace(lbrace2).build().build();

    let inner_end = EndConstructor::builder().r_brace(rbrace2).build().build();

    let bind3_node = BindConstructor::builder().bind(eq3).build().build();

    // ObjectOpt for the inner object (no comma after last item)
    let inner_object_opt = ObjectOptConstructor::builder().build().build();

    // Inner object list: age = 30 with empty recursion
    let inner_object_list = ObjectListConstructor::builder()
        .key(age_key_node)
        .bind(bind3_node)
        .value(thirty_value)
        .object_opt(inner_object_opt)
        .object_list(ObjectListConstructor::empty())
        .build()
        .build();

    let inner_object = ObjectConstructor::builder()
        .begin(inner_begin)
        .object_list(inner_object_list)
        .end(inner_end)
        .build()
        .build();
    let inner_value = ValueConstructor::Object(inner_object).build();

    // Build outer object items
    let name_ident = IdentConstructor::builder().ident(name_key).build().build();
    let name_key_base = KeyBaseConstructor::Ident(name_ident).build();
    let name_key_opt = KeyOptConstructor::builder().build().build();
    let name_key_node = KeyConstructor::builder()
        .key_base(name_key_base)
        .key_opt(name_key_opt)
        .build()
        .build();

    let alice_str = StrConstructor::builder().str(alice).build().build();
    // Build Strings from Str
    let alice_strings = StringsConstructor::builder()
        .str(alice_str)
        .strings_list(StringsListConstructor::empty())
        .build()
        .build();
    let alice_value = ValueConstructor::Strings(alice_strings).build();

    // Build outer object bindings
    let bind1_node = BindConstructor::builder().bind(eq1).build().build();

    let bind2_node = BindConstructor::builder().bind(eq2).build().build();

    // Data binding
    let data_ident = IdentConstructor::builder().ident(data_key).build().build();
    let data_key_base = KeyBaseConstructor::Ident(data_ident).build();
    let data_key_opt = KeyOptConstructor::builder().build().build();
    let data_key_node = KeyConstructor::builder()
        .key_base(data_key_base)
        .key_opt(data_key_opt)
        .build()
        .build();

    // Build outer object with recursive structure
    let comma_node = CommaConstructor::builder().comma(comma).build().build();

    // ObjectOpt with comma for name item
    let name_object_opt = ObjectOptConstructor::builder()
        .comma(comma_node)
        .build()
        .build();

    // ObjectOpt without comma for data item (last item)
    let data_object_opt = ObjectOptConstructor::builder().build().build();

    // Data object list: data = { ... } with empty recursion
    let data_object_list = ObjectListConstructor::builder()
        .key(data_key_node)
        .bind(bind2_node)
        .value(inner_value)
        .object_opt(data_object_opt)
        .object_list(ObjectListConstructor::empty())
        .build()
        .build();

    // Name object list: name = "Alice", followed by data object list
    let outer_object_list = ObjectListConstructor::builder()
        .key(name_key_node)
        .bind(bind1_node)
        .value(alice_value)
        .object_opt(name_object_opt)
        .object_list(data_object_list)
        .build()
        .build();

    let outer_begin = BeginConstructor::builder().l_brace(lbrace1).build().build();

    let outer_end = EndConstructor::builder().r_brace(rbrace1).build().build();

    let outer_object = ObjectConstructor::builder()
        .begin(outer_begin)
        .object_list(outer_object_list)
        .end(outer_end)
        .build()
        .build();

    let outer_value = ValueConstructor::Object(outer_object).build();

    // Build document
    let key = terminals::ident("obj");
    let key_opt = KeyOptConstructor::builder().build().build();
    let key_base =
        KeyBaseConstructor::Ident(IdentConstructor::builder().ident(key).build().build()).build();
    let key_node = KeyConstructor::builder()
        .key_base(key_base)
        .key_opt(key_opt)
        .build()
        .build();
    let keys_list = KeysListConstructor::empty();
    let keys = KeysConstructor::builder()
        .key(key_node)
        .keys_list(keys_list)
        .build()
        .build();
    let equals = terminals::bind();
    let bind_node = BindConstructor::builder().bind(equals).build().build();

    let value_binding = ValueBindingConstructor::builder()
        .bind(bind_node)
        .value(outer_value)
        .build()
        .build();

    let binding_rhs = BindingRhsConstructor::ValueBinding(value_binding).build();
    let binding = BindingConstructor::builder()
        .keys(keys)
        .binding_rhs(binding_rhs)
        .build()
        .build();
    let eure_bindings = EureBindingsConstructor::builder()
        .binding(binding)
        .eure_bindings(EureBindingsConstructor::empty())
        .build()
        .build();
    let eure_sections = EureSectionsConstructor::empty();
    let eure = EureConstructor::builder()
        .eure_bindings(eure_bindings)
        .eure_sections(eure_sections)
        .build()
        .build();
    let root = RootConstructor::builder().eure(eure).build().build();

    // Create tree
    let mut tree = ConcreteSyntaxTree::new(CstNode::Terminal {
        kind: TerminalKind::Whitespace,
        data: eure_tree::tree::TerminalData::Dynamic(eure_tree::tree::DynamicTokenId(0)),
    });
    tree.insert_dynamic_terminal("");

    let root_id = root.into_builder().apply(&mut tree);
    tree.set_root(root_id);

    // Count nodes - with nested object we expect many more
    let mut node_count = 0;
    for i in 0..200 {
        if tree.node_data(eure_tree::tree::CstNodeId(i)).is_some() {
            node_count += 1;
        } else {
            break;
        }
    }

    assert!(
        node_count >= 40,
        "Expected at least 40 nodes for nested object, got {node_count}"
    );

    // Visit the tree to extract the value
    let mut visitor = ValueVisitor::new("");
    tree.visit_from_root(&mut visitor)
        .expect("Failed to visit tree");

    // Get the document value
    let doc = visitor.into_document();
    let document = eure_tree::value_visitor::document_to_value(doc);

    // Verify the nested object structure
    match document {
        EureValue::Map(Map(map)) => {
            assert_eq!(map.len(), 1, "Expected one binding");
            let obj_key = KeyCmpValue::String("obj".to_string());
            let obj_value = map.get(&obj_key).expect("Missing 'obj' key");

            match obj_value {
                EureValue::Map(Map(obj_map)) => {
                    assert_eq!(obj_map.len(), 2, "Expected two fields in object");

                    // Check name field
                    let name_key = KeyCmpValue::String("name".to_string());
                    let name_value = obj_map.get(&name_key).expect("Missing 'name' key");
                    match name_value {
                        EureValue::String(s) if s == "Alice" => (),
                        _ => panic!("Expected String(\"Alice\"), got {name_value:?}"),
                    }

                    // Check data field
                    let data_key = KeyCmpValue::String("data".to_string());
                    let data_value = obj_map.get(&data_key).expect("Missing 'data' key");
                    match data_value {
                        EureValue::Map(Map(data_map)) => {
                            assert_eq!(data_map.len(), 1, "Expected one field in nested object");
                            let age_key = KeyCmpValue::String("age".to_string());
                            let age_value = data_map.get(&age_key).expect("Missing 'age' key");
                            match age_value {
                                EureValue::I64(30) => (),
                                _ => panic!("Expected I64(30), got {age_value:?}"),
                            }
                        }
                        _ => panic!("Expected Map for 'data', got {data_value:?}"),
                    }
                }
                _ => panic!("Expected Map value for 'obj', got {obj_value:?}"),
            }
        }
        _ => panic!("Expected Map value, got {document:?}"),
    }
}
