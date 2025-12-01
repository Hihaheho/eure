//! Custom test runner for the Eure test suite.
//!
//! This binary runs all test cases and reports results in a friendly format:
//! - basic/primitives 5/5 PASS
//! - schemas/array 2/3 FAIL

use std::path::Path;
use std::sync::mpsc;

use clap::Parser;
use rayon::prelude::*;
use test_suite::{
    CaseResult, CollectCasesError, RunConfig, ScenarioResult, cases_dir, collect_cases,
    format_parse_error,
};

#[derive(Parser)]
#[command(name = "test-suite", about = "Eure test suite runner")]
struct Args {
    /// Enable trace output for debugging
    #[arg(short, long)]
    trace: bool,

    /// Filter tests by name pattern (substring match)
    #[arg(short, long)]
    filter: Option<String>,

    /// Show short error summaries instead of detailed output
    #[arg(short, long)]
    short: bool,
}

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

/// Detailed failure information
struct FailureDetail {
    /// Short one-line description
    short: String,
    /// Detailed multi-line description
    detailed: String,
}

/// Result of a single test case execution
enum TestCaseOutcome {
    /// Case ran successfully with scenario results
    Ran {
        case_name: String,
        result: CaseResult,
        /// Status info for detailed error reporting
        status_info: Option<String>,
        /// Unimplemented flag and optional reason
        unimplemented: Option<String>,
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
    let args = Args::parse();
    let exit_code = run(&args);
    std::process::exit(exit_code);
}

fn run(args: &Args) -> i32 {
    let config = RunConfig { trace: args.trace };

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

    // Filter cases by name if --filter is specified
    let cases: Vec<_> = if let Some(ref filter) = args.filter {
        cases
            .into_iter()
            .filter(|case_result| {
                let path = match case_result {
                    Ok(parse_result) => &parse_result.case.path,
                    Err(CollectCasesError::IoError { path, .. }) => path,
                    Err(CollectCasesError::ParseError { path, .. }) => path,
                };
                let case_name = case_name_from_path(path, &cases_base_dir);
                case_name.contains(filter.as_str())
            })
            .collect()
    } else {
        cases
    };

    if cases.is_empty() {
        println!(
            "{}{}Warning:{} No test cases found{}",
            colors::BOLD,
            colors::YELLOW,
            colors::RESET,
            if args.filter.is_some() {
                " matching filter"
            } else {
                ""
            }
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
                        let unimplemented = parse_result.case.unimplemented.clone();
                        let preprocessed = parse_result.case.preprocess();
                        let status_info = {
                            let summary = preprocessed.status_summary();
                            if summary.is_empty() {
                                None
                            } else {
                                Some(summary)
                            }
                        };
                        let result = preprocessed.run_all(&config);
                        TestCaseOutcome::Ran {
                            case_name,
                            result,
                            status_info,
                            unimplemented,
                        }
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
        let mut unimplemented_cases: Vec<(String, bool)> = Vec::new(); // (name, all_passed)
        let mut failures: Vec<(String, Vec<FailureDetail>)> = Vec::new();

        for outcome in &outcomes {
            match outcome {
                TestCaseOutcome::Ran {
                    case_name,
                    result,
                    status_info,
                    unimplemented,
                } => {
                    let passed = result.passed_count();
                    let total = result.total_count();
                    total_scenarios_passed += passed;
                    total_scenarios += total;

                    // Determine base status (PASS/FAIL)
                    let (status_text, color) = if result.all_passed() {
                        ("PASS", colors::GREEN)
                    } else {
                        ("FAIL", colors::RED)
                    };

                    // Build unimplemented annotation
                    let unimpl_annotation = if let Some(reason) = unimplemented {
                        if reason.is_empty() {
                            " (unimplemented)".to_string()
                        } else {
                            format!(" (unimplemented: \"{}\")", reason)
                        }
                    } else {
                        String::new()
                    };

                    println!(
                        "  {}{}{}{} {} {}{}/{}{}{}",
                        colors::BOLD,
                        color,
                        status_text,
                        colors::RESET,
                        case_name,
                        colors::DIM,
                        passed,
                        total,
                        colors::RESET,
                        unimpl_annotation
                    );

                    // Track for summary and warnings
                    if unimplemented.is_some() {
                        unimplemented_cases.push((case_name.clone(), result.all_passed()));
                        // Don't count as failed even if scenarios fail
                    } else if result.all_passed() {
                        total_passed += 1;
                    } else {
                        total_failed += 1;

                        // Collect failure details
                        let failed_details: Vec<FailureDetail> = result
                            .failed_scenarios()
                            .iter()
                            .map(|s| {
                                let error = match &s.result {
                                    ScenarioResult::Failed { error } => error.clone(),
                                    _ => "Unknown error".to_string(),
                                };
                                let short = format!("{}: {}", s.name, error);
                                let detailed = if let Some(info) = status_info {
                                    format!("{}\n\n{}", info, error)
                                } else {
                                    error
                                };
                                FailureDetail { short, detailed }
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
                    failures.push((
                        case_name.clone(),
                        vec![FailureDetail {
                            short: "Parse error".to_string(),
                            detailed: error.clone(),
                        }],
                    ));
                }
            }
        }

        // Print summary
        println!("\n{}{}Summary{}", colors::BOLD, colors::CYAN, colors::RESET);
        println!("{}{}", colors::DIM, "-".repeat(50));
        println!("{}", colors::RESET);

        println!(
            "  Cases:     {} passed, {} failed, {} unimplemented, {} total",
            total_passed,
            total_failed,
            unimplemented_cases.len(),
            total_passed + total_failed + unimplemented_cases.len()
        );
        println!(
            "  Scenarios: {} passed, {} failed, {} total",
            total_scenarios_passed,
            total_scenarios - total_scenarios_passed,
            total_scenarios
        );

        // Show warning for fully passing unimplemented cases
        let fully_passing_unimpl: Vec<_> = unimplemented_cases
            .iter()
            .filter(|(_, all_passed)| *all_passed)
            .collect();

        if !fully_passing_unimpl.is_empty() {
            println!(
                "\n{}Note:{} {} unimplemented case(s) have all scenarios passing",
                colors::BOLD,
                colors::RESET,
                fully_passing_unimpl.len()
            );
        }

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
                    let text = if args.short {
                        &detail.short
                    } else {
                        &detail.detailed
                    };
                    // Indent all lines
                    for line in text.lines() {
                        println!("    {}", line);
                    }
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
