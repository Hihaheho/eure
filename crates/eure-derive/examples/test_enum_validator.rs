use eure_derive::Eure;
use eure_schema::{
    DocumentSchema, FieldSchema, ObjectSchema, ToEureSchema, Type, validate_with_schema_value,
};
use serde::{Deserialize, Serialize};

#[derive(Eure, Serialize, Deserialize)]
enum Status {
    Success { message: String },
    Error { code: u32, message: String },
    Pending,
}

fn validate_document<T: ToEureSchema>(document: &str) -> Result<(), String> {
    // Parse the EURE document
    let parsed = match eure_parol::parse(document) {
        Ok(cst) => cst,
        Err(e) => return Err(format!("Parse error: {e:?}")),
    };

    // Get the schema
    let schema = T::eure_schema();

    // Create a document schema with the type schema as root
    let mut doc_schema = DocumentSchema::default();

    // Add the generated schema to the document schema
    if let Type::Object(obj_schema) = schema.type_expr {
        doc_schema.root = obj_schema;
    } else {
        // For non-object types like enums, wrap in a single field
        let mut root = ObjectSchema::default();
        root.fields.insert(
            eure_schema::KeyCmpValue::String("value".to_string()),
            schema,
        );
        doc_schema.root = root;
    }

    println!(
        "Document schema root fields: {:?}",
        doc_schema.root.fields.keys().collect::<Vec<_>>()
    );

    // Validate the document
    let errors = validate_with_schema_value(document, doc_schema);

    match errors {
        Ok(errors) if errors.is_empty() => Ok(()),
        Ok(errors) => {
            let error_messages: Vec<String> = errors
                .iter()
                .filter(|e| e.severity == eure_schema::Severity::Error)
                .map(|e| format!("{:?}", e.kind))
                .collect();
            Err(format!("Validation errors: {}", error_messages.join(", ")))
        }
        Err(e) => Err(format!("Schema error: {:?}", e)),
    }
}

fn main() {
    // Test document
    let success = r#"
type = "Success"
message = "Operation completed"
"#;

    match validate_document::<Status>(success) {
        Ok(_) => println!("Validation succeeded"),
        Err(e) => println!("Validation failed: {e}"),
    }

    // Let's also try the variant syntax
    let with_variant = r#"
$variant = "Success"
message = "Operation completed"
"#;

    match validate_document::<Status>(with_variant) {
        Ok(_) => println!("Variant validation succeeded"),
        Err(e) => println!("Variant validation failed: {e}"),
    }
}
