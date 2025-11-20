pub mod case;
pub mod parser;

use std::fs;
use std::path::{Path, PathBuf};

use annotate_snippets::{Level, Renderer, Snippet};
use eure::tree::LineNumbers;

use crate::parser::{ParseError, ParseResult, parse_case};

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum CollectCasesError {
    IoError {
        path: PathBuf,
        error: std::io::Error,
    },
    ParseError {
        path: PathBuf,
        error: ParseError,
        input: String,
    },
}

/// Format a ParseError using annotate-snippets
pub fn format_parse_error(error: &ParseError, input: &str, path: &Path) -> String {
    match error {
        ParseError::ParolError(e) => {
            format!("Parse error: {}\n  --> {}\n", e, path.display())
        }
        ParseError::DocumentConstructionError { error, cst } => {
            let line_numbers = LineNumbers::new(input);

            // Try to get the span from the error
            let span_opt = error.span(cst);

            if let Some(span) = span_opt {
                let start_info = line_numbers.get_char_info(span.start);

                let report = Level::ERROR.primary_title(error.to_string()).element(
                    Snippet::source(input)
                        .line_start((start_info.line_number + 1) as usize)
                        .path(path.display().to_string())
                        .annotation(
                            annotate_snippets::AnnotationKind::Primary
                                .span((span.start as usize)..(span.end as usize))
                                .label(error.to_string()),
                        ),
                );

                let renderer = Renderer::styled();
                renderer.render(&[report]).to_string()
            } else {
                // No span information available, just display the error
                format!("error: {}\n  --> {}\n", error, path.display())
            }
        }
        ParseError::IdentifierError { error, .. } => {
            format!("Identifier error: {}\n  --> {}\n", error, path.display())
        }
    }
}

/// Collect all cases from the `cases` directory.
/// Returns a vector of results, where each result is either a successfully parsed case
/// or a parse error. This allows the test suite to continue even if some cases fail to parse.
pub fn collect_cases() -> Result<Vec<Result<ParseResult, CollectCasesError>>, std::io::Error> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let cases_dir = Path::new(manifest_dir).join("cases");
    let mut cases = Vec::new();
    collect_cases_recursive(&cases_dir, &mut cases)?;
    Ok(cases)
}

fn collect_cases_recursive(
    dir: &Path,
    cases: &mut Vec<Result<ParseResult, CollectCasesError>>,
) -> Result<(), std::io::Error> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_cases_recursive(&path, cases)?;
        } else if path.extension().is_some_and(|ext| ext == "eure") {
            let content = match fs::read_to_string(&path) {
                Ok(content) => content,
                Err(e) => {
                    cases.push(Err(CollectCasesError::IoError {
                        path: path.clone(),
                        error: e,
                    }));
                    continue;
                }
            };
            let case_result = parse_case(&content, path.clone());
            cases.push(case_result.map_err(|e| CollectCasesError::ParseError {
                path: path.clone(),
                error: e,
                input: content,
            }));
        }
    }
    Ok(())
}

#[test]
fn run_test_suite() {
    use rayon::prelude::*;
    use std::sync::mpsc;

    enum TestResult {
        Passed,
        ParseError(String),
        ExecutionError(String),
        Panicked(String),
    }

    let cases = collect_cases().unwrap();
    let (tx, rx) = mpsc::channel();

    std::thread::scope(|s| {
        s.spawn(|| {
            cases.par_iter().for_each_with(tx, |tx, case_result| {
                let (path, result) = match case_result {
                    Ok(parse_result) => {
                        let path = parse_result.case.path.clone();
                        let result =
                            match std::panic::catch_unwind(|| parse_result.case.preprocess().run())
                            {
                                Ok(Ok(())) => TestResult::Passed,
                                Ok(Err(e)) => TestResult::ExecutionError(format!("{:?}", e)),
                                Err(e) => {
                                    let panic_msg = if let Some(s) = e.downcast_ref::<&str>() {
                                        s.to_string()
                                    } else if let Some(s) = e.downcast_ref::<String>() {
                                        s.clone()
                                    } else {
                                        "Unknown panic".to_string()
                                    };
                                    TestResult::Panicked(panic_msg)
                                }
                            };
                        (path, result)
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

                        let result = TestResult::ParseError(error_msg);
                        (path, result)
                    }
                };
                tx.send((path, result)).unwrap();
            });
        });

        let mut passed = 0;
        let mut failed = 0;

        for (path, result) in rx {
            match result {
                TestResult::Passed => {
                    println!("Test case passed: {}", path.display());
                    passed += 1;
                }
                TestResult::ParseError(e) => {
                    println!("Test case failed:\n{}", e);
                    failed += 1;
                }
                TestResult::ExecutionError(e) => {
                    println!(
                        "Test case failed: {} with execution error:\n{}",
                        path.display(),
                        e
                    );
                    failed += 1;
                }
                TestResult::Panicked(e) => {
                    println!("Test case failed: {} with panic:\n{}", path.display(), e);
                    failed += 1;
                }
            }
        }

        println!(
            "\nTest suite finished. Passed: {}, Failed: {}",
            passed, failed
        );
        if failed > 0 {
            panic!("{} test cases failed", failed);
        }
    });
}
