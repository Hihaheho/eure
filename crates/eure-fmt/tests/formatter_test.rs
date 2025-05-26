use eure_fmt::fmt;
use eure_parol::parse;

#[test]
fn test_fmt_basic() {
    
    let input = r#"$eure{version:1.0}
"#;

    let mut cst = parse(input).unwrap();
    
    fmt(input, &mut cst).unwrap();
    
    let mut output = String::new();
    cst.write(input, &mut output).unwrap();
    
    assert!(output.contains("$eure {"));
    assert!(output.contains("version: 1.0"));
}

#[test]
fn test_fmt_indentation() {
    
    let input = r#"$eure{nested:{key:"value"}}
"#;

    let mut cst = parse(input).unwrap();
    
    fmt(input, &mut cst).unwrap();
    
    let mut output = String::new();
    cst.write(input, &mut output).unwrap();
    
    assert!(output.contains("$eure {"));
    assert!(output.contains("nested: {"));
    assert!(output.contains("    key: \"value\""));
}
