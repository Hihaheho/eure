//! Custom test runner for the Eure test suite.
//!
//! This binary runs all test cases and reports results in a friendly format:
//! - basic/primitives 5/5 PASS
//! - schemas/array 2/3 FAIL

use std::path::Path;
use std::sync::mpsc;

use rayon::prelude::*;
use test_suite::{
    CaseResult, CollectCasesError, ScenarioResult, cases_dir, collect_cases, format_parse_error,
};

/// ANSI color codes
mod colors {
    pub const GREEN: &str = "\x1b[32m";
    pub const RED: &str = "\x1b[31m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const CYAN: &str = "\x1b[36m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";
    pub const RESET: &str = "\x1b[0m";
}

/// Result of a single test case execution
enum TestCaseOutcome {
    /// Case ran successfully with scenario results
    Ran {
        case_name: String,
        result: CaseResult,
    },
    /// Failed to parse the test case file
    ParseError { case_name: String, error: String },
}

/// Extract a friendly case name from the full path
fn case_name_from_path(path: &Path, cases_dir: &Path) -> String {
    path.strip_prefix(cases_dir)
        .unwrap_or(path)
        .with_extension("")
        .display()
        .to_string()
}

fn main() {
    let exit_code = run();
    std::process::exit(exit_code);
}

fn run() -> i32 {
    println!(
        "\n{}{}Eure Test Suite{}",
        colors::BOLD,
        colors::CYAN,
        colors::RESET
    );
    println!("{}{}", colors::DIM, "=".repeat(50));
    println!("{}\n", colors::RESET);

    let cases_base_dir = cases_dir();
    let cases = match collect_cases() {
        Ok(cases) => cases,
        Err(e) => {
            eprintln!(
                "{}{}Error:{} Failed to collect test cases: {}",
                colors::BOLD,
                colors::RED,
                colors::RESET,
                e
            );
            return 1;
        }
    };

    if cases.is_empty() {
        println!(
            "{}{}Warning:{} No test cases found in {}",
            colors::BOLD,
            colors::YELLOW,
            colors::RESET,
            cases_base_dir.display()
        );
        return 0;
    }

    let (tx, rx) = mpsc::channel();

    // Run tests in parallel
    std::thread::scope(|s| {
        s.spawn(|| {
            cases.par_iter().for_each_with(tx, |tx, case_result| {
                let outcome = match case_result {
                    Ok(parse_result) => {
                        let case_name =
                            case_name_from_path(&parse_result.case.path, &cases_base_dir);
                        let preprocessed = parse_result.case.preprocess();
                        let result = preprocessed.run_all();
                        TestCaseOutcome::Ran { case_name, result }
                    }
                    Err(collect_error) => {
                        let (path, error_msg) = match collect_error {
                            CollectCasesError::IoError { path, error } => {
                                (path.clone(), format!("IO error: {}", error))
                            }
                            CollectCasesError::ParseError { path, error, input } => {
                                let msg = format_parse_error(error, input, path);
                                (path.clone(), msg)
                            }
                        };
                        let case_name = case_name_from_path(&path, &cases_base_dir);
                        TestCaseOutcome::ParseError {
                            case_name,
                            error: error_msg,
                        }
                    }
                };
                tx.send(outcome).unwrap();
            });
        });

        // Collect results
        let mut outcomes: Vec<TestCaseOutcome> = rx.iter().collect();

        // Sort by case name for consistent output
        outcomes.sort_by(|a, b| {
            let name_a = match a {
                TestCaseOutcome::Ran { case_name, .. } => case_name,
                TestCaseOutcome::ParseError { case_name, .. } => case_name,
            };
            let name_b = match b {
                TestCaseOutcome::Ran { case_name, .. } => case_name,
                TestCaseOutcome::ParseError { case_name, .. } => case_name,
            };
            name_a.cmp(name_b)
        });

        // Display results
        let mut total_passed = 0;
        let mut total_failed = 0;
        let mut total_scenarios_passed = 0;
        let mut total_scenarios = 0;
        let mut failures: Vec<(String, Vec<String>)> = Vec::new();

        for outcome in &outcomes {
            match outcome {
                TestCaseOutcome::Ran { case_name, result } => {
                    let passed = result.passed_count();
                    let total = result.total_count();
                    total_scenarios_passed += passed;
                    total_scenarios += total;

                    if result.all_passed() {
                        println!(
                            "  {}{}PASS{} {} {}{}/{}{}",
                            colors::BOLD,
                            colors::GREEN,
                            colors::RESET,
                            case_name,
                            colors::DIM,
                            passed,
                            total,
                            colors::RESET
                        );
                        total_passed += 1;
                    } else {
                        println!(
                            "  {}{}FAIL{} {} {}{}/{}{}",
                            colors::BOLD,
                            colors::RED,
                            colors::RESET,
                            case_name,
                            colors::DIM,
                            passed,
                            total,
                            colors::RESET
                        );
                        total_failed += 1;

                        // Collect failure details
                        let failed_details: Vec<String> = result
                            .failed_scenarios()
                            .iter()
                            .map(|s| {
                                let error = match &s.result {
                                    ScenarioResult::Failed { error } => error.clone(),
                                    _ => "Unknown error".to_string(),
                                };
                                format!("    - {}: {}", s.name, error)
                            })
                            .collect();
                        failures.push((case_name.clone(), failed_details));
                    }
                }
                TestCaseOutcome::ParseError { case_name, error } => {
                    println!(
                        "  {}{}PARSE ERROR{} {}",
                        colors::BOLD,
                        colors::RED,
                        colors::RESET,
                        case_name
                    );
                    total_failed += 1;
                    failures.push((case_name.clone(), vec![error.clone()]));
                }
            }
        }

        // Print summary
        println!("\n{}{}Summary{}", colors::BOLD, colors::CYAN, colors::RESET);
        println!("{}{}", colors::DIM, "-".repeat(50));
        println!("{}", colors::RESET);

        println!(
            "  Cases:     {} passed, {} failed, {} total",
            total_passed,
            total_failed,
            total_passed + total_failed
        );
        println!(
            "  Scenarios: {} passed, {} failed, {} total",
            total_scenarios_passed,
            total_scenarios - total_scenarios_passed,
            total_scenarios
        );

        // Print detailed failure reports
        if !failures.is_empty() {
            println!("\n{}{}Failures{}", colors::BOLD, colors::RED, colors::RESET);
            println!("{}{}", colors::DIM, "-".repeat(50));
            println!("{}", colors::RESET);

            for (case_name, details) in &failures {
                println!(
                    "\n  {}{}{}{}",
                    colors::BOLD,
                    colors::RED,
                    case_name,
                    colors::RESET
                );
                for detail in details {
                    // Indent multi-line errors properly
                    let indented = detail
                        .lines()
                        .enumerate()
                        .map(|(i, line)| {
                            if i == 0 {
                                line.to_string()
                            } else {
                                format!("      {}", line)
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    println!("{}", indented);
                }
            }
        }

        // Final status line
        println!();
        if total_failed == 0 {
            println!(
                "{}{}All tests passed!{}",
                colors::BOLD,
                colors::GREEN,
                colors::RESET
            );
            0
        } else {
            println!(
                "{}{}{} test(s) failed.{}",
                colors::BOLD,
                colors::RED,
                total_failed,
                colors::RESET
            );
            1
        }
    })
}
