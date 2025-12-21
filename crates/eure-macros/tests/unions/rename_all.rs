use eure::ParseDocument;

/// Tests for enum with rename_all using untagged representation (default).
/// Container-level rename_all only renames variant names, not struct variant fields
/// (matching serde's behavior).

#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document, rename_all = "camelCase")]
enum Action {
    DoSomething(String),
    ProcessData { input_value: i32, output_format: String },
}

/// Tests newtype variant (rename_all doesn't affect this).
#[test]
fn test_parse_enum_with_rename_all_camel_case_newtype() {
    use eure::eure;
    let doc = eure!({ = "hello" });
    assert_eq!(
        doc.parse::<Action>(doc.get_root_id()).unwrap(),
        Action::DoSomething("hello".to_string())
    );
}

/// Tests that struct variant fields use original field names (not renamed).
/// Container-level rename_all only affects variant names, not fields.
#[test]
fn test_parse_enum_with_rename_all_struct_variant_original_field_names() {
    use eure::eure;
    // Fields should use original snake_case names, not camelCase
    let doc = eure!({ = { input_value => 42, output_format => "json" } });
    assert_eq!(
        doc.parse::<Action>(doc.get_root_id()).unwrap(),
        Action::ProcessData {
            input_value: 42,
            output_format: "json".to_string()
        }
    );
}

/// Tests that using camelCase field names fails (fields are NOT renamed by container-level rename_all).
#[test]
fn test_parse_enum_with_rename_all_camel_case_field_names_error() {
    use eure::eure;
    // Using camelCase field names should fail because container-level rename_all
    // only affects variant names, not struct variant fields
    let doc = eure!({ = { inputValue => 42, outputFormat => "json" } });
    let result = doc.parse::<Action>(doc.get_root_id());
    assert!(result.is_err());
}

// Tests for rename_all_fields attribute

#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document, rename_all_fields = "camelCase")]
enum ActionWithFieldRename {
    DoSomething(String),
    ProcessData { input_value: i32, output_format: String },
}

/// Tests that struct variant fields are renamed by rename_all_fields.
#[test]
fn test_parse_enum_with_rename_all_fields_camel_case() {
    use eure::eure;
    // Fields should use camelCase due to rename_all_fields
    let doc = eure!({ = { inputValue => 42, outputFormat => "json" } });
    assert_eq!(
        doc.parse::<ActionWithFieldRename>(doc.get_root_id()).unwrap(),
        ActionWithFieldRename::ProcessData {
            input_value: 42,
            output_format: "json".to_string()
        }
    );
}

/// Tests that snake_case field names fail when rename_all_fields = "camelCase".
#[test]
fn test_parse_enum_with_rename_all_fields_wrong_case_error() {
    use eure::eure;
    // Using snake_case field names should fail
    let doc = eure!({ = { input_value => 42, output_format => "json" } });
    let result = doc.parse::<ActionWithFieldRename>(doc.get_root_id());
    assert!(result.is_err());
}

// Tests for combining rename_all and rename_all_fields

#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document, rename_all = "snake_case", rename_all_fields = "camelCase")]
enum Event {
    UserCreated { user_id: i32, created_at: String },
    OrderPlaced { order_id: i32 },
}

/// Tests that both variant names and field names are renamed correctly.
#[test]
fn test_parse_enum_with_both_rename_all_and_rename_all_fields() {
    use eure::eure;
    // Variant names use snake_case, fields use camelCase
    let doc = eure!({ = { userId => 123, createdAt => "2024-01-01" } });
    assert_eq!(
        doc.parse::<Event>(doc.get_root_id()).unwrap(),
        Event::UserCreated {
            user_id: 123,
            created_at: "2024-01-01".to_string()
        }
    );
}
