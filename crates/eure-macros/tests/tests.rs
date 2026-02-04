mod unions {
    automod::dir!("./tests/unions");
}

mod records {
    automod::dir!("./tests/records");
}

mod build_schema {
    #![allow(dead_code)]
    automod::dir!("./tests/build_schema");
}

mod must_be_text {
    automod::dir!("./tests/must_be_text");
}

#[test]
fn compile_fail_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/compile_pass/*.rs");
    t.compile_fail("tests/compile_fail/*.rs");
}
