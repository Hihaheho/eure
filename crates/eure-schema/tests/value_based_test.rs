use eure_schema::{extract_schema_from_value, validate_with_schema_value};
use eure_value::value::ObjectKey;

#[test]
fn test_value_based_schema_extraction() {
    let schema_doc = r#"
# Root-level field definitions come BEFORE sections
users.$array = .$types.User

# Define types in a types section
@ $types.User
name.$type = .string
age.$type = .number
"#;

    // Extract schema
    let extracted =
        extract_schema_from_value(schema_doc).expect("Schema extraction should succeed");

    println!(
        "Extracted types: {:?}",
        extracted.document_schema.types.keys().collect::<Vec<_>>()
    );
    println!(
        "Root fields: {:?}",
        extracted
            .document_schema
            .root
            .fields
            .keys()
            .collect::<Vec<_>>()
    );
    println!("Is pure schema: {}", extracted.is_pure_schema);

    // Debug why users field is missing
    if extracted.document_schema.root.fields.is_empty() {
        println!("No root fields found!");
    }

    assert!(
        extracted.is_pure_schema,
        "Document should be recognized as pure schema"
    );
    assert!(
        extracted
            .document_schema
            .types
            .contains_key(&ObjectKey::String("User".to_string()))
    );

    // Check that User type is properly defined
    let user_type = &extracted.document_schema.types[&ObjectKey::String("User".to_string())];
    match &user_type.type_expr {
        eure_schema::Type::Object(obj) => {
            assert!(
                obj.fields
                    .contains_key(&ObjectKey::String("name".to_string()))
            );
            assert!(
                obj.fields
                    .contains_key(&ObjectKey::String("age".to_string()))
            );
        }
        _ => panic!("User type should be an Object"),
    }

    // Check root schema has users array field
    assert!(
        extracted
            .document_schema
            .root
            .fields
            .contains_key(&ObjectKey::String("users".to_string()))
    );
    let users_field = &extracted
        .document_schema
        .root
        .fields
        .get(&ObjectKey::String("users".to_string()))
        .unwrap();
    println!("Users field type: {:?}", users_field.type_expr);
    match &users_field.type_expr {
        eure_schema::Type::Array(elem_type) => match elem_type.as_ref() {
            eure_schema::Type::TypeRef(name) => assert_eq!(name.to_string(), "User"),
            _ => panic!("Array element should be a TypeRef to User"),
        },
        _ => panic!("users field should be an Array"),
    }
}

#[test]
fn test_value_based_validation() {
    let schema_doc = r#"
# Root-level field definitions come BEFORE sections
users.$array = .$types.User

# Define types in a types section
@ $types.User
name.$type = .string
age.$type = .number
"#;

    let valid_doc = r#"
@ users[]
name = "Alice"
age = 30

@ users[]
name = "Bob"
age = 25
"#;

    let invalid_doc = r#"
@ users[]
name = "Alice"
age = "thirty"  # Wrong type - should be number

@ users[]
name = 123  # Wrong type - should be string
age = 25
"#;

    // Extract schema
    let extracted =
        extract_schema_from_value(schema_doc).expect("Schema extraction should succeed");

    // Validate valid document
    let errors = validate_with_schema_value(valid_doc, extracted.document_schema.clone())
        .expect("Validation should succeed");
    assert!(errors.is_empty(), "Valid document should have no errors");

    // Validate invalid document
    let errors = validate_with_schema_value(invalid_doc, extracted.document_schema)
        .expect("Validation should succeed");
    assert!(!errors.is_empty(), "Invalid document should have errors");

    // Check that we got type mismatch errors
    let type_errors: Vec<_> = errors
        .iter()
        .filter(|e| {
            matches!(
                e.kind,
                eure_schema::ValidationErrorKind::TypeMismatch { .. }
            )
        })
        .collect();
    assert_eq!(type_errors.len(), 2, "Should have 2 type mismatch errors");
}

#[test]
fn test_variant_schema_extraction() {
    let schema_doc = r#"
# Root-level field definitions
actions.$array = .$types.Action

# Type definitions
@ $types.Action
@ $types.Action.$variants.set-text
speaker.$type = .string
text.$type = .string

@ $types.Action.$variants.set-choice
prompt.$type = .string
options.$array = .string
"#;

    let extracted =
        extract_schema_from_value(schema_doc).expect("Schema extraction should succeed");

    // Check Action type is a variant
    println!(
        "Types: {:?}",
        extracted.document_schema.types.keys().collect::<Vec<_>>()
    );
    let action_type = &extracted.document_schema.types[&ObjectKey::String("Action".to_string())];
    println!("Action type: {action_type:?}");
    match &action_type.type_expr {
        eure_schema::Type::Variants(variant_schema) => {
            println!(
                "Variants found: {:?}",
                variant_schema.variants.keys().collect::<Vec<_>>()
            );
            assert!(
                variant_schema
                    .variants
                    .contains_key(&ObjectKey::String("set-text".to_string()))
            );
            assert!(
                variant_schema
                    .variants
                    .contains_key(&ObjectKey::String("set-choice".to_string()))
            );

            // Check set-text variant fields
            let set_text = &variant_schema.variants[&ObjectKey::String("set-text".to_string())];
            assert!(
                set_text
                    .fields
                    .contains_key(&ObjectKey::String("speaker".to_string()))
            );
            assert!(
                set_text
                    .fields
                    .contains_key(&ObjectKey::String("text".to_string()))
            );

            // Check set-choice variant fields
            let set_choice =
                &variant_schema.variants[&ObjectKey::String("set-choice".to_string())];
            assert!(
                set_choice
                    .fields
                    .contains_key(&ObjectKey::String("prompt".to_string()))
            );
            assert!(
                set_choice
                    .fields
                    .contains_key(&ObjectKey::String("options".to_string()))
            );
        }
        _ => panic!("Action type should be Variants"),
    }
}
