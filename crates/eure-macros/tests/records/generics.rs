use eure::FromEure;

#[derive(Debug, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
struct Wrapper<T> {
    inner: T,
}

#[derive(Debug, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
struct Pair<A, B> {
    first: A,
    second: B,
}

#[derive(Debug, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
struct Wrapped<T>(T);

#[test]
fn test_wrapper_with_string() {
    use eure::eure;
    let doc = eure!({ inner = "hello" });
    let result: Wrapper<String> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(
        result,
        Wrapper {
            inner: "hello".to_string()
        }
    );
}

#[test]
fn test_wrapper_with_i32() {
    use eure::eure;
    let doc = eure!({ inner = 42 });
    let result: Wrapper<i32> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(result, Wrapper { inner: 42 });
}

#[test]
fn test_wrapper_with_bool() {
    use eure::eure;
    let doc = eure!({ inner = true });
    let result: Wrapper<bool> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(result, Wrapper { inner: true });
}

#[test]
fn test_wrapper_with_vec() {
    use eure::eure;
    let doc = eure!({ inner = [1, 2, 3] });
    let result: Wrapper<Vec<i32>> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(
        result,
        Wrapper {
            inner: vec![1, 2, 3]
        }
    );
}

#[test]
fn test_pair_with_different_types() {
    use eure::eure;
    let doc = eure!({ first = "hello", second = 42 });
    let result: Pair<String, i32> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(
        result,
        Pair {
            first: "hello".to_string(),
            second: 42
        }
    );
}

#[test]
fn test_pair_with_same_types() {
    use eure::eure;
    let doc = eure!({ first = "hello", second = "world" });
    let result: Pair<String, String> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(
        result,
        Pair {
            first: "hello".to_string(),
            second: "world".to_string()
        }
    );
}

#[test]
fn test_newtype_wrapper_with_string() {
    use eure::eure;
    let doc = eure!({ = "hello" });
    let result: Wrapped<String> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(result, Wrapped("hello".to_string()));
}

#[test]
fn test_newtype_wrapper_with_i32() {
    use eure::eure;
    let doc = eure!({ = 42 });
    let result: Wrapped<i32> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(result, Wrapped(42));
}

#[test]
fn test_nested_generic_structs() {
    use eure::eure;
    // In eure syntax, nested records use => for key-value pairs inside braces
    let doc = eure!({ inner = { inner => "nested" } });
    let result: Wrapper<Wrapper<String>> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(
        result,
        Wrapper {
            inner: Wrapper {
                inner: "nested".to_string()
            }
        }
    );
}
