use eure::ParseDocument;

#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
enum Item<T> {
    Normal(T),
    List(Vec<T>),
}

#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
enum Either<L, R> {
    Left(L),
    Right(R),
}

#[test]
fn test_item_normal_string() {
    use eure::eure;
    let doc = eure!({ = "hello" });
    let result: Item<String> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(result, Item::Normal("hello".to_string()));
}

#[test]
fn test_item_normal_i32() {
    use eure::eure;
    let doc = eure!({ = 42 });
    let result: Item<i32> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(result, Item::Normal(42));
}

#[test]
fn test_item_list_i32() {
    use eure::eure;
    let doc = eure!({ = [1, 2, 3] });
    let result: Item<i32> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(result, Item::List(vec![1, 2, 3]));
}

#[test]
fn test_item_list_string() {
    use eure::eure;
    let doc = eure!({ = ["a", "b", "c"] });
    let result: Item<String> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(
        result,
        Item::List(vec!["a".to_string(), "b".to_string(), "c".to_string()])
    );
}

#[test]
fn test_either_left_string() {
    use eure::eure;
    let doc = eure!({ = "hello" });
    let result: Either<String, i32> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(result, Either::Left("hello".to_string()));
}

#[test]
fn test_either_right_i32() {
    use eure::eure;
    let doc = eure!({ = 42 });
    let result: Either<String, i32> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(result, Either::Right(42));
}

#[test]
fn test_either_left_bool() {
    use eure::eure;
    let doc = eure!({ = true });
    let result: Either<bool, String> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(result, Either::Left(true));
}

#[test]
fn test_either_right_string() {
    use eure::eure;
    let doc = eure!({ = "world" });
    let result: Either<bool, String> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(result, Either::Right("world".to_string()));
}
