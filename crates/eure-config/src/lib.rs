//! Configuration types for Eure tools.
//!
//! This crate provides configuration data types for the Eure CLI and Language Server.
//! The configuration is stored in `Eure.eure` files at project roots.
//!
//! This crate only defines data structures. Query logic and error reporting
//! are in the `eure` crate.
//!
//! # Features
//!
//! - `lint` - Include lint configuration types
//! - `ls` - Include language server configuration types
//! - `cli` - Include CLI configuration (enables `lint` and `ls`)
//! - `all` - Include all configuration types

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use eure_document::parse::{ParseContext, ParseDocument, ParseError};
use eure_macros::ParseDocument;
use eure_parol::EureParseError;

/// The standard configuration filename.
pub const CONFIG_FILENAME: &str = "Eure.eure";

/// Error type for configuration parsing.
///
/// Note: Document construction errors are handled separately in the eure crate.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Syntax error: {0}")]
    Syntax(EureParseError),

    #[error("Config error: {0}")]
    Parse(#[from] ParseError),
}

impl PartialEq for ConfigError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ConfigError::Io(a), ConfigError::Io(b)) => a.kind() == b.kind(),
            (ConfigError::Syntax(a), ConfigError::Syntax(b)) => a.to_string() == b.to_string(),
            (ConfigError::Parse(a), ConfigError::Parse(b)) => a == b,
            _ => false,
        }
    }
}

impl From<EureParseError> for ConfigError {
    fn from(err: EureParseError) -> Self {
        ConfigError::Syntax(err)
    }
}

/// A check target definition.
#[derive(Debug, Clone, ParseDocument, PartialEq)]
#[eure(crate = eure_document, allow_unknown_fields)]
pub struct Target {
    /// Glob patterns for files to include in this target.
    pub globs: Vec<String>,
    /// Optional schema file path (relative to config file).
    #[eure(default)]
    pub schema: Option<String>,
}

/// CLI-specific configuration.
#[cfg(feature = "cli")]
#[derive(Debug, Clone, Default, ParseDocument, PartialEq)]
#[eure(crate = eure_document, rename_all = "kebab-case", allow_unknown_fields)]
pub struct CliConfig {
    /// Default targets to check when running `eure check` without arguments.
    #[eure(default)]
    pub default_targets: Vec<String>,
}

/// Language server configuration.
#[cfg(feature = "ls")]
#[derive(Debug, Clone, Default, ParseDocument, PartialEq)]
#[eure(crate = eure_document, rename_all = "kebab-case", allow_unknown_fields)]
pub struct LsConfig {
    /// Whether to format on save.
    #[eure(default)]
    pub format_on_save: bool,
}

/// The main Eure configuration.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EureConfig {
    /// Check targets (name -> target definition).
    pub targets: HashMap<String, Target>,

    /// CLI-specific configuration.
    #[cfg(feature = "cli")]
    pub cli: Option<CliConfig>,

    /// Language server configuration.
    #[cfg(feature = "ls")]
    pub ls: Option<LsConfig>,
}

impl ParseDocument<'_> for EureConfig {
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let rec = ctx.parse_record()?;

        // Parse targets as a map
        let targets = if let Some(targets_ctx) = rec.field_optional("targets") {
            let targets_rec = targets_ctx.parse_record()?;
            let mut targets = HashMap::new();
            for (name, target_ctx) in targets_rec.unknown_fields() {
                let target = target_ctx.parse::<Target>()?;
                targets.insert(name.to_string(), target);
            }
            targets_rec.allow_unknown_fields()?;
            targets
        } else {
            HashMap::new()
        };

        #[cfg(feature = "cli")]
        let cli = rec
            .field_optional("cli")
            .map(|ctx| ctx.parse::<CliConfig>())
            .transpose()?;

        #[cfg(feature = "ls")]
        let ls = rec
            .field_optional("ls")
            .map(|ctx| ctx.parse::<LsConfig>())
            .transpose()?;

        rec.allow_unknown_fields()?;

        Ok(EureConfig {
            targets,
            #[cfg(feature = "cli")]
            cli,
            #[cfg(feature = "ls")]
            ls,
        })
    }
}

impl EureConfig {
    /// Find the configuration file by searching upward from the given directory.
    pub fn find_config_file(start_dir: &Path) -> Option<PathBuf> {
        let mut current = start_dir.to_path_buf();
        loop {
            let config_path = current.join(CONFIG_FILENAME);
            if config_path.exists() {
                return Some(config_path);
            }
            if !current.pop() {
                return None;
            }
        }
    }

    /// Get the default targets for CLI check command.
    #[cfg(feature = "cli")]
    pub fn default_targets(&self) -> &[String] {
        self.cli
            .as_ref()
            .map(|c| c.default_targets.as_slice())
            .unwrap_or(&[])
    }

    /// Get a target by name.
    pub fn get_target(&self, name: &str) -> Option<&Target> {
        self.targets.get(name)
    }

    /// Get all target names.
    pub fn target_names(&self) -> impl Iterator<Item = &str> {
        self.targets.keys().map(|s| s.as_str())
    }

    /// Find the schema for a file path by matching against target globs.
    ///
    /// Returns the first matching target's schema path, if any.
    pub fn schema_for_path(&self, file_path: &Path, config_dir: &Path) -> Option<PathBuf> {
        // Use explicit options for consistent cross-platform behavior
        let options = glob::MatchOptions {
            case_sensitive: true,
            require_literal_separator: true,
            require_literal_leading_dot: false,
        };

        for target in self.targets.values() {
            if let Some(ref schema) = target.schema {
                for glob_pattern in &target.globs {
                    // Make glob pattern absolute relative to config dir
                    let full_pattern = config_dir.join(glob_pattern);
                    if let Ok(pattern) = glob::Pattern::new(&full_pattern.to_string_lossy())
                        && pattern.matches_path_with(file_path, options)
                    {
                        // Return schema path relative to config dir
                        return Some(config_dir.join(schema));
                    }
                }
            }
        }
        None
    }
}
