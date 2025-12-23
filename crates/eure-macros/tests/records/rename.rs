use eure::ParseDocument;

// Basic struct with explicit field rename
#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
struct BasicRename {
    #[eure(rename = "userName")]
    user_name: String,
    #[eure(rename = "emailAddress")]
    email_address: String,
}

// Struct with rename that overrides rename_all
#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document, rename_all = "snake_case")]
struct RenameOverridesRenameAll {
    // This field uses rename_all (snake_case, but already snake_case so no change)
    first_name: String,
    // This field overrides rename_all with explicit rename
    #[eure(rename = "customLastName")]
    last_name: String,
}

// Struct with ext field and rename
#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
struct ExtFieldRename {
    name: String,
    #[eure(ext, rename = "isDeprecated")]
    deprecated: bool,
}

// Struct in parse_ext context with rename
#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document, parse_ext)]
struct ParseExtRename {
    #[eure(rename = "enableFeature")]
    enable: bool,
}

#[test]
fn test_basic_field_rename() {
    use eure::eure;
    let doc = eure!({ userName = "Alice", emailAddress = "alice@example.com" });
    assert_eq!(
        doc.parse::<BasicRename>(doc.get_root_id()).unwrap(),
        BasicRename {
            user_name: "Alice".to_string(),
            email_address: "alice@example.com".to_string()
        }
    );
}

#[test]
fn test_field_rename_wrong_name_error() {
    use eure::eure;
    // Using snake_case instead of the renamed camelCase should fail
    let doc = eure!({ user_name = "Alice", email_address = "alice@example.com" });
    let result = doc.parse::<BasicRename>(doc.get_root_id());
    assert!(result.is_err());
}

#[test]
fn test_rename_overrides_rename_all() {
    use eure::eure;
    // first_name uses rename_all (snake_case, already snake_case)
    // last_name uses explicit rename "customLastName"
    let doc = eure!({ first_name = "John", customLastName = "Doe" });
    assert_eq!(
        doc.parse::<RenameOverridesRenameAll>(doc.get_root_id()).unwrap(),
        RenameOverridesRenameAll {
            first_name: "John".to_string(),
            last_name: "Doe".to_string()
        }
    );
}

#[test]
fn test_rename_overrides_rename_all_wrong_name_error() {
    use eure::eure;
    // Using snake_case for last_name should fail (it expects customLastName)
    let doc = eure!({ first_name = "John", last_name = "Doe" });
    let result = doc.parse::<RenameOverridesRenameAll>(doc.get_root_id());
    assert!(result.is_err());
}

#[test]
fn test_ext_field_rename() {
    use eure::eure;
    let doc = eure!({ name = "MyFeature", %isDeprecated = true });
    assert_eq!(
        doc.parse::<ExtFieldRename>(doc.get_root_id()).unwrap(),
        ExtFieldRename {
            name: "MyFeature".to_string(),
            deprecated: true
        }
    );
}

#[test]
fn test_ext_field_rename_wrong_name_error() {
    use eure::eure;
    // Using original field name instead of renamed should fail
    let doc = eure!({ name = "MyFeature", %deprecated = true });
    let result = doc.parse::<ExtFieldRename>(doc.get_root_id());
    assert!(result.is_err());
}

#[test]
fn test_parse_ext_context_rename() {
    use eure::document::node::NodeValue;
    use eure::document::EureDocument;
    use eure::value::PrimitiveValue;

    let mut doc = EureDocument::new();
    let root_id = doc.get_root_id();

    // Add extension with renamed name
    let ext_id = doc
        .add_extension("enableFeature".parse().unwrap(), root_id)
        .unwrap()
        .node_id;
    doc.node_mut(ext_id).content = NodeValue::Primitive(PrimitiveValue::Bool(true));

    let ctx = doc.parse_extension_context(root_id);
    let result: ParseExtRename = ctx.parse().unwrap();
    assert_eq!(result, ParseExtRename { enable: true });
}
