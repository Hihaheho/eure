use eure::ParseDocument;

#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document, rename_all = "camelCase")]
struct User {
    user_name: String,
    email_address: String,
}

#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document, rename_all = "kebab-case")]
struct Config {
    max_retries: i32,
    timeout_seconds: i32,
}

#[allow(non_snake_case)]
#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document, rename_all = "snake_case")]
struct PascalCaseFields {
    UserName: String,
    EmailAddress: String,
}

#[test]
fn test_parse_struct_with_rename_all_camel_case() {
    use eure::eure;
    let doc = eure!({ userName = "Alice", emailAddress = "alice@example.com" });
    assert_eq!(
        doc.parse::<User>(doc.get_root_id()).unwrap(),
        User {
            user_name: "Alice".to_string(),
            email_address: "alice@example.com".to_string()
        }
    );
}

#[test]
fn test_parse_struct_with_rename_all_kebab_case() {
    use eure::eure;
    let doc = eure!({ "max-retries" = 3, "timeout-seconds" = 30 });
    assert_eq!(
        doc.parse::<Config>(doc.get_root_id()).unwrap(),
        Config {
            max_retries: 3,
            timeout_seconds: 30
        }
    );
}

#[test]
fn test_parse_struct_with_rename_all_snake_case() {
    use eure::eure;
    let doc = eure!({ user_name = "Bob", email_address = "bob@example.com" });
    assert_eq!(
        doc.parse::<PascalCaseFields>(doc.get_root_id()).unwrap(),
        PascalCaseFields {
            UserName: "Bob".to_string(),
            EmailAddress: "bob@example.com".to_string()
        }
    );
}

#[test]
fn test_parse_struct_with_rename_all_wrong_case_error() {
    use eure::eure;
    // Using snake_case instead of camelCase should fail
    let doc = eure!({ user_name = "Alice", email_address = "alice@example.com" });
    let result = doc.parse::<User>(doc.get_root_id());
    assert!(result.is_err());
}
