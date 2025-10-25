//! Integration tests for eure-schema

use eure_schema::*;
use eure_value::value::{KeyCmpValue, Path};

/// Helper to parse and extract schema from a document
fn extract(input: &str) -> ExtractedSchema {
    extract_schema_from_value(input).expect("Failed to extract schema")
}

/// Helper to parse and validate with schema
fn validate(input: &str, schema: DocumentSchema) -> Vec<ValidationError> {
    validate_with_schema_value(input, schema).expect("Failed to validate")
}

/// Helper to validate with inline schema extraction
fn validate_with_inline(input: &str, base_schema: DocumentSchema) -> Vec<ValidationError> {
    let extracted = extract_schema_from_value(input).expect("Failed to extract schema");

    // Merge inline schemas with base schema
    let mut merged_schema = base_schema;

    // Merge types
    for (name, type_def) in extracted.document_schema.types {
        merged_schema.types.insert(name, type_def);
    }

    // Merge root fields
    for (name, field_schema) in extracted.document_schema.root.fields {
        merged_schema.root.fields.insert(name, field_schema);
    }

    validate_with_schema_value(input, merged_schema).expect("Failed to validate")
}

/// Helper to parse and validate self-describing document
fn validate_self(input: &str) -> ValidationResult {
    validate_self_describing(input).expect("Failed to validate self-describing document")
}

#[cfg(test)]
mod basic_type_validation {
    use super::*;

    #[test]
    fn test_string_type_validation() {
        let schema_doc = r#"
name.$type = .string
"#;
        let schema = extract(schema_doc).document_schema;

        // Valid string
        let valid_doc = r#"name = "Alice""#;
        let errors = validate(valid_doc, schema.clone());
        assert!(errors.is_empty());

        // Invalid - number instead of string
        let invalid_doc = r#"name = 42"#;
        let errors = validate(invalid_doc, schema.clone());
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0].kind,
            ValidationErrorKind::TypeMismatch { .. }
        ));
    }

    #[test]
    fn test_number_type_validation() {
        let schema_doc = r#"
age.$type = .number
"#;
        let schema = extract(schema_doc).document_schema;

        // Valid number
        let valid_doc = r#"age = 30"#;
        let errors = validate(valid_doc, schema.clone());
        assert!(errors.is_empty());

        // Invalid - string instead of number
        let invalid_doc = r#"age = "thirty""#;
        let errors = validate(invalid_doc, schema.clone());
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0].kind,
            ValidationErrorKind::TypeMismatch { .. }
        ));
    }

    #[test]
    fn test_boolean_type_validation() {
        let schema_doc = r#"
active.$type = .boolean
"#;
        let schema = extract(schema_doc).document_schema;

        // Valid boolean
        let valid_doc = r#"active = true"#;
        let errors = validate(valid_doc, schema.clone());
        assert!(errors.is_empty());

        // Also valid
        let valid_doc2 = r#"active = false"#;
        let errors = validate(valid_doc2, schema.clone());
        assert!(errors.is_empty());

        // Invalid - string instead of boolean
        let invalid_doc = r#"active = "yes""#;
        let errors = validate(invalid_doc, schema.clone());
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0].kind,
            ValidationErrorKind::TypeMismatch { .. }
        ));
    }

    #[test]
    fn test_null_type_validation() {
        let schema_doc = r#"
optional.$type = .null
"#;
        let schema = extract(schema_doc).document_schema;

        // Valid null
        let valid_doc = r#"optional = null"#;
        let errors = validate(valid_doc, schema.clone());
        assert!(errors.is_empty());

        // Invalid - string instead of null
        let invalid_doc = r#"optional = "not null""#;
        let errors = validate(invalid_doc, schema.clone());
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0].kind,
            ValidationErrorKind::TypeMismatch { .. }
        ));
    }

    #[test]
    fn test_path_type_validation() {
        let schema_doc = r#"
reference.$type = .path
"#;
        let schema = extract(schema_doc).document_schema;

        // Valid path
        let valid_doc = r#"reference = .some.path.value"#;
        let errors = validate(valid_doc, schema.clone());
        assert!(errors.is_empty());

        // Invalid - string instead of path
        let invalid_doc = r#"reference = "not.a.path""#;
        let errors = validate(invalid_doc, schema.clone());
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0].kind,
            ValidationErrorKind::TypeMismatch { .. }
        ));
    }

    #[test]
    fn test_typed_string_validation() {
        let schema_doc = r#"
email.$type = .code.email
"#;
        let schema = extract(schema_doc).document_schema;

        // Valid - strings are accepted for typed strings
        let valid_doc = r#"email = email`user@example.com`"#;
        let errors = validate(valid_doc, schema.clone());
        assert!(errors.is_empty());

        // Invalid - number instead of string
        let invalid_doc = r#"email = 123"#;
        let errors = validate(invalid_doc, schema.clone());
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0].kind,
            ValidationErrorKind::TypeMismatch { .. }
        ));
    }

    #[test]
    fn test_code_type_validation() {
        let schema_doc = r#"
script.$type = .code.javascript
"#;
        let schema = extract(schema_doc).document_schema;

        // Valid - strings are accepted for code
        let valid_doc = r#"script = javascript`console.log('hello')`"#;
        let errors = validate(valid_doc, schema.clone());
        assert!(errors.is_empty());

        // Valid - strings are accepted for code
        let valid_doc = r#"script = ```javascript
        console.log('hello')
        ```"#;
        let errors = validate(valid_doc, schema.clone());
        assert!(errors.is_empty());

        // Invalid - boolean instead of string
        let invalid_doc = r#"script = true"#;
        let errors = validate(invalid_doc, schema.clone());
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0].kind,
            ValidationErrorKind::TypeMismatch { .. }
        ));
    }
}

#[cfg(test)]
mod constraint_validation {
    use super::*;

    #[test]
    fn test_string_length_constraints() {
        let schema_doc = r#"
username.$type = .string
username.$length = (3, 20)
"#;
        let schema = extract(schema_doc).document_schema;

        // Valid length
        let valid_doc = r#"username = "alice""#;
        let errors = validate(valid_doc, schema.clone());
        assert!(errors.is_empty());

        // Too short
        let invalid_short = r#"username = "ab""#;
        let errors = validate(invalid_short, schema.clone());
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0].kind,
            ValidationErrorKind::StringLengthViolation { .. }
        ));

        // Too long
        let invalid_long = r#"username = "this_username_is_way_too_long""#;
        let errors = validate(invalid_long, schema.clone());
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0].kind,
            ValidationErrorKind::StringLengthViolation { .. }
        ));
    }

    #[test]
    fn test_string_pattern_constraint() {
        let schema_doc = r#"
email.$type = .string
email.$pattern = "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"
"#;
        let schema = extract(schema_doc).document_schema;

        // Valid email pattern
        let valid_doc = r#"email = "user@example.com""#;
        let errors = validate(valid_doc, schema.clone());
        assert!(errors.is_empty());

        // Invalid email pattern
        let invalid_doc = r#"email = "not-an-email""#;
        let errors = validate(invalid_doc, schema.clone());
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0].kind,
            ValidationErrorKind::PatternMismatch { .. }
        ));
    }

    #[test]
    fn test_number_range_constraints() {
        let schema_doc = r#"
age.$type = .number
age.$range = (18, 150)
"#;
        let schema = extract(schema_doc).document_schema;

        // Valid age
        let valid_doc = r#"age = 25"#;
        let errors = validate(valid_doc, schema.clone());
        assert!(errors.is_empty());

        // Too low
        let invalid_low = r#"age = 5"#;
        let errors = validate(invalid_low, schema.clone());
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0].kind,
            ValidationErrorKind::RangeViolation { .. }
        ));

        // Too high
        let invalid_high = r#"age = 200"#;
        let errors = validate(invalid_high, schema.clone());
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0].kind,
            ValidationErrorKind::RangeViolation { .. }
        ));
    }

    // Removed test_array_length_constraints as $min-items and $max-items
    // have been removed from the constraint system per language designer's request.
}

#[cfg(test)]
mod schema_definition_tests {
    use super::*;

    #[test]
    fn test_type_definitions() {
        let schema_doc = r#"
@ $types.Person
$type = .object
@ $types.Person.name
$type = .string
@ $types.Person.age
$type = .number
$optional = true
"#;
        let schema = extract(schema_doc);

        // Check that Person type was extracted
        assert!(
            schema
                .document_schema
                .types
                .contains_key(&KeyCmpValue::String("Person".to_string()))
        );
        let person_type = &schema.document_schema.types[&KeyCmpValue::String("Person".to_string())];

        // Check type is object
        assert!(matches!(person_type.type_expr, Type::Object(_)));

        // Check that it's a pure schema document
        assert!(schema.is_pure_schema);
    }

    #[test]
    fn test_type_references() {
        let schema_doc = r#"
@ $types.Name
$type = .string

@ $types.Person
$type = .object
@ $types.Person.name
$type = .$types.Name
"#;
        let schema = extract(schema_doc).document_schema;

        // Create a document using the Person type
        let doc = r#"
@ person
$type = .$types.Person
name = "Alice"
"#;

        // Use validate_with_inline which handles merging for us
        let errors = validate_with_inline(doc, schema);
        if !errors.is_empty() {
            eprintln!("Validation errors:");
            for error in &errors {
                eprintln!("  - {error:?}");
            }
        }
        assert!(errors.is_empty());
    }

    #[test]
    fn test_cascade_type() {
        let schema_doc = r#"
$cascade-type = .string
"#;
        let schema = extract(schema_doc).document_schema;

        // Check cascade type was set on root
        assert!(matches!(
            schema.cascade_types.get(&Path::root()),
            Some(Type::String)
        ));

        // All fields should accept strings by default
        let doc = r#"
any_field = "value"
another_field = "another value"
"#;
        let errors = validate(doc, schema);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_union_types() {
        let schema_doc = r#"
@ $types.StringOrNumber
$union = [.string, .number]
"#;
        let schema = extract(schema_doc).document_schema;

        // Valid - string
        let valid_string = r#"
value = "hello"
value.$type = .$types.StringOrNumber
"#;
        // Extract and merge inline schemas
        let mut test_schema = schema.clone();
        let doc_schema = extract_schema_from_value(valid_string).expect("Failed to extract schema");
        for (name, field) in doc_schema.document_schema.root.fields {
            test_schema.root.fields.insert(name, field);
        }
        let errors =
            validate_with_schema_value(valid_string, test_schema).expect("Failed to validate");
        assert!(errors.is_empty());

        // Valid - number
        let valid_number = r#"
value = 42
value.$type = .$types.StringOrNumber
"#;
        let mut test_schema = schema.clone();
        let doc_schema = extract_schema_from_value(valid_number).expect("Failed to extract schema");
        for (name, field) in doc_schema.document_schema.root.fields {
            test_schema.root.fields.insert(name, field);
        }
        let errors =
            validate_with_schema_value(valid_number, test_schema).expect("Failed to validate");
        assert!(errors.is_empty());

        // Invalid - boolean
        let invalid = r#"
value = true
value.$type = .$types.StringOrNumber
"#;
        let mut test_schema = schema.clone();
        let doc_schema = extract_schema_from_value(invalid).expect("Failed to extract schema");
        for (name, field) in doc_schema.document_schema.root.fields {
            test_schema.root.fields.insert(name, field);
        }
        let errors = validate_with_schema_value(invalid, test_schema).expect("Failed to validate");
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0].kind,
            ValidationErrorKind::TypeMismatch { .. }
        ));
    }

    #[test]
    fn test_variant_types() {
        let schema_doc = r#"
@ $types.Action
@ $types.Action.$variants.create.name
$type = .string
@ $types.Action.$variants.delete.id
$type = .number
"#;
        let schema = extract(schema_doc).document_schema;

        // Valid create variant
        let valid_create = r#"
@ action
$type = .$types.Action
$variant = "create"
name = "New Item"
"#;
        let errors = validate_with_inline(valid_create, schema.clone());
        if !errors.is_empty() {
            eprintln!("Validation errors for create variant:");
            for error in &errors {
                eprintln!("  - {:?}", error.kind);
            }
        }
        assert!(errors.is_empty());

        // Valid delete variant
        let valid_delete = r#"
@ action
$type = .$types.Action
$variant = "delete"
id = 123
"#;
        let errors = validate_with_inline(valid_delete, schema);
        assert!(errors.is_empty());
    }
}

#[cfg(test)]
mod inline_schema_tests {
    use super::*;

    #[test]
    fn test_inline_type_definition() {
        let doc = r#"
name = "Alice"
name.$type = .string

age = 30
age.$type = .number
"#;
        let result = validate_self(doc);

        // Debug: print what we got
        println!(
            "Root fields found: {}",
            result.schema.document_schema.root.fields.len()
        );
        for (name, schema) in &result.schema.document_schema.root.fields {
            println!("  Field: {:?}, Type: {:?}", name, schema.type_expr);
        }

        // Debug: print cascade types
        println!(
            "Cascade types: {:?}",
            result.schema.document_schema.cascade_types
        );

        // Debug: print any errors
        if !result.errors.is_empty() {
            println!("Validation errors:");
            for error in &result.errors {
                println!("  {:?}", error.kind);
            }
        }

        // Should extract inline schemas into root fields
        assert_eq!(result.schema.document_schema.root.fields.len(), 2);
        assert!(
            result
                .schema
                .document_schema
                .root
                .fields
                .contains_key(&KeyCmpValue::String("name".to_string()))
        );
        assert!(
            result
                .schema
                .document_schema
                .root
                .fields
                .contains_key(&KeyCmpValue::String("age".to_string()))
        );

        // Should validate successfully
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_inline_constraints() {
        let doc = r#"
username = "alice"
username.$type = .string
username.$length = (3, 20)

score = 85
score.$type = .number
score.$range = (0, 100)
"#;
        let result = validate_self(doc);

        // Debug: print extracted fields
        println!("First test - fields found:");
        for (name, schema) in &result.schema.document_schema.root.fields {
            println!(
                "  Field: {:?}, Type: {:?}, Constraints: {:?}",
                name, schema.type_expr, schema.constraints
            );
        }

        assert!(result.errors.is_empty());

        // Test constraint violation
        let invalid_doc = r#"
username = "ab"
username.$type = .string
username.$length = (3, 20)
"#;
        let result = validate_self(invalid_doc);

        // Debug: print extracted fields and errors
        println!("\nSecond test - fields found:");
        for (name, schema) in &result.schema.document_schema.root.fields {
            println!(
                "  Field: {:?}, Type: {:?}, Constraints: {:?}",
                name, schema.type_expr, schema.constraints
            );
        }
        println!("Errors: {}", result.errors.len());
        for error in &result.errors {
            println!("  {:?}", error.kind);
        }

        assert_eq!(result.errors.len(), 1);
        assert!(matches!(
            result.errors[0].kind,
            ValidationErrorKind::StringLengthViolation { .. }
        ));
    }

    #[test]
    fn test_mixed_schema_and_data() {
        let doc = r#"
@ $types.Person {
    $type = .object
    @ name {
        $type = .string
    }
}

@ user {
    $type = .$types.Person
    name = "Bob"
}
"#;
        let result = validate_self(doc);

        // Not a pure schema (has data)
        assert!(!result.schema.is_pure_schema);

        // Should validate successfully
        if !result.errors.is_empty() {
            eprintln!("Validation errors in mixed schema/data test:");
            for error in &result.errors {
                eprintln!("  - {:?}", error.kind);
            }
        }
        assert!(result.errors.is_empty());
    }
}

#[cfg(test)]
mod preference_and_serde_tests {
    use super::*;

    // Removed test_prefer_section_warning - preferences are not implemented in value-based validator

    // Removed test_prefer_array_warning - $prefer.array is not a real extension

    #[test]
    fn test_serde_rename() {
        let schema_doc = r#"
@ $types.User
$type = .object
@ $types.User.first_name
$type = .string
$rename = "firstName"
"#;
        let schema = extract(schema_doc);

        // Check that rename was extracted
        let user_type = &schema.document_schema.types[&KeyCmpValue::String("User".to_string())];
        if let Type::Object(obj_schema) = &user_type.type_expr {
            let field = &obj_schema
                .fields
                .get(&KeyCmpValue::String("first_name".to_string()))
                .unwrap();
            assert_eq!(field.serde.rename, Some("firstName".to_string()));
        } else {
            panic!("Expected object type");
        }
    }

    #[test]
    fn test_serde_rename_all() {
        let schema_doc = r#"
$rename-all = "camelCase"

@ $types.Person
$type = .object
$rename-all = "snake_case"
"#;
        let schema = extract(schema_doc);

        // Check global rename-all
        assert_eq!(
            schema.document_schema.serde_options.rename_all,
            Some(RenameRule::CamelCase)
        );

        // Check type-specific rename-all
        let person_type = &schema.document_schema.types[&KeyCmpValue::String("Person".to_string())];
        assert_eq!(person_type.serde.rename_all, Some(RenameRule::SnakeCase));
    }
}

#[cfg(test)]
mod error_detection_tests {
    use super::*;

    #[test]
    fn test_required_field_missing() {
        let schema_doc = r#"
@ $types.User
$type = .object
@ $types.User.name
$type = .string
@ $types.User.email
$type = .string
$optional = true
"#;
        let schema = extract(schema_doc).document_schema;

        // Missing required field
        let doc = r#"
@ user
$type = .$types.User
email = "user@example.com"
"#;
        let errors = validate_with_inline(doc, schema);

        // Check that we have a required field missing error for "name"
        let has_name_missing = errors.iter().any(|e|
            matches!(&e.kind, ValidationErrorKind::RequiredFieldMissing { field, .. } if matches!(field, KeyCmpValue::String(s) if s == "name"))
        );
        assert!(
            has_name_missing,
            "Expected 'name' to be flagged as missing required field"
        );
    }

    #[test]
    fn test_unexpected_field() {
        let schema_doc = r#"
@ $types.Strict
$type = .object
@ $types.Strict.allowed
$type = .string
"#;
        let schema = extract(schema_doc).document_schema;

        // Extra field not in schema
        let doc = r#"
@ data
$type = .$types.Strict
allowed = "yes"
extra = "not allowed"
"#;
        let errors = validate_with_inline(doc, schema);
        // This test is checking that when using inline schemas,
        // extra fields are detected. However, our current implementation
        // doesn't handle inline schemas during validation.
        // For now, we'll just check that "extra" is flagged as unexpected
        let has_extra_error = errors.iter().any(|e|
            matches!(&e.kind, ValidationErrorKind::UnexpectedField { field, .. } if matches!(field, KeyCmpValue::String(s) if s == "extra"))
        );
        assert!(
            has_extra_error,
            "Expected 'extra' to be flagged as unexpected field"
        );
    }

    #[test]
    fn test_unknown_type_reference() {
        // Create schema with cascade type to allow any field
        let mut schema = DocumentSchema::default();
        schema.cascade_types.insert(Path::root(), Type::Any);

        // Reference to non-existent type
        let doc = r#"
value = "hello"
value.$type = .$types.NonExistent
"#;
        let errors = validate_with_inline(doc, schema);
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0].kind,
            ValidationErrorKind::UnknownType(_)
        ));
    }
}

#[cfg(test)]
mod complex_scenario_tests {
    use super::*;

    #[test]
    fn test_nested_object_validation() {
        let schema_doc = r#"
@ $types.Address
$type = .object
@ $types.Address.street
$type = .string
@ $types.Address.city
$type = .string
@ $types.Address.zip
$type = .string
$pattern = "^[0-9]{5}$"

@ $types.Person
$type = .object
@ $types.Person.name
$type = .string
@ $types.Person.address
$type = .$types.Address
"#;
        let schema = extract(schema_doc).document_schema;

        // Valid nested object
        let valid_doc = r#"
@ person
$type = .$types.Person
name = "Alice"
@ person.address
street = "123 Main St"
city = "Springfield"
zip = "12345"
"#;
        let errors = validate_with_inline(valid_doc, schema.clone());
        assert!(errors.is_empty());

        // Invalid zip code
        let invalid_doc = r#"
@ person
$type = .$types.Person
name = "Bob"
@ person.address
street = "456 Oak Ave"
city = "Shelbyville"
zip = "invalid"
"#;
        let errors = validate_with_inline(invalid_doc, schema);
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0].kind,
            ValidationErrorKind::PatternMismatch { .. }
        ));
    }

    #[test]
    fn test_real_world_config_schema() {
        let schema_doc = r#"
@ $types.DatabaseConfig
$type = .object
@ $types.DatabaseConfig.host
$type = .string
@ $types.DatabaseConfig.port
$type = .number
$range = (1, 65535)
@ $types.DatabaseConfig.username
$type = .string
@ $types.DatabaseConfig.password
$type = .string
$optional = true

@ $types.ServerConfig
$type = .object
@ $types.ServerConfig.listen
$type = .string
$pattern = "^[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}:[0-9]+$"
@ $types.ServerConfig.workers
$type = .number
$range = (1, 100)

@ $types.AppConfig
$type = .object
@ $types.AppConfig.server
$type = .$types.ServerConfig
@ $types.AppConfig.database
$type = .$types.DatabaseConfig
"#;
        let schema = extract(schema_doc).document_schema;

        let config_doc = r#"
@ config
$type = .$types.AppConfig
@ config.server
listen = "127.0.0.1:8080"
workers = 4
@ config.database
host = "localhost"
port = 5432
username = "app_user"
"#;
        let errors = validate_with_inline(config_doc, schema);
        if !errors.is_empty() {
            println!("Real world config test errors:");
            for error in &errors {
                println!("  {:?}", error.kind);
            }
        }
        assert!(errors.is_empty());
    }

    #[test]
    fn test_variant_with_constraints() {
        let schema_doc = r#"
@ $types.Event
@ $types.Event.$variants.user-created.username
$type = .string
$length = (3, 20)
@ $types.Event.$variants.user-created.email
$type = .code.email
@ $types.Event.$variants.user-deleted.user-id
$type = .number
@ $types.Event.$variants.user-deleted.reason
$type = .string
$optional = true
"#;
        let schema = extract(schema_doc).document_schema;

        // Valid user-created event
        let valid_created = r#"
@ event
$type = .$types.Event
$variant = "user-created"
username = "alice123"
email = email`alice@example.com`
"#;
        let errors = validate_with_inline(valid_created, schema.clone());
        if !errors.is_empty() {
            eprintln!("Variant with constraints - validation errors for valid_created:");
            for error in &errors {
                eprintln!("  - {:?}", error.kind);
            }
        }
        assert!(errors.is_empty());

        // Invalid username length
        let invalid_username = r#"
@ event
$type = .$types.Event
$variant = "user-created"
username = "ab"
email = email`ab@example.com`
"#;
        let errors = validate_with_inline(invalid_username, schema);
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0].kind,
            ValidationErrorKind::StringLengthViolation { .. }
        ));
    }
}

#[cfg(test)]
mod array_tests {
    use super::*;

    #[test]
    fn test_array_element_validation() {
        let schema_doc = r#"
@ $types.StringArray
$array = .string
"#;
        let schema = extract(schema_doc).document_schema;

        // Valid array of strings
        let valid_doc = r#"
items = ["one", "two", "three"]
items.$type = .$types.StringArray
"#;
        let errors = validate_with_inline(valid_doc, schema.clone());
        assert!(errors.is_empty());

        // Invalid - contains number
        let invalid_doc = r#"
items = ["one", 2, "three"]
items.$type = .$types.StringArray
"#;
        let errors = validate_with_inline(invalid_doc, schema);
        assert!(!errors.is_empty());
    }
}
