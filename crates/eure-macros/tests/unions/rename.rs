use eure::FromEure;

// Basic enum with explicit variant rename
#[derive(Debug, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
enum BasicVariantRename {
    #[eure(rename = "first_action")]
    FirstAction(String),
    #[eure(rename = "second_action")]
    SecondAction(i32),
}

// Enum with rename that overrides rename_all
#[derive(Debug, PartialEq, FromEure)]
#[eure(crate = ::eure::document, rename_all = "snake_case")]
enum RenameOverridesRenameAll {
    // This variant uses rename_all (snake_case)
    NormalVariant(String),
    // This variant overrides rename_all with explicit rename
    #[eure(rename = "customName")]
    OverriddenVariant(i32),
}

// Enum with struct variant and field rename
#[derive(Debug, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
enum StructVariantFieldRename {
    Simple(String),
    Complex {
        #[eure(rename = "userName")]
        user_name: String,
        #[eure(rename = "userAge")]
        user_age: i32,
    },
}

// Enum with field rename that overrides rename_all_fields
#[derive(Debug, PartialEq, FromEure)]
#[eure(crate = ::eure::document, rename_all_fields = "camelCase")]
enum FieldRenameOverridesRenameAllFields {
    Data {
        // Uses rename_all_fields (camelCase)
        first_name: String,
        // Overrides with explicit rename
        #[eure(rename = "customField")]
        last_name: String,
    },
}

// Enum with both variant and field renames
#[derive(Debug, PartialEq, FromEure)]
#[eure(crate = ::eure::document, rename_all = "snake_case", rename_all_fields = "camelCase")]
enum CombinedRenames {
    // Variant uses explicit rename, overriding rename_all
    #[eure(rename = "CUSTOM_VARIANT")]
    MyVariant {
        // Field uses rename_all_fields (camelCase)
        user_id: i32,
        // Field overrides with explicit rename
        #[eure(rename = "CUSTOM_FIELD")]
        user_name: String,
    },
}

#[test]
fn test_basic_variant_rename() {
    use eure::eure;
    let doc = eure!({ = "hello" });
    assert_eq!(
        doc.parse::<BasicVariantRename>(doc.get_root_id()).unwrap(),
        BasicVariantRename::FirstAction("hello".to_string())
    );
}

#[test]
fn test_variant_rename_second() {
    use eure::eure;
    let doc = eure!({ = 42 });
    assert_eq!(
        doc.parse::<BasicVariantRename>(doc.get_root_id()).unwrap(),
        BasicVariantRename::SecondAction(42)
    );
}

#[test]
fn test_variant_rename_overrides_rename_all() {
    use eure::eure;
    // NormalVariant uses rename_all (snake_case -> normal_variant)
    let doc1 = eure!({ = "test" });
    assert_eq!(
        doc1.parse::<RenameOverridesRenameAll>(doc1.get_root_id())
            .unwrap(),
        RenameOverridesRenameAll::NormalVariant("test".to_string())
    );

    // OverriddenVariant uses explicit rename "customName"
    let doc2 = eure!({ = 123 });
    assert_eq!(
        doc2.parse::<RenameOverridesRenameAll>(doc2.get_root_id())
            .unwrap(),
        RenameOverridesRenameAll::OverriddenVariant(123)
    );
}

#[test]
fn test_struct_variant_field_rename() {
    use eure::eure;
    let doc = eure!({ = { userName => "Alice", userAge => 30 } });
    assert_eq!(
        doc.parse::<StructVariantFieldRename>(doc.get_root_id())
            .unwrap(),
        StructVariantFieldRename::Complex {
            user_name: "Alice".to_string(),
            user_age: 30
        }
    );
}

#[test]
fn test_struct_variant_field_rename_wrong_name_error() {
    use eure::eure;
    // Using original field names instead of renamed should fail
    let doc = eure!({ = { user_name => "Alice", user_age => 30 } });
    let result = doc.parse::<StructVariantFieldRename>(doc.get_root_id());
    assert!(result.is_err());
}

#[test]
fn test_field_rename_overrides_rename_all_fields() {
    use eure::eure;
    // first_name uses rename_all_fields (camelCase -> firstName)
    // last_name uses explicit rename "customField"
    let doc = eure!({ = { firstName => "John", customField => "Doe" } });
    assert_eq!(
        doc.parse::<FieldRenameOverridesRenameAllFields>(doc.get_root_id())
            .unwrap(),
        FieldRenameOverridesRenameAllFields::Data {
            first_name: "John".to_string(),
            last_name: "Doe".to_string()
        }
    );
}

#[test]
fn test_field_rename_overrides_rename_all_fields_wrong_name_error() {
    use eure::eure;
    // Using lastName (from rename_all_fields) instead of customField should fail
    let doc = eure!({ = { firstName => "John", lastName => "Doe" } });
    let result = doc.parse::<FieldRenameOverridesRenameAllFields>(doc.get_root_id());
    assert!(result.is_err());
}

#[test]
fn test_combined_renames() {
    use eure::eure;
    // Variant uses explicit rename "CUSTOM_VARIANT" (overrides snake_case)
    // user_id uses rename_all_fields (camelCase -> userId)
    // user_name uses explicit rename "CUSTOM_FIELD"
    let doc = eure!({ = { userId => 42, "CUSTOM_FIELD" => "Alice" } });
    assert_eq!(
        doc.parse::<CombinedRenames>(doc.get_root_id()).unwrap(),
        CombinedRenames::MyVariant {
            user_id: 42,
            user_name: "Alice".to_string()
        }
    );
}
