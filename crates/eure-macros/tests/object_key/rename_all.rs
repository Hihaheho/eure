use eure::document::parse::ParseObjectKey;
use eure::document::value::ObjectKey;
use eure::ObjectKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, ObjectKey)]
#[eure(crate = ::eure::document, rename_all = "snake_case")]
enum HttpMethod {
    GetAll,
    PostNew,
    DeleteOne,
}

#[test]
fn test_parse_snake_case() {
    let key = ObjectKey::String("get_all".to_string());
    let method: HttpMethod = ParseObjectKey::from_object_key(&key).unwrap();
    assert_eq!(method, HttpMethod::GetAll);
}

#[test]
fn test_into_snake_case() {
    let key: ObjectKey = HttpMethod::PostNew.into();
    assert_eq!(key, ObjectKey::String("post_new".to_string()));
}

#[test]
fn test_roundtrip_snake_case() {
    for (variant, name) in [
        (HttpMethod::GetAll, "get_all"),
        (HttpMethod::PostNew, "post_new"),
        (HttpMethod::DeleteOne, "delete_one"),
    ] {
        let key: ObjectKey = variant.into();
        assert_eq!(key, ObjectKey::String(name.to_string()));

        let parsed: HttpMethod = ParseObjectKey::from_object_key(&key).unwrap();
        assert_eq!(parsed, variant);
    }
}

#[test]
fn test_original_name_does_not_match() {
    let key = ObjectKey::String("GetAll".to_string());
    let result: Result<HttpMethod, _> = ParseObjectKey::from_object_key(&key);
    assert!(result.is_err());
}
