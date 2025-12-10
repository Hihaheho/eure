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
    Case, CaseResult, CollectCasesError, RunConfig, ScenarioResult, cases_dir, collect_cases,
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

    /// Treat unimplemented cases as failures instead of TODOs
    #[arg(short, long)]
    all: bool,
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
struct CaseOutcome {
    case_name: String,
    result: CaseResult,
    status_info: Option<String>,
    unimplemented: Option<String>,
}

/// Result of a test file execution (may contain multiple cases)
enum TestFileOutcome {
    /// File ran successfully with case results
    Ran {
        file_name: String,
        cases: Vec<CaseOutcome>,
    },
    /// Failed to parse the test file
    ParseError { file_name: String, error: String },
}

/// Extract a friendly file name from the full path
fn file_name_from_path(path: &Path, cases_dir: &Path) -> String {
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
                    Ok(parse_result) => &parse_result.case_file.path,
                    Err(CollectCasesError::IoError { path, .. }) => path,
                    Err(CollectCasesError::ParseError { path, .. }) => path,
                };
                let file_name = file_name_from_path(path, &cases_base_dir);
                file_name.contains(filter.as_str())
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
                        let file_name =
                            file_name_from_path(&parse_result.case_file.path, &cases_base_dir);
                        let case_file = &parse_result.case_file;

                        // Process all cases in the file
                        let case_outcomes: Vec<CaseOutcome> = case_file
                            .all_cases()
                            .map(|(name, case_data)| {
                                let case = Case::new(
                                    case_file.path.clone(),
                                    name.to_string(),
                                    case_data.clone(),
                                );
                                let unimplemented = case.data.unimplemented.clone();
                                let preprocessed = case.preprocess();
                                let status_info = {
                                    let summary = preprocessed.status_summary();
                                    if summary.is_empty() {
                                        None
                                    } else {
                                        Some(summary)
                                    }
                                };
                                let result = preprocessed.run_all(&config);

                                CaseOutcome {
                                    case_name: name.to_string(),
                                    result,
                                    status_info,
                                    unimplemented,
                                }
                            })
                            .collect();

                        TestFileOutcome::Ran {
                            file_name,
                            cases: case_outcomes,
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
                        let file_name = file_name_from_path(&path, &cases_base_dir);
                        TestFileOutcome::ParseError {
                            file_name,
                            error: error_msg,
                        }
                    }
                };
                tx.send(outcome).unwrap();
            });
        });

        // Collect results
        let mut outcomes: Vec<TestFileOutcome> = rx.iter().collect();

        // Sort by file name for consistent output
        outcomes.sort_by(|a, b| {
            let name_a = match a {
                TestFileOutcome::Ran { file_name, .. } => file_name,
                TestFileOutcome::ParseError { file_name, .. } => file_name,
            };
            let name_b = match b {
                TestFileOutcome::Ran { file_name, .. } => file_name,
                TestFileOutcome::ParseError { file_name, .. } => file_name,
            };
            name_a.cmp(name_b)
        });

        // Display results
        let mut total_passed = 0;
        let mut total_failed = 0;
        let mut total_scenarios_passed = 0;
        let mut total_scenarios = 0;
        let mut unimplemented_cases: Vec<(String, bool)> = Vec::new();
        let mut failures: Vec<(String, Vec<FailureDetail>)> = Vec::new();

        for outcome in &outcomes {
            match outcome {
                TestFileOutcome::Ran { file_name, cases } => {
                    let is_multi_case = cases.len() > 1;

                    if is_multi_case {
                        // Calculate aggregate status for multi-case file header
                        // Priority: FAIL > TODO > PASS
                        // In strict mode (--all), unimplemented cases are treated as regular cases
                        let has_fail = cases.iter().any(|c| {
                            let is_unimpl = !args.all && c.unimplemented.is_some();
                            !is_unimpl && !c.result.all_passed()
                        });
                        let has_todo = !args.all && cases.iter().any(|c| c.unimplemented.is_some());

                        let (header_status, header_color) = if has_fail {
                            ("FAIL", colors::RED)
                        } else if has_todo {
                            ("TODO", colors::YELLOW)
                        } else {
                            ("PASS", colors::GREEN)
                        };

                        // Nested display for multi-case files
                        println!(
                            "  {}{}{}{} {}",
                            colors::BOLD,
                            header_color,
                            header_status,
                            colors::RESET,
                            file_name
                        );

                        for case_outcome in cases {
                            display_case_outcome(
                                case_outcome,
                                file_name,
                                true, // nested
                                args.all,
                                &mut total_passed,
                                &mut total_failed,
                                &mut total_scenarios_passed,
                                &mut total_scenarios,
                                &mut unimplemented_cases,
                                &mut failures,
                            );
                        }
                    } else if let Some(case_outcome) = cases.first() {
                        // Single case: display on one line
                        display_case_outcome(
                            case_outcome,
                            file_name,
                            false, // not nested
                            args.all,
                            &mut total_passed,
                            &mut total_failed,
                            &mut total_scenarios_passed,
                            &mut total_scenarios,
                            &mut unimplemented_cases,
                            &mut failures,
                        );
                    }
                }
                TestFileOutcome::ParseError { file_name, error } => {
                    println!(
                        "  {}{}PARSE ERROR{} {}",
                        colors::BOLD,
                        colors::RED,
                        colors::RESET,
                        file_name
                    );
                    total_failed += 1;
                    failures.push((
                        file_name.clone(),
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

#[allow(clippy::too_many_arguments)]
fn display_case_outcome(
    case_outcome: &CaseOutcome,
    file_name: &str,
    nested: bool,
    strict_mode: bool,
    total_passed: &mut usize,
    total_failed: &mut usize,
    total_scenarios_passed: &mut usize,
    total_scenarios: &mut usize,
    unimplemented_cases: &mut Vec<(String, bool)>,
    failures: &mut Vec<(String, Vec<FailureDetail>)>,
) {
    let passed = case_outcome.result.passed_count();
    let total = case_outcome.result.total_count();
    *total_scenarios_passed += passed;
    *total_scenarios += total;

    // Build case identifier
    let case_id = if nested {
        if case_outcome.case_name.is_empty() {
            format!("{}[root]", file_name)
        } else {
            format!("{}[{}]", file_name, case_outcome.case_name)
        }
    } else {
        file_name.to_string()
    };

    // Build display name for output
    let display_name = if nested {
        if case_outcome.case_name.is_empty() {
            "[root]".to_string()
        } else {
            format!("[{}]", case_outcome.case_name)
        }
    } else {
        file_name.to_string()
    };

    // In strict mode (--all), treat unimplemented cases as regular cases
    let effective_unimplemented = if strict_mode {
        None
    } else {
        case_outcome.unimplemented.as_ref()
    };

    // Determine status
    let (status_text, color, unimpl_annotation) = if let Some(reason) = effective_unimplemented {
        let annotation = if reason.is_empty() {
            String::new()
        } else {
            format!(" (\"{}\")", reason)
        };
        ("TODO", colors::YELLOW, annotation)
    } else if case_outcome.result.all_passed() {
        ("PASS", colors::GREEN, String::new())
    } else {
        ("FAIL", colors::RED, String::new())
    };

    // Print with appropriate indentation
    let indent = if nested { "    " } else { "  " };
    println!(
        "{}{}{}{}{} {} {}{}/{}{}{}",
        indent,
        colors::BOLD,
        color,
        status_text,
        colors::RESET,
        display_name,
        colors::DIM,
        passed,
        total,
        colors::RESET,
        unimpl_annotation
    );

    // Track for summary
    if effective_unimplemented.is_some() {
        unimplemented_cases.push((case_id.clone(), case_outcome.result.all_passed()));
    } else if case_outcome.result.all_passed() {
        *total_passed += 1;
    } else {
        *total_failed += 1;

        // Collect failure details
        let failed_details: Vec<FailureDetail> = case_outcome
            .result
            .failed_scenarios()
            .iter()
            .map(|s| {
                let error = match &s.result {
                    ScenarioResult::Failed { error } => error.clone(),
                    _ => "Unknown error".to_string(),
                };
                let short = format!("{}: {}", s.name, error);
                let detailed = if let Some(ref info) = case_outcome.status_info {
                    format!("{}\n\n{}", info, error)
                } else {
                    error
                };
                FailureDetail { short, detailed }
            })
            .collect();
        failures.push((case_id, failed_details));
    }
}
