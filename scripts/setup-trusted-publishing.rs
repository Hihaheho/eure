#!/usr/bin/env -S cargo +nightly -Zscript
---
[dependencies]
clap = { version = "4", features = ["derive"] }
reqwest = { version = "0.12", features = ["blocking", "json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
---

use clap::{Parser, Subcommand};
use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::process::Command;

const REPOSITORY_OWNER: &str = "Hihaheho";
const REPOSITORY_NAME: &str = "eure";
const WORKFLOW_FILENAME: &str = "release.yml";
const ENVIRONMENT: &str = "release";

#[derive(Parser)]
#[command(name = "setup-trusted-publishing")]
#[command(about = "Setup trusted publishing for crates.io")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Setup GitHub Actions trusted publishing config
    Config {
        /// Crates to configure (default: all publishable crates)
        #[arg(value_name = "CRATE")]
        crates: Vec<String>,
    },
    /// Enable trustpub_only (require trusted publishing for all new versions)
    TrustpubOnly {
        /// Crates to configure (default: all publishable crates)
        #[arg(value_name = "CRATE")]
        crates: Vec<String>,
    },
    /// Setup both config and trustpub_only
    All {
        /// Crates to configure (default: all publishable crates)
        #[arg(value_name = "CRATE")]
        crates: Vec<String>,
    },
}

#[derive(Debug, Deserialize)]
struct CargoMetadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Package {
    name: String,
    id: String,
    publish: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct GitHubConfigsResponse {
    github_configs: Vec<GitHubConfig>,
}

#[derive(Debug, Deserialize)]
struct GitHubConfig {
    repository_owner: String,
    repository_name: String,
    workflow_filename: String,
    environment: Option<String>,
}

#[derive(Debug, Serialize)]
struct CreateConfigRequest {
    github_config: NewGitHubConfig,
}

#[derive(Debug, Serialize)]
struct NewGitHubConfig {
    #[serde(rename = "crate")]
    crate_name: String,
    repository_owner: String,
    repository_name: String,
    workflow_filename: String,
    environment: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    errors: Vec<ErrorDetail>,
}

#[derive(Debug, Deserialize)]
struct ErrorDetail {
    detail: String,
}

#[derive(Debug, Deserialize)]
struct CrateResponse {
    #[serde(rename = "crate")]
    krate: CrateInfo,
}

#[derive(Debug, Deserialize)]
struct CrateInfo {
    trustpub_only: bool,
}

#[derive(Debug, Serialize)]
struct PatchCrateRequest {
    #[serde(rename = "crate")]
    krate: PatchCrateSettings,
}

#[derive(Debug, Serialize)]
struct PatchCrateSettings {
    trustpub_only: bool,
}

#[derive(Clone, Copy)]
struct Operations {
    config: bool,
    trustpub_only: bool,
}

fn main() {
    let cli = Cli::parse();

    let token = match std::env::var("CRATES_IO_TOKEN") {
        Ok(t) => t,
        Err(_) => {
            eprintln!("Error: CRATES_IO_TOKEN environment variable is not set");
            std::process::exit(1);
        }
    };

    let (ops, specified_crates) = match cli.command {
        Commands::Config { crates } => (
            Operations {
                config: true,
                trustpub_only: false,
            },
            crates,
        ),
        Commands::TrustpubOnly { crates } => (
            Operations {
                config: false,
                trustpub_only: true,
            },
            crates,
        ),
        Commands::All { crates } => (
            Operations {
                config: true,
                trustpub_only: true,
            },
            crates,
        ),
    };

    let target_crates = if specified_crates.is_empty() {
        get_publishable_crates()
    } else {
        specified_crates
    };

    println!("Processing {} crate(s)", target_crates.len());

    let client = Client::new();
    let mut config_created = 0;
    let mut config_skipped = 0;
    let mut trustpub_enabled = 0;
    let mut trustpub_skipped = 0;
    let mut errors = 0;

    for crate_name in &target_crates {
        print!("  {}: ", crate_name);
        match process_crate(&client, &token, crate_name, ops) {
            Ok(result) => {
                let mut actions = Vec::new();
                if ops.config {
                    if result.config_created {
                        actions.push("config created");
                        config_created += 1;
                    } else {
                        actions.push("config exists");
                        config_skipped += 1;
                    }
                }
                if ops.trustpub_only {
                    if result.trustpub_enabled {
                        actions.push("trustpub_only enabled");
                        trustpub_enabled += 1;
                    } else {
                        actions.push("trustpub_only already set");
                        trustpub_skipped += 1;
                    }
                }
                println!("{}", actions.join(", "));
            }
            Err(e) => {
                println!("ERROR: {}", e);
                errors += 1;
            }
        }
    }

    println!();
    println!("Summary:");
    if ops.config {
        println!(
            "  GitHub config: {} created, {} skipped",
            config_created, config_skipped
        );
    }
    if ops.trustpub_only {
        println!(
            "  trustpub_only: {} enabled, {} skipped",
            trustpub_enabled, trustpub_skipped
        );
    }
    println!("  Errors: {}", errors);

    if errors > 0 {
        std::process::exit(1);
    }
}

struct ProcessResult {
    config_created: bool,
    trustpub_enabled: bool,
}

fn get_publishable_crates() -> Vec<String> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version=1", "--no-deps"])
        .output()
        .expect("Failed to run cargo metadata");

    if !output.status.success() {
        eprintln!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::process::exit(1);
    }

    let metadata: CargoMetadata =
        serde_json::from_slice(&output.stdout).expect("Failed to parse cargo metadata");

    let workspace_member_ids: std::collections::HashSet<_> =
        metadata.workspace_members.into_iter().collect();

    metadata
        .packages
        .into_iter()
        .filter(|p| workspace_member_ids.contains(&p.id))
        .filter(|p| {
            // publish = None means publishable (default)
            // publish = Some([]) means publish = false
            // publish = Some(["registry"]) means publish to specific registries
            match &p.publish {
                None => true,
                Some(registries) => !registries.is_empty(),
            }
        })
        .map(|p| p.name)
        .collect()
}

fn process_crate(
    client: &Client,
    token: &str,
    crate_name: &str,
    ops: Operations,
) -> Result<ProcessResult, String> {
    let config_created = if ops.config {
        setup_github_config(client, token, crate_name)?
    } else {
        false
    };

    let trustpub_enabled = if ops.trustpub_only {
        enable_trustpub_only(client, token, crate_name)?
    } else {
        false
    };

    Ok(ProcessResult {
        config_created,
        trustpub_enabled,
    })
}

fn setup_github_config(client: &Client, token: &str, crate_name: &str) -> Result<bool, String> {
    // Check existing configs
    let url = format!(
        "https://crates.io/api/v1/trusted_publishing/github_configs?crate={}",
        crate_name
    );

    let response = client
        .get(&url)
        .header(USER_AGENT, "eure-trusted-publishing-setup")
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!(
            "GET configs failed with status {}: {}",
            status, body
        ));
    }

    let configs: GitHubConfigsResponse = response
        .json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    // Check if matching config already exists
    let matching_config = configs.github_configs.iter().find(|c| {
        c.repository_owner == REPOSITORY_OWNER
            && c.repository_name == REPOSITORY_NAME
            && c.workflow_filename == WORKFLOW_FILENAME
            && c.environment.as_deref() == Some(ENVIRONMENT)
    });

    if matching_config.is_some() {
        return Ok(false);
    }

    // Create new config
    let create_url = "https://crates.io/api/v1/trusted_publishing/github_configs";
    let request_body = CreateConfigRequest {
        github_config: NewGitHubConfig {
            crate_name: crate_name.to_string(),
            repository_owner: REPOSITORY_OWNER.to_string(),
            repository_name: REPOSITORY_NAME.to_string(),
            workflow_filename: WORKFLOW_FILENAME.to_string(),
            environment: Some(ENVIRONMENT.to_string()),
        },
    };

    let response = client
        .post(create_url)
        .header(USER_AGENT, "eure-trusted-publishing-setup")
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .header(CONTENT_TYPE, "application/json")
        .json(&request_body)
        .send()
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();

        // Try to parse error response for better messages
        if let Ok(error_resp) = serde_json::from_str::<ErrorResponse>(&body) {
            let details: Vec<_> = error_resp
                .errors
                .iter()
                .map(|e| e.detail.as_str())
                .collect();
            return Err(format!(
                "POST config failed with status {}: {}",
                status,
                details.join(", ")
            ));
        }

        return Err(format!(
            "POST config failed with status {}: {}",
            status, body
        ));
    }

    Ok(true)
}

fn enable_trustpub_only(client: &Client, token: &str, crate_name: &str) -> Result<bool, String> {
    // Check current crate settings
    let url = format!("https://crates.io/api/v1/crates/{}", crate_name);

    let response = client
        .get(&url)
        .header(USER_AGENT, "eure-trusted-publishing-setup")
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!(
            "GET crate failed with status {}: {}",
            status, body
        ));
    }

    let crate_resp: CrateResponse = response
        .json()
        .map_err(|e| format!("Failed to parse crate response: {}", e))?;

    if crate_resp.krate.trustpub_only {
        return Ok(false);
    }

    // Enable trustpub_only
    let request_body = PatchCrateRequest {
        krate: PatchCrateSettings { trustpub_only: true },
    };

    let response = client
        .patch(&url)
        .header(USER_AGENT, "eure-trusted-publishing-setup")
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .header(CONTENT_TYPE, "application/json")
        .json(&request_body)
        .send()
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();

        if let Ok(error_resp) = serde_json::from_str::<ErrorResponse>(&body) {
            let details: Vec<_> = error_resp
                .errors
                .iter()
                .map(|e| e.detail.as_str())
                .collect();
            return Err(format!(
                "PATCH crate failed with status {}: {}",
                status,
                details.join(", ")
            ));
        }

        return Err(format!(
            "PATCH crate failed with status {}: {}",
            status, body
        ));
    }

    Ok(true)
}
