use std::path::PathBuf;

use test_suite::case_schema::{CaseSchemaError, case_schema_path, generate_case_schema_source};
use thiserror::Error;

#[derive(Debug, Error)]
enum GenerateCaseSchemaError {
    #[error(transparent)]
    Schema(#[from] CaseSchemaError),
    #[error("failed to write schema file at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), GenerateCaseSchemaError> {
    let schema_path = case_schema_path();
    let source = generate_case_schema_source()?;
    std::fs::write(&schema_path, source).map_err(|source| GenerateCaseSchemaError::Io {
        path: schema_path.clone(),
        source,
    })?;
    println!("{}", schema_path.display());
    Ok(())
}
