use std::path::PathBuf;

use eure::{
    document::{DocumentConstructionError, EureDocument},
    parol::parol_runtime::ParolError,
    tree::Cst,
    value::Code,
};

pub struct Case {
    pub path: PathBuf,
    pub input_eure: Option<Code>,
    pub normalized: Option<Code>,
    pub output_json: Option<Code>,
}

pub struct PreprocessedCase {
    pub input_eure: Option<PreprocessedEure>,
    pub normalized: Option<PreprocessedEure>,
    pub output_json: Option<serde_json::Value>,
}

pub enum PreprocessedEure {
    Ok {
        cst: Cst,
        doc: EureDocument,
    },
    ErrParol(ParolError),
    ErrDocument {
        cst: Cst,
        error: DocumentConstructionError,
    },
}

impl PreprocessedEure {
    pub fn status(&self) -> String {
        match self {
            PreprocessedEure::Ok { .. } => "OK".to_string(),
            PreprocessedEure::ErrParol(e) => format!("PARSE_ERROR({})", e),
            PreprocessedEure::ErrDocument { error, .. } => format!("DOC_ERROR({})", error),
        }
    }

    pub fn is_ok(&self) -> bool {
        matches!(self, PreprocessedEure::Ok { .. })
    }

    pub fn cst(&self) -> eros::Result<&Cst> {
        match self {
            PreprocessedEure::Ok { cst, .. } => Ok(cst),
            PreprocessedEure::ErrDocument { cst, .. } => Ok(cst),
            PreprocessedEure::ErrParol(e) => Err(eros::traced!("{}", e)),
        }
    }

    pub fn doc(&self) -> eros::Result<&EureDocument> {
        match self {
            PreprocessedEure::Ok { doc, .. } => Ok(doc),
            PreprocessedEure::ErrParol(e) => Err(eros::traced!("{}", e)),
            PreprocessedEure::ErrDocument { error, .. } => Err(eros::traced!("{}", error.clone())),
        }
    }
}

impl Case {
    fn preprocess_eure(code: &str) -> PreprocessedEure {
        match eure::parol::parse(code) {
            Ok(cst) => match eure::document::cst_to_document(code, &cst) {
                Ok(doc) => PreprocessedEure::Ok { cst, doc },
                Err(e) => PreprocessedEure::ErrDocument { cst, error: e },
            },
            Err(e) => PreprocessedEure::ErrParol(e),
        }
    }
    pub fn preprocess(&self) -> PreprocessedCase {
        let input_eure = self
            .input_eure
            .as_ref()
            .map(|input_eure| Self::preprocess_eure(input_eure.as_str()));
        let normalized = self
            .normalized
            .as_ref()
            .map(|normalized| Self::preprocess_eure(normalized.as_str()));
        let output_json = self
            .output_json
            .as_ref()
            .map(|code| serde_json::from_str(code.as_str()).unwrap());

        PreprocessedCase {
            input_eure,
            normalized,
            output_json,
        }
    }
}

pub struct NormalizationScenario<'a> {
    input: &'a PreprocessedEure,
    normalized: &'a PreprocessedEure,
}

impl NormalizationScenario<'_> {
    pub fn run(&self) -> eros::Result<()> {
        let input_doc = self.input.doc()?;
        let normalized_doc = self.normalized.doc()?;
        assert_eq!(input_doc, normalized_doc);
        Ok(())
    }
}

pub struct EureToJsonScenario<'a> {
    input: &'a PreprocessedEure,
    output_json: &'a serde_json::Value,
    source: &'static str,
}

impl EureToJsonScenario<'_> {
    pub fn run(&self) -> eros::Result<()> {
        let input_doc = self.input.doc()?;
        let output_json = self.output_json;
        assert_eq!(
            eure_json::document_to_value(input_doc, &eure_json::Config::default()).unwrap(),
            *output_json
        );
        Ok(())
    }
}

impl PreprocessedCase {
    pub fn run(&self) -> eros::Result<()> {
        let trace = std::env::var("EURE_TEST_TRACE").is_ok();

        if trace {
            eprintln!("\n=== PreprocessedCase Debug Trace ===");
            eprintln!(
                "input_eure: {}",
                self.input_eure
                    .as_ref()
                    .map_or("None".to_string(), |e| e.status().to_string())
            );
            eprintln!(
                "normalized: {}",
                self.normalized
                    .as_ref()
                    .map_or("None".to_string(), |e| e.status().to_string())
            );
            eprintln!(
                "output_json: {}",
                if self.output_json.is_some() {
                    "Some"
                } else {
                    "None"
                }
            );
        }

        if let Some(normalization_scenario) = self.normalization_scenario() {
            if trace {
                eprintln!("\n--- Running Normalization Scenario ---");
            }
            normalization_scenario.run()?;
            if trace {
                eprintln!("✓ Normalization scenario passed");
            }
        } else if trace {
            eprintln!("\n--- Normalization Scenario: SKIPPED (missing fields) ---");
        }

        let json_scenarios = self.eure_to_json_scenario();
        if trace {
            eprintln!(
                "\n--- EureToJson Scenarios: {} total ---",
                json_scenarios.len()
            );
        }

        for (i, scenario) in json_scenarios.iter().enumerate() {
            if trace {
                eprintln!(
                    "Running scenario {} (source: {}): input status = {}",
                    i + 1,
                    scenario.source,
                    scenario.input.status()
                );
            }
            scenario.run()?;
            if trace {
                eprintln!("✓ Scenario {} passed", i + 1);
            }
        }

        if trace {
            eprintln!("=== End Debug Trace ===\n");
        }

        Ok(())
    }

    pub fn normalization_scenario(&self) -> Option<NormalizationScenario<'_>> {
        match (&self.input_eure, &self.normalized) {
            (Some(input), Some(normalized)) => Some(NormalizationScenario { input, normalized }),
            _ => None,
        }
    }

    pub fn eure_to_json_scenario(&self) -> Vec<EureToJsonScenario<'_>> {
        let mut scenarios = Vec::new();
        if let (Some(input), Some(output_json)) = (&self.input_eure, &self.output_json) {
            scenarios.push(EureToJsonScenario {
                input,
                output_json,
                source: "input_eure",
            });
        }
        if let (Some(normalized), Some(output_json)) = (&self.normalized, &self.output_json) {
            scenarios.push(EureToJsonScenario {
                input: normalized,
                output_json,
                source: "normalized",
            });
        }

        scenarios
    }
}
