use test_suite::TestRunner;

#[test]
fn run_all_test_cases() {
    let runner = TestRunner::new("cases");
    let results = runner.run_all().expect("Failed to run test suite");

    // Print results
    println!("\n=== Test Suite Results ===");
    println!("Total: {}", results.total);
    println!("Passed: {}", results.passed);
    println!("Failed: {}", results.failed);

    if !results.failures.is_empty() {
        println!("\n=== Failures ===");
        for failure in &results.failures {
            println!("\n[FAIL] {}", failure.test_name);
            println!("  {}", failure.error);
        }
    }

    assert!(
        results.is_success(),
        "Test suite failed with {} failures",
        results.failed
    );
}
