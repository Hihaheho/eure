use eure_tree::constructors::terminals;
use eure_tree::{CstNode, constructors::*, node_kind::TerminalKind, tree::ConcreteSyntaxTree};
use eure_value::value::{Array, Code, KeyCmpValue, Map, Value, Variant};
use indexmap::IndexMap;

/// Format a Value as EURE syntax using CST construction
pub fn format_eure(value: &Value) -> String {
    // Build the CST
    let eure_node = match value {
        Value::Map(Map(map)) if !map.is_empty() => build_eure_from_map(&Map(map.clone())),
        _ => {
            // For non-map values, create a single binding
            let key = terminals::ident("value");
            let key_opt = KeyOptConstructor::builder().build().build();

            let key_node = KeyConstructor::builder()
                .key_base(
                    KeyBaseConstructor::Ident(
                        IdentConstructor::builder().ident(key).build().build(),
                    )
                    .build(),
                )
                .key_opt(key_opt)
                .build()
                .build();
            let keys_list = KeysListConstructor::empty();
            let keys = KeysConstructor::builder()
                .key(key_node)
                .keys_list(keys_list)
                .build()
                .build();
            let value_node = build_value(value);
            let binding = build_value_binding(value_node);
            let binding_rhs = BindingRhsConstructor::ValueBinding(binding).build();
            let binding_node = BindingConstructor::builder()
                .keys(keys)
                .binding_rhs(binding_rhs)
                .build()
                .build();

            let eure_bindings = EureBindingsConstructor::builder()
                .binding(binding_node)
                .eure_bindings(EureBindingsConstructor::empty())
                .build()
                .build();
            let eure_sections = EureSectionsConstructor::empty();

            EureConstructor::builder()
                .eure_bindings(eure_bindings)
                .eure_sections(eure_sections)
                .build()
                .build()
        }
    };

    // Create Root node
    let root_node = RootConstructor::builder().eure(eure_node).build().build();

    // Create tree with a dummy initial node (will be replaced)
    // Note: ConcreteSyntaxTree requires an initial root node in its constructor.
    // We create a minimal whitespace terminal that will be immediately replaced
    // by the actual root. This is necessary because:
    // 1. The tree must have at least one node to be valid
    // 2. The builder pattern needs an existing tree to apply nodes to
    // 3. The actual root is built separately and then set as the new root
    // This approach ensures the tree is always in a valid state while allowing
    // flexible construction of the actual syntax tree.
    let mut tree = ConcreteSyntaxTree::new(CstNode::Terminal {
        kind: TerminalKind::Whitespace,
        data: eure_tree::tree::TerminalData::Dynamic(eure_tree::tree::DynamicTokenId(0)),
    });
    tree.insert_dynamic_terminal("");

    // Apply the builder from the root node
    let root_id = root_node.into_builder().apply(&mut tree);
    tree.set_root(root_id);

    // Debug: inspect tree structure before formatting
    if std::env::var("DEBUG_CST").is_ok() {
        eprintln!("Root ID: {root_id:?}");
        eprintln!("Tree root: {:?}", tree.root());

        // Check what the root node actually is
        if let Some(root_data) = tree.node_data(root_id) {
            eprintln!("Root node data: {root_data:?}");
        }

        // Get children of root
        let children: Vec<_> = tree.children(root_id).collect();
        eprintln!("Root children: {children:?}");
        for child_id in &children {
            if let Some(child_data) = tree.node_data(*child_id) {
                eprintln!("  Child {child_id:?}: {child_data:?}");
            }
        }

        let mut debug_buffer = String::new();
        match tree.inspect("", &mut debug_buffer) {
            Ok(_) => eprintln!("Tree structure before formatting:\n{debug_buffer}"),
            Err(e) => eprintln!("Error inspecting tree: {e:?}"),
        }
    }

    // Apply formatting
    let input = ""; // No input since we're building from scratch
    if let Err(e) = eure_fmt::fmt(input, &mut tree) {
        eprintln!("Warning: Failed to apply formatting: {e}");
    }

    // Debug: inspect tree structure after formatting
    if std::env::var("DEBUG_CST").is_ok() {
        let mut debug_buffer = String::new();
        tree.inspect("", &mut debug_buffer).unwrap();
        eprintln!("Tree structure after formatting:\n{debug_buffer}");
    }

    // Write tree to string
    let mut buffer = String::new();
    tree.write("", &mut buffer).unwrap();
    buffer
}

/// Format a Value as EURE bindings (for root-level objects)
pub fn format_eure_bindings(value: &Value) -> String {
    match value {
        Value::Map(_map) => format_eure(value),
        _ => format_eure(value),
    }
}

fn build_eure_from_map(map: &Map) -> EureNode {
    let bindings: Vec<_> = map
        .0
        .iter()
        .map(|(key, value)| {
            let keys = build_keys_for_key(key);
            let value_node = build_value(value);
            let binding = build_value_binding(value_node);
            let binding_rhs = BindingRhsConstructor::ValueBinding(binding).build();
            BindingConstructor::builder()
                .keys(keys)
                .binding_rhs(binding_rhs)
                .build()
                .build()
        })
        .collect();

    // Build bindings list recursively
    let mut eure_bindings = EureBindingsConstructor::empty();
    for binding in bindings.into_iter().rev() {
        eure_bindings = EureBindingsConstructor::builder()
            .binding(binding)
            .eure_bindings(eure_bindings)
            .build()
            .build();
    }

    let eure_sections = EureSectionsConstructor::empty();

    EureConstructor::builder()
        .eure_bindings(eure_bindings)
        .eure_sections(eure_sections)
        .build()
        .build()
}

fn build_keys_for_key(key: &KeyCmpValue) -> KeysNode {
    let key_base = match key {
        KeyCmpValue::String(s) => {
            if is_valid_identifier(s) {
                let ident = terminals::ident(s);
                KeyBaseConstructor::Ident(IdentConstructor::builder().ident(ident).build().build())
                    .build()
            } else {
                let str_token = terminals::str(&format!("\"{}\"", escape_string(s)));
                KeyBaseConstructor::Str(StrConstructor::builder().str(str_token).build().build())
                    .build()
            }
        }
        KeyCmpValue::I64(i) => {
            let int_token = terminals::integer(&i.to_string());
            KeyBaseConstructor::Integer(
                IntegerConstructor::builder()
                    .integer(int_token)
                    .build()
                    .build(),
            )
            .build()
        }
        KeyCmpValue::U64(u) => {
            let int_token = terminals::integer(&u.to_string());
            KeyBaseConstructor::Integer(
                IntegerConstructor::builder()
                    .integer(int_token)
                    .build()
                    .build(),
            )
            .build()
        }
        _ => {
            // Fallback for unsupported key types
            let str_token = terminals::str("\"<unsupported-key>\"");
            KeyBaseConstructor::Str(StrConstructor::builder().str(str_token).build().build())
                .build()
        }
    };

    let key_opt = KeyOptConstructor::builder().build().build();

    let key_node = KeyConstructor::builder()
        .key_base(key_base)
        .key_opt(key_opt)
        .build()
        .build();

    let keys_list = KeysListConstructor::empty();
    KeysConstructor::builder()
        .key(key_node)
        .keys_list(keys_list)
        .build()
        .build()
}

fn build_value_binding(value: ValueNode) -> ValueBindingNode {
    let bind_token = terminals::bind();
    let bind = BindConstructor::builder().bind(bind_token).build().build();
    ValueBindingConstructor::builder()
        .bind(bind)
        .value(value)
        .build()
        .build()
}

fn build_value(value: &Value) -> ValueNode {
    match value {
        Value::Null => {
            let null_token = terminals::null();
            let null_node = NullConstructor::builder().null(null_token).build().build();
            ValueConstructor::Null(null_node).build()
        }
        Value::Bool(b) => {
            if *b {
                let true_token = terminals::r#true();
                let true_node = TrueConstructor::builder()
                    .r#true(true_token)
                    .build()
                    .build();
                let bool_node = BooleanConstructor::True(true_node).build();
                ValueConstructor::Boolean(bool_node).build()
            } else {
                let false_token = terminals::r#false();
                let false_node = FalseConstructor::builder()
                    .r#false(false_token)
                    .build()
                    .build();
                let bool_node = BooleanConstructor::False(false_node).build();
                ValueConstructor::Boolean(bool_node).build()
            }
        }
        Value::I64(i) => build_integer_value(&i.to_string()),
        Value::U64(u) => build_integer_value(&u.to_string()),
        Value::F32(f) => build_integer_value(&format!("{f}")),
        Value::F64(f) => build_integer_value(&format!("{f}")),
        Value::String(s) => {
            let str_token = terminals::str(&format!("\"{}\"", escape_string(s)));
            let str_node = StrConstructor::builder().str(str_token).build().build();
            let strings_list = StringsListConstructor::empty();
            let strings_node = StringsConstructor::builder()
                .str(str_node)
                .strings_list(strings_list)
                .build()
                .build();
            ValueConstructor::Strings(strings_node).build()
        }
        Value::Code(Code { language, content }) => {
            // Build named code using the named_code terminal
            let named_code = terminals::named_code(&format!("{language}`{content}`"));
            let named_code_node = NamedCodeConstructor::builder()
                .named_code(named_code)
                .build()
                .build();
            ValueConstructor::NamedCode(named_code_node).build()
        }
        Value::CodeBlock(Code { language, content }) => {
            let code_block = terminals::code_block(&format!("```{language}\n{content}\n```"));
            let code_node = CodeBlockConstructor::builder()
                .code_block(code_block)
                .build()
                .build();
            ValueConstructor::CodeBlock(code_node).build()
        }
        Value::Array(Array(values)) => build_array_value(values),
        Value::Map(Map(map)) => {
            let index_map: IndexMap<_, _> =
                map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            build_object_value(&index_map)
        }
        Value::Tuple(tuple) => {
            // Build tuple using the new tuple literal syntax: (value1, value2, value3)
            build_tuple_value(&tuple.0)
        }
        Value::Variant(Variant { tag, content }) => {
            // Build as an object with special $variant key
            let mut map = IndexMap::new();
            map.insert(
                KeyCmpValue::String("$variant".to_string()),
                Value::String(tag.clone()),
            );
            map.insert(
                KeyCmpValue::String("content".to_string()),
                content.as_ref().clone(),
            );
            build_object_value(&map)
        }
        Value::Unit => {
            // Represent unit as null
            let null_token = terminals::null();
            let null_node = NullConstructor::builder().null(null_token).build().build();
            ValueConstructor::Null(null_node).build()
        }
        Value::Path(path) => {
            // Build path value with dot notation
            build_path_value(path)
        }
        Value::Hole => {
            // Build hole value using the exclamation mark
            let hole_token = terminals::hole();
            let hole_node = HoleConstructor::builder().hole(hole_token).build().build();
            ValueConstructor::Hole(hole_node).build()
        }
    }
}

fn build_integer_value(text: &str) -> ValueNode {
    let int_token = terminals::integer(text);
    let int_node = IntegerConstructor::builder()
        .integer(int_token)
        .build()
        .build();
    ValueConstructor::Integer(int_node).build()
}

fn build_tuple_value(values: &[Value]) -> ValueNode {
    let lparen = terminals::l_paren();
    let l_paren_node = LParenConstructor::builder().l_paren(lparen).build().build();

    let tuple_opt = if values.is_empty() {
        TupleOptConstructor::builder().build().build()
    } else {
        // Build tuple elements recursively from right to left
        let mut tuple_elements_opt = TupleElementsOptConstructor::builder().build().build();

        for (_idx, value) in values.iter().enumerate().skip(1).rev() {
            let comma = terminals::comma();
            let comma_node = CommaConstructor::builder().comma(comma).build().build();

            let value_node = build_value(value);

            // Build tuple elements for this value
            let elements = TupleElementsConstructor::builder()
                .value(value_node)
                .tuple_elements_opt(tuple_elements_opt)
                .build()
                .build();

            // Build tail with comma and elements
            let tail = TupleElementsTailConstructor::builder()
                .comma(comma_node)
                .tuple_elements_tail_opt(
                    TupleElementsTailOptConstructor::builder()
                        .tuple_elements(elements)
                        .build()
                        .build(),
                )
                .build()
                .build();

            // Update tuple_elements_opt for next iteration
            tuple_elements_opt = TupleElementsOptConstructor::builder()
                .tuple_elements_tail(tail)
                .build()
                .build();
        }

        // Build the first element
        let first_value = build_value(&values[0]);
        let elements = TupleElementsConstructor::builder()
            .value(first_value)
            .tuple_elements_opt(tuple_elements_opt)
            .build()
            .build();

        TupleOptConstructor::builder()
            .tuple_elements(elements)
            .build()
            .build()
    };

    let rparen = terminals::r_paren();
    let r_paren_node = RParenConstructor::builder().r_paren(rparen).build().build();

    let tuple_node = TupleConstructor::builder()
        .l_paren(l_paren_node)
        .tuple_opt(tuple_opt)
        .r_paren(r_paren_node)
        .build()
        .build();

    ValueConstructor::Tuple(tuple_node).build()
}

fn build_array_value(values: &[Value]) -> ValueNode {
    let l_bracket = terminals::l_bracket();
    let array_begin = ArrayBeginConstructor::builder()
        .l_bracket(l_bracket)
        .build()
        .build();

    let array_opt = if values.is_empty() {
        ArrayOptConstructor::builder().build().build()
    } else {
        // Build array elements recursively from right to left
        let mut array_elements_opt = ArrayElementsOptConstructor::builder().build().build();

        for (_idx, value) in values.iter().enumerate().skip(1).rev() {
            let comma = terminals::comma();
            let comma_node = CommaConstructor::builder().comma(comma).build().build();

            let value_node = build_value(value);

            // Build array elements for this value
            let elements = ArrayElementsConstructor::builder()
                .value(value_node)
                .array_elements_opt(array_elements_opt)
                .build()
                .build();

            // Build tail with comma and elements
            let tail = ArrayElementsTailConstructor::builder()
                .comma(comma_node)
                .array_elements_tail_opt(
                    ArrayElementsTailOptConstructor::builder()
                        .array_elements(elements)
                        .build()
                        .build(),
                )
                .build()
                .build();

            // Update array_elements_opt for next iteration
            array_elements_opt = ArrayElementsOptConstructor::builder()
                .array_elements_tail(tail)
                .build()
                .build();
        }

        // Build the first element
        let first_value = build_value(&values[0]);
        let elements = ArrayElementsConstructor::builder()
            .value(first_value)
            .array_elements_opt(array_elements_opt)
            .build()
            .build();

        ArrayOptConstructor::builder()
            .array_elements(elements)
            .build()
            .build()
    };

    let r_bracket = terminals::r_bracket();
    let array_end = ArrayEndConstructor::builder()
        .r_bracket(r_bracket)
        .build()
        .build();

    let array_node = ArrayConstructor::builder()
        .array_begin(array_begin)
        .array_opt(array_opt)
        .array_end(array_end)
        .build()
        .build();

    ValueConstructor::Array(array_node).build()
}

fn build_object_value(map: &IndexMap<KeyCmpValue, Value>) -> ValueNode {
    let l_brace = terminals::l_brace();
    let begin = BeginConstructor::builder().l_brace(l_brace).build().build();

    // Build object list recursively
    let mut object_list = ObjectListConstructor::empty();

    for (idx, (key, value)) in map.iter().enumerate().rev() {
        // Build the key
        let key_base = match key {
            KeyCmpValue::String(s) => {
                if is_valid_identifier(s) {
                    let ident = terminals::ident(s);
                    KeyBaseConstructor::Ident(
                        IdentConstructor::builder().ident(ident).build().build(),
                    )
                    .build()
                } else {
                    let str_token = terminals::str(&format!("\"{}\"", escape_string(s)));
                    KeyBaseConstructor::Str(
                        StrConstructor::builder().str(str_token).build().build(),
                    )
                    .build()
                }
            }
            KeyCmpValue::I64(i) => {
                let int_token = terminals::integer(&i.to_string());
                KeyBaseConstructor::Integer(
                    IntegerConstructor::builder()
                        .integer(int_token)
                        .build()
                        .build(),
                )
                .build()
            }
            KeyCmpValue::U64(u) => {
                let int_token = terminals::integer(&u.to_string());
                KeyBaseConstructor::Integer(
                    IntegerConstructor::builder()
                        .integer(int_token)
                        .build()
                        .build(),
                )
                .build()
            }
            _ => {
                let str_token = terminals::str("\"<unsupported-key>\"");
                KeyBaseConstructor::Str(StrConstructor::builder().str(str_token).build().build())
                    .build()
            }
        };

        let key_opt = KeyOptConstructor::builder().build().build();

        let key_node = KeyConstructor::builder()
            .key_base(key_base)
            .key_opt(key_opt)
            .build()
            .build();

        // Build the bind token
        let bind_token = terminals::bind();
        let bind = BindConstructor::builder().bind(bind_token).build().build();

        // Build the value
        let value_node = build_value(value);

        // Build comma if not the last item
        let object_opt = if idx > 0 {
            let comma = terminals::comma();
            let comma_node = CommaConstructor::builder().comma(comma).build().build();
            ObjectOptConstructor::builder()
                .comma(comma_node)
                .build()
                .build()
        } else {
            ObjectOptConstructor::builder().build().build()
        };

        object_list = ObjectListConstructor::builder()
            .key(key_node)
            .bind(bind)
            .value(value_node)
            .object_opt(object_opt)
            .object_list(object_list)
            .build()
            .build();
    }

    let r_brace = terminals::r_brace();
    let end = EndConstructor::builder().r_brace(r_brace).build().build();

    let object_node = ObjectConstructor::builder()
        .begin(begin)
        .object_list(object_list)
        .end(end)
        .build()
        .build();

    ValueConstructor::Object(object_node).build()
}

fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    if let Some(first) = chars.next()
        && !first.is_alphabetic()
        && first != '_'
        && first != '$'
    {
        return false;
    }
    chars.all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '$')
}

fn escape_string(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '"' => "\\\"".to_string(),
            '\\' => "\\\\".to_string(),
            '\n' => "\\n".to_string(),
            '\r' => "\\r".to_string(),
            '\t' => "\\t".to_string(),
            c => c.to_string(),
        })
        .collect()
}

fn build_path_value(path: &eure_value::value::Path) -> ValueNode {
    use eure_value::value::PathSegment;

    // Build the dot token
    let dot_token = terminals::dot();
    let dot = DotConstructor::builder().dot(dot_token).build().build();

    // Build keys from path segments
    let mut keys_list = KeysListConstructor::empty();

    // Process segments in reverse order for proper list building
    for segment in path.0.iter().skip(1).rev() {
        let (key_base, array_marker_opt) = match segment {
            PathSegment::Ident(id) => {
                let ident = terminals::ident(id.as_ref());
                (
                    KeyBaseConstructor::Ident(
                        IdentConstructor::builder().ident(ident).build().build(),
                    )
                    .build(),
                    None,
                )
            }
            PathSegment::Extension(id) => {
                let ext_token = terminals::dollar();
                let ext_node = ExtConstructor::builder().dollar(ext_token).build().build();
                let ident = terminals::ident(id.as_ref());
                let ext_namespace = ExtensionNameSpaceConstructor::builder()
                    .ext(ext_node)
                    .ident(IdentConstructor::builder().ident(ident).build().build())
                    .build()
                    .build();
                (
                    KeyBaseConstructor::ExtensionNameSpace(ext_namespace).build(),
                    None,
                )
            }
            PathSegment::MetaExt(id) => {
                let meta_ext_token = terminals::dollar_dollar();
                let meta_ext_node = MetaExtConstructor::builder()
                    .dollar_dollar(meta_ext_token)
                    .build()
                    .build();
                let ident = terminals::ident(id.as_ref());
                let meta_ext_key = MetaExtKeyConstructor::builder()
                    .meta_ext(meta_ext_node)
                    .ident(IdentConstructor::builder().ident(ident).build().build())
                    .build()
                    .build();
                (KeyBaseConstructor::MetaExtKey(meta_ext_key).build(), None)
            }
            PathSegment::Value(v) => {
                let key_base = match v {
                    KeyCmpValue::String(s) => {
                        if is_valid_identifier(s) {
                            let ident = terminals::ident(s);
                            KeyBaseConstructor::Ident(
                                IdentConstructor::builder().ident(ident).build().build(),
                            )
                            .build()
                        } else {
                            let str_token = terminals::str(&format!("\"{}\"", escape_string(s)));
                            KeyBaseConstructor::Str(
                                StrConstructor::builder().str(str_token).build().build(),
                            )
                            .build()
                        }
                    }
                    KeyCmpValue::I64(i) => {
                        let int_token = terminals::integer(&i.to_string());
                        KeyBaseConstructor::Integer(
                            IntegerConstructor::builder()
                                .integer(int_token)
                                .build()
                                .build(),
                        )
                        .build()
                    }
                    KeyCmpValue::U64(u) => {
                        let int_token = terminals::integer(&u.to_string());
                        KeyBaseConstructor::Integer(
                            IntegerConstructor::builder()
                                .integer(int_token)
                                .build()
                                .build(),
                        )
                        .build()
                    }
                    KeyCmpValue::Bool(b) => {
                        if *b {
                            KeyBaseConstructor::True(
                                TrueConstructor::builder()
                                    .r#true(terminals::r#true())
                                    .build()
                                    .build(),
                            )
                            .build()
                        } else {
                            KeyBaseConstructor::False(
                                FalseConstructor::builder()
                                    .r#false(terminals::r#false())
                                    .build()
                                    .build(),
                            )
                            .build()
                        }
                    }
                    KeyCmpValue::Null => KeyBaseConstructor::Null(
                        NullConstructor::builder()
                            .null(terminals::null())
                            .build()
                            .build(),
                    )
                    .build(),
                    KeyCmpValue::Unit => {
                        // Unit is not a valid key, use null as fallback
                        KeyBaseConstructor::Null(
                            NullConstructor::builder()
                                .null(terminals::null())
                                .build()
                                .build(),
                        )
                        .build()
                    }
                    KeyCmpValue::Tuple(_) => {
                        todo!()
                    }
                    KeyCmpValue::MetaExtension(_) => {
                        todo!("This must be serialization error, not supported type to serialize")
                    }
                    KeyCmpValue::Hole => {
                        todo!("This must be serialization error, not supported type to serialize")
                    }
                };
                (key_base, None)
            }
            PathSegment::Array { key, index } => {
                // Build key with array marker
                let key_base = match key {
                    Value::String(s) if is_valid_identifier(s) => {
                        let ident = terminals::ident(s);
                        KeyBaseConstructor::Ident(
                            IdentConstructor::builder().ident(ident).build().build(),
                        )
                        .build()
                    }
                    Value::String(s) => {
                        let str_token = terminals::str(&format!("\"{}\"", escape_string(s)));
                        KeyBaseConstructor::Str(
                            StrConstructor::builder().str(str_token).build().build(),
                        )
                        .build()
                    }
                    Value::I64(i) => {
                        let int_token = terminals::integer(&i.to_string());
                        KeyBaseConstructor::Integer(
                            IntegerConstructor::builder()
                                .integer(int_token)
                                .build()
                                .build(),
                        )
                        .build()
                    }
                    Value::U64(u) => {
                        let int_token = terminals::integer(&u.to_string());
                        KeyBaseConstructor::Integer(
                            IntegerConstructor::builder()
                                .integer(int_token)
                                .build()
                                .build(),
                        )
                        .build()
                    }
                    _ => {
                        // For other types, convert to string representation
                        let str_representation = match key {
                            Value::Bool(b) => b.to_string(),
                            Value::Null => "null".to_string(),
                            _ => "<complex-value>".to_string(),
                        };
                        let str_token = terminals::str(&format!("\"{str_representation}\""));
                        KeyBaseConstructor::Str(
                            StrConstructor::builder().str(str_token).build().build(),
                        )
                        .build()
                    }
                };

                // Build array marker with optional index
                let array_begin = ArrayBeginConstructor::builder()
                    .l_bracket(terminals::l_bracket())
                    .build()
                    .build();

                let array_marker_opt = if let Some(idx) = index {
                    // Build index value
                    let index_str = match idx {
                        Value::I64(i) => i.to_string(),
                        Value::U64(u) => u.to_string(),
                        _ => "0".to_string(), // Default to 0 for non-numeric indices
                    };
                    let integer_token = terminals::integer(&index_str);
                    let integer_node = IntegerConstructor::builder()
                        .integer(integer_token)
                        .build()
                        .build();
                    ArrayMarkerOptConstructor::builder()
                        .integer(integer_node)
                        .build()
                        .build()
                } else {
                    // No index specified
                    ArrayMarkerOptConstructor::builder().build().build()
                };

                let array_end = ArrayEndConstructor::builder()
                    .r_bracket(terminals::r_bracket())
                    .build()
                    .build();

                let array_marker = ArrayMarkerConstructor::builder()
                    .array_begin(array_begin)
                    .array_marker_opt(array_marker_opt)
                    .array_end(array_end)
                    .build()
                    .build();

                // Return key_base and array marker separately
                (key_base, Some(array_marker))
            }
            PathSegment::TupleIndex(idx) => {
                // Tuple indices are represented as simple integer keys
                let integer = terminals::integer(&idx.to_string());
                let integer_node = IntegerConstructor::builder()
                    .integer(integer)
                    .build()
                    .build();
                let key_base = KeyBaseConstructor::Integer(integer_node).build();
                (key_base, None)
            }
        };

        // Build the key with optional array marker
        let key = if let Some(array_marker) = array_marker_opt {
            KeyConstructor::builder()
                .key_base(key_base)
                .key_opt(
                    KeyOptConstructor::builder()
                        .array_marker(array_marker)
                        .build()
                        .build(),
                )
                .build()
                .build()
        } else {
            KeyConstructor::builder()
                .key_base(key_base)
                .key_opt(KeyOptConstructor::builder().build().build())
                .build()
                .build()
        };

        let dot_for_list = terminals::dot();
        let dot_node = DotConstructor::builder().dot(dot_for_list).build().build();

        keys_list = KeysListConstructor::builder()
            .dot(dot_node)
            .key(key)
            .keys_list(keys_list)
            .build()
            .build();
    }

    // Build the first key
    if let Some(first_segment) = path.0.first() {
        let first_key_base = match first_segment {
            PathSegment::Ident(id) => {
                let ident = terminals::ident(id.as_ref());
                KeyBaseConstructor::Ident(IdentConstructor::builder().ident(ident).build().build())
                    .build()
            }
            _ => {
                // Paths should typically start with an identifier
                let ident = terminals::ident("path");
                KeyBaseConstructor::Ident(IdentConstructor::builder().ident(ident).build().build())
                    .build()
            }
        };

        let key_opt = KeyOptConstructor::builder().build().build();
        let first_key = KeyConstructor::builder()
            .key_base(first_key_base)
            .key_opt(key_opt)
            .build()
            .build();

        let keys = KeysConstructor::builder()
            .key(first_key)
            .keys_list(keys_list)
            .build()
            .build();

        let path_node = PathConstructor::builder()
            .dot(dot)
            .keys(keys)
            .build()
            .build();

        ValueConstructor::Path(path_node).build()
    } else {
        // Empty path - just a dot
        let keys = KeysConstructor::builder()
            .key(
                KeyConstructor::builder()
                    .key_base(
                        KeyBaseConstructor::Ident(
                            IdentConstructor::builder()
                                .ident(terminals::ident("empty"))
                                .build()
                                .build(),
                        )
                        .build(),
                    )
                    .key_opt(KeyOptConstructor::builder().build().build())
                    .build()
                    .build(),
            )
            .keys_list(KeysListConstructor::empty())
            .build()
            .build();

        let path_node = PathConstructor::builder()
            .dot(dot)
            .keys(keys)
            .build()
            .build();

        ValueConstructor::Path(path_node).build()
    }
}
