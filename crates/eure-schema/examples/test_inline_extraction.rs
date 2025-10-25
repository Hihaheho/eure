use eure_schema::*;

fn main() {
    let input = r#"
name = "Alice"
name.$type = .string

age = 30
age.$type = .number
"#;

    println!("Testing inline schema extraction...");

    // Extract schema
    match extract_schema_from_value(input) {
        Ok(extracted) => {
            println!("Extraction successful!");
            println!("Is pure schema: {}", extracted.is_pure_schema);
            println!(
                "Number of root fields: {}",
                extracted.document_schema.root.fields.len()
            );

            for (name, field) in &extracted.document_schema.root.fields {
                println!(
                    "Field '{:?}': type = {:?}, optional = {}",
                    name, field.type_expr, field.optional
                );
            }
        }
        Err(e) => {
            println!("Extraction failed: {:?}", e);
        }
    }
}
