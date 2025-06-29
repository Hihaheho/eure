use eure_parol::parse;

fn main() {
    let tests = vec![
        ("Simple binding", r#"name = "test""#),
        ("Extension binding", r#"$type = "string""#),
        ("Path value", r#"type = .string"#),
        ("Section with binding", r#"@ section
name = "test""#),
        ("Section with extension", r#"@ section
$type = "string""#),
        ("Inline extension old", r#"name { $type = "string" } = "test""#),
        ("Inline extension new", r#"name {$type = "string"} = "test""#),
        ("Type def section", r#"@ $types.Name
$type = "string""#),
        ("Array value", r#"$type = ["string", "number"]"#),
        ("Object value", r#"$type = {}"#),
        ("Object with fields", r#"$type = { "name": "string" }"#),
        ("Section with brace", r#"@ section {
    name = "test"
}"#),
        ("Section nested", r#"@ section
@ subsection
name = "test""#),
        ("Section brace extension", r#"@ section {
    $type = "string"
}"#),
    ];
    
    for (name, input) in tests {
        println!("\n{}: ", name);
        match parse(input) {
            Ok(tree) => {
                println!("✓ OK");
                // Try to extract schema to see if it works
                use eure_schema::extract_schema;
                let schema = extract_schema(input, &tree);
                println!("  Types found: {}", schema.document_schema.types.len());
            },
            Err(e) => {
                let msg = format!("{:?}", e);
                // Extract just the first error cause
                if let Some(start) = msg.find("cause: \"") {
                    if let Some(end) = msg[start+8..].find("\"") {
                        println!("✗ FAILED: {}", &msg[start+8..start+8+end]);
                    } else {
                        println!("✗ FAILED");
                    }
                } else {
                    println!("✗ FAILED");
                }
            }
        }
    }
}