use eure::FromEure;

#[derive(Debug, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
enum TestEnum {
    Unit,
    Tuple(i32, bool),
    Struct { a: i32, b: bool },
    Newtype(String),
}

#[test]
fn test_parse_union() {
    use eure::eure;
    let a = eure!({ = "Unit" });
    assert_eq!(
        a.parse::<TestEnum>(a.get_root_id()).unwrap(),
        TestEnum::Unit
    );
    let b = eure!({ = (1, true) });
    assert_eq!(
        b.parse::<TestEnum>(b.get_root_id()).unwrap(),
        TestEnum::Tuple(1, true)
    );
    let c = eure!({ = { a => 1, b => true } });
    assert_eq!(
        c.parse::<TestEnum>(c.get_root_id()).unwrap(),
        TestEnum::Struct { a: 1, b: true }
    );
    let d = eure!({ = "hello" });
    assert_eq!(
        d.parse::<TestEnum>(d.get_root_id()).unwrap(),
        TestEnum::Newtype("hello".to_string())
    );
}
