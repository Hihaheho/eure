use eure_editor_support::cst_path_extractor::CstPathExtractor;

// Helper to get CST reference from ParseResult
fn get_cst_ref(parse_result: &eure_parol::ParseResult) -> &eure_tree::Cst {
    match parse_result {
        eure_parol::ParseResult::Ok(cst) => cst,
        eure_parol::ParseResult::ErrWithCst { cst, .. } => cst,
    }
}

#[test]
fn test_nested_path_extraction() {
    let input = "@ a.b.c\nkey = ";
    let parse_result = eure_parol::parse_tolerant(input);

    // Extract path at the position after 'c'
    let mut extractor = CstPathExtractor::new(input.to_string(), 7); // Position after 'c'
    let path = extractor.extract_path(get_cst_ref(&parse_result));

    eprintln!("Extracted path: {path:?}");
    assert_eq!(path, vec!["a", "b", "c"], "Should extract full nested path");
}

#[test]
fn test_path_extraction_at_binding() {
    let input = "@ a.b\nkey = value";
    let parse_result = eure_parol::parse_tolerant(input);

    // Extract path at the binding position
    let mut extractor = CstPathExtractor::new(input.to_string(), 9); // Position at 'key'
    let path = extractor.extract_path(get_cst_ref(&parse_result));

    // When in a binding under section a.b, should get section path
    assert_eq!(
        path,
        vec!["a", "b"],
        "Should get section path when in binding"
    );
}
