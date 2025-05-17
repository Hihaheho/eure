use swon_fmt::fmt;
use swon_parol::parse;

#[test]
fn test_fmt_basic() {
    
    let input = r#"$swon{version:1.0}
"#;

    let mut cst = parse(input).unwrap();
    
    fmt(input, &mut cst).unwrap();
    
    let mut output = String::new();
    cst.write(input, &mut output).unwrap();
    
    assert!(output.contains("$swon {"));
    assert!(output.contains("version: 1.0"));
}

#[test]
fn test_fmt_indentation() {
    
    let input = r#"$swon{nested:{key:"value"}}
"#;

    let mut cst = parse(input).unwrap();
    
    fmt(input, &mut cst).unwrap();
    
    let mut output = String::new();
    cst.write(input, &mut output).unwrap();
    
    assert!(output.contains("$swon {"));
    assert!(output.contains("nested: {"));
    assert!(output.contains("    key: \"value\""));
}
