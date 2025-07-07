use eure_tree::prelude::*;
use eure_value::value::{Code, Value};

#[test]
fn test_inline_code_vs_typed_string() {
    // Test that inline code (programming languages) creates Value::Code
    let code_examples = vec![
        ("rust`let a = 1;`", "rust", "let a = 1;"),
        ("js`console.log('hello');`", "js", "console.log('hello');"),
        ("python`print('world')`", "python", "print('world')"),
        ("go`fmt.Println(\"test\")`", "go", "fmt.Println(\"test\")"),
    ];

    for (input, expected_lang, expected_content) in code_examples {
        let document = format!("test = {input}");
        let tree = eure_parol::parse(&document).unwrap_or_else(|_| panic!("Failed to parse: {input}"));

        let mut visitor = eure_tree::value_visitor::ValueVisitor::new(&document);
        tree.visit_from_root(&mut visitor)
            .unwrap_or_else(|_| panic!("Failed to visit tree for: {input}"));

        let doc = visitor.into_document();
        let eure_value = eure_tree::value_visitor::document_to_value(doc);
        if let Value::Map(map) = eure_value {
            let test_value = map
                .0
                .get(&eure_value::value::KeyCmpValue::String("test".to_string()))
                .unwrap();

            // Should be Code
            match test_value {
                Value::Code(Code { language, content }) => {
                    assert_eq!(language, expected_lang, "Wrong language for {input}");
                    assert_eq!(content, expected_content, "Wrong content for {input}");
                }
                _ => panic!("Expected Value::Code for {input}, got {test_value:?}"),
            }
        }
    }

    // Test that non-programming languages also create Value::Code
    let typed_string_examples = vec![
        ("regex`^[a-z]+$`", "regex", "^[a-z]+$"),
        ("email`test@example.com`", "email", "test@example.com"),
        ("url`https://example.com`", "url", "https://example.com"),
    ];

    for (input, expected_type, expected_value) in typed_string_examples {
        let document = format!("test = {input}");
        let tree = eure_parol::parse(&document).unwrap_or_else(|_| panic!("Failed to parse: {input}"));

        let mut visitor = eure_tree::value_visitor::ValueVisitor::new(&document);
        tree.visit_from_root(&mut visitor)
            .unwrap_or_else(|_| panic!("Failed to visit tree for: {input}"));

        let doc = visitor.into_document();
        let eure_value = eure_tree::value_visitor::document_to_value(doc);
        if let Value::Map(map) = eure_value {
            let test_value = map
                .0
                .get(&eure_value::value::KeyCmpValue::String("test".to_string()))
                .unwrap();

            // Should be Code (all named code creates Value::Code)
            match test_value {
                Value::Code(Code { language, content }) => {
                    assert_eq!(language, expected_type, "Wrong type for {input}");
                    assert_eq!(content, expected_value, "Wrong value for {input}");
                }
                _ => panic!(
                    "Expected Value::Code for {input}, got {test_value:?}"
                ),
            }
        }
    }
}

#[test]
fn test_code_block_still_works() {
    // Test that code blocks still work correctly
    // Test single code block first
    let document = r#"code1 = ```rust
fn main() {
    println!("Hello");
}
```"#;

    let tree = eure_parol::parse(document).expect("Failed to parse code blocks");

    let mut visitor = eure_tree::value_visitor::ValueVisitor::new(document);
    tree.visit_from_root(&mut visitor)
        .expect("Failed to visit tree");

    let doc = visitor.into_document();
    let eure_value = eure_tree::value_visitor::document_to_value(doc);
    if let Value::Map(map) = eure_value {
        // Check code1
        let code1 = map
            .0
            .get(&eure_value::value::KeyCmpValue::String("code1".to_string()))
            .unwrap();
        match code1 {
            Value::CodeBlock(Code { language, content }) => {
                assert_eq!(language, "rust");
                assert_eq!(content, "fn main() {\n    println!(\"Hello\");\n}");
            }
            _ => panic!("Expected Value::CodeBlock for code1, got {code1:?}"),
        }
    }
}

#[test]
fn test_simple_code_block() {
    // Test simple code block without language
    let document = r#"code = ```
plain code
```"#;

    let tree = eure_parol::parse(document).expect("Failed to parse code block");

    let mut visitor = eure_tree::value_visitor::ValueVisitor::new(document);
    tree.visit_from_root(&mut visitor)
        .expect("Failed to visit tree");

    let doc = visitor.into_document();
    let eure_value = eure_tree::value_visitor::document_to_value(doc);

    if let Value::Map(map) = eure_value {
        let code = map
            .0
            .get(&eure_value::value::KeyCmpValue::String("code".to_string()))
            .unwrap();
        match code {
            Value::CodeBlock(Code { language, content }) => {
                assert_eq!(language, "");
                assert_eq!(content, "plain code");
            }
            _ => panic!("Expected Value::CodeBlock, got {code:?}"),
        }
    }
}
