//! Environment and configuration for Eure tools.
//!
//! This crate provides configuration data types and caching for the Eure CLI
//! and Language Server. The configuration is stored in `Eure.eure` files at project roots.
//!
//! # Features
//!
//! - `lint` - Include lint configuration types
//! - `ls` - Include language server configuration types
//! - `cli` - Include CLI configuration (enables `lint` and `ls`)
//! - `native` - Include native I/O for remote schema caching (requires network/filesystem dependencies)
//! - `all` - Include all configuration types

pub mod cache;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use eure_document::parse::{FromEure, ParseContext, ParseError, ParseErrorKind};
use eure_macros::FromEure;
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
#[derive(Debug, Clone, FromEure, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, Default, FromEure, PartialEq)]
#[eure(crate = eure_document, rename_all = "kebab-case", allow_unknown_fields)]
pub struct CliConfig {
    /// Default targets to check when running `eure check` without arguments.
    #[eure(default)]
    pub default_targets: Vec<String>,
}

/// Language server configuration.
#[cfg(feature = "ls")]
#[derive(Debug, Clone, Default, FromEure, PartialEq)]
#[eure(crate = eure_document, rename_all = "kebab-case", allow_unknown_fields)]
pub struct LsConfig {
    /// Whether to format on save.
    #[eure(default)]
    pub format_on_save: bool,
}

/// Security configuration for remote URL access.
#[derive(Debug, Clone, Default, FromEure, PartialEq)]
#[eure(crate = eure_document, rename_all = "kebab-case", allow_unknown_fields)]
pub struct SecurityConfig {
    /// Additional allowed hosts for remote URL fetching (beyond eure.dev).
    ///
    /// Supports exact matches (e.g., "example.com") and wildcard subdomains
    /// (e.g., "*.example.com" matches "sub.example.com" and "example.com").
    #[eure(default)]
    pub allowed_hosts: Vec<String>,
}

/// The main Eure configuration.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EureConfig {
    /// Check targets (name -> target definition).
    pub targets: HashMap<String, Target>,

    /// Security configuration (remote URL access control).
    pub security: Option<SecurityConfig>,

    /// CLI-specific configuration.
    #[cfg(feature = "cli")]
    pub cli: Option<CliConfig>,

    /// Language server configuration.
    #[cfg(feature = "ls")]
    pub ls: Option<LsConfig>,
}

impl FromEure<'_> for EureConfig {
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let rec = ctx.parse_record()?;

        // Parse targets as a map
        let targets = if let Some(targets_ctx) = rec.field_optional("targets") {
            let targets_rec = targets_ctx.parse_record()?;
            let mut targets = HashMap::new();
            for result in targets_rec.unknown_fields() {
                let (name, target_ctx) = result.map_err(|(key, ctx)| ParseError {
                    node_id: ctx.node_id(),
                    kind: ParseErrorKind::InvalidKeyType(key.clone()),
                })?;
                let target = target_ctx.parse::<Target>()?;
                targets.insert(name.to_string(), target);
            }
            targets_rec.allow_unknown_fields()?;
            targets
        } else {
            HashMap::new()
        };

        let security = rec
            .field_optional("security")
            .map(|ctx| ctx.parse::<SecurityConfig>())
            .transpose()?;

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
            security,
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
    pub fn schema_for_path(&self, file_path: &Path, config_dir: &Path) -> Option<String> {
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
                        return Some(schema.clone());
                    }
                }
            }
        }
        None
    }

    /// Get the allowed hosts for remote URL fetching from security config.
    ///
    /// Returns an empty slice if no security config is present.
    /// Note: This does NOT include the default `eure.dev` - callers should
    /// check that separately.
    pub fn allowed_hosts(&self) -> &[String] {
        self.security
            .as_ref()
            .map(|s| s.allowed_hosts.as_slice())
            .unwrap_or(&[])
    }
}
