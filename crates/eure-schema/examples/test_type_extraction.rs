use eure_schema::*;

fn main() {
    let schema_doc = r#"
@ $types.Person
$type = .object
@ $types.Person.name
$type = .string
@ $types.Person.age
$type = .number
$optional = true
"#;

    println!("Extracting schema from: {}", schema_doc);

    match extract_schema_from_value(schema_doc) {
        Ok(result) => {
            println!("\nExtraction successful!");
            println!("Is pure schema: {}", result.is_pure_schema);
            println!(
                "Document schema types: {:?}",
                result.document_schema.types.keys().collect::<Vec<_>>()
            );

            if let Some(person_type) =
                result
                    .document_schema
                    .types
                    .get(&eure_value::value::KeyCmpValue::String(
                        "Person".to_string(),
                    ))
            {
                println!("\nPerson type found!");
                println!("Type expression: {:?}", person_type.type_expr);

                if let Type::Object(obj) = &person_type.type_expr {
                    println!("Fields: {:?}", obj.fields.keys().collect::<Vec<_>>());
                }
            } else {
                println!("Person type NOT found!");
            }
        }
        Err(e) => {
            println!("Error extracting schema: {:?}", e);
        }
    }
}
