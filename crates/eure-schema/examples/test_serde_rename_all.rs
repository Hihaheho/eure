use eure_schema::*;

fn main() {
    let schema_doc = r#"
$serde.rename-all = "camelCase"

@ $types.Person
$type = .object
$serde.rename-all = "snake_case"
"#;
    let schema = extract_schema_from_value(schema_doc).expect("Failed to extract");

    // Check global rename-all
    println!(
        "Global rename-all: {:?}",
        schema.document_schema.serde_options.rename_all
    );

    // Check type-specific rename-all
    if let Some(person_type) =
        schema
            .document_schema
            .types
            .get(&eure_value::value::KeyCmpValue::String(
                "Person".to_string(),
            ))
    {
        println!("Person type serde options: {:?}", person_type.serde);
    } else {
        println!("Person type not found!");
    }
}
