//! Configuration types for Eure tools.
//!
//! This crate provides configuration types for the Eure CLI and Language Server.
//! The configuration is stored in `Eure.eure` files at project roots.
//!
//! # Features
//!
//! - `lint` - Include lint configuration types
//! - `ls` - Include language server configuration types
//! - `cli` - Include CLI configuration (enables `lint` and `ls`)
//! - `all` - Include all configuration types

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use eure::document::parse_to_document;
use eure_document::parse::{ParseContext, ParseDocument, ParseError};

// Re-export for convenience
pub use eure_document::parse::ParseError as ConfigParseError;

/// The standard configuration filename.
pub const CONFIG_FILENAME: &str = "Eure.eure";

/// Error type for configuration loading.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Config error at {node_id:?}: {kind}")]
    Config {
        node_id: eure_document::document::NodeId,
        kind: String,
    },
}

impl From<ParseError> for ConfigError {
    fn from(err: ParseError) -> Self {
        ConfigError::Config {
            node_id: err.node_id,
            kind: err.kind.to_string(),
        }
    }
}

/// A check target definition.
#[derive(Debug, Clone)]
pub struct Target {
    /// Glob patterns for files to include in this target.
    pub globs: Vec<String>,
    /// Optional schema file path (relative to config file).
    pub schema: Option<String>,
}

impl ParseDocument<'_> for Target {
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let mut rec = ctx.parse_record()?;

        let globs = rec.parse_field::<Vec<String>>("globs")?;
        let schema = rec.parse_field_optional::<String>("schema")?;

        rec.allow_unknown_fields()?;

        Ok(Target { globs, schema })
    }
}

/// CLI-specific configuration.
#[cfg(feature = "cli")]
#[derive(Debug, Clone, Default)]
pub struct CliConfig {
    /// Default targets to check when running `eure check` without arguments.
    pub default_targets: Vec<String>,
}

#[cfg(feature = "cli")]
impl ParseDocument<'_> for CliConfig {
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let mut rec = ctx.parse_record()?;

        let default_targets = rec
            .parse_field_optional::<Vec<String>>("default-targets")?
            .unwrap_or_default();

        rec.allow_unknown_fields()?;

        Ok(CliConfig { default_targets })
    }
}

/// Language server configuration.
#[cfg(feature = "ls")]
#[derive(Debug, Clone, Default)]
pub struct LsConfig {
    /// Whether to format on save.
    pub format_on_save: bool,
}

#[cfg(feature = "ls")]
impl ParseDocument<'_> for LsConfig {
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let mut rec = ctx.parse_record()?;

        let format_on_save = rec
            .parse_field_optional::<bool>("format-on-save")?
            .unwrap_or(false);

        rec.allow_unknown_fields()?;

        Ok(LsConfig { format_on_save })
    }
}

/// The main Eure configuration.
#[derive(Debug, Clone, Default)]
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
        let mut rec = ctx.parse_record()?;

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
    /// Load configuration from a file.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse_str(&content)
    }

    /// Parse configuration from a string.
    pub fn parse_str(content: &str) -> Result<Self, ConfigError> {
        let doc = parse_to_document(content).map_err(|e| ConfigError::Parse(e.to_string()))?;
        let root_id = doc.get_root_id();
        let config: EureConfig = doc.parse(root_id)?;
        Ok(config)
    }

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

    /// Load configuration by searching upward from the given directory.
    pub fn load_from_dir(start_dir: &Path) -> Result<Option<(PathBuf, Self)>, ConfigError> {
        if let Some(config_path) = Self::find_config_file(start_dir) {
            let config = Self::load(&config_path)?;
            Ok(Some((config_path, config)))
        } else {
            Ok(None)
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let content = r#"
@ targets.adr
globs = ["docs/adrs/**/*.eure"]
schema = "assets/schemas/eure-adr.schema.eure"

@ targets.schemas
globs = ["assets/schemas/**/*.eure"]

@ cli
default-targets = ["adr", "schemas"]
"#;

        let config = EureConfig::parse_str(content).unwrap();
        assert_eq!(config.targets.len(), 2);

        let adr = config.get_target("adr").unwrap();
        assert_eq!(adr.globs, vec!["docs/adrs/**/*.eure"]);
        assert_eq!(
            adr.schema.as_deref(),
            Some("assets/schemas/eure-adr.schema.eure")
        );

        let schemas = config.get_target("schemas").unwrap();
        assert_eq!(schemas.globs, vec!["assets/schemas/**/*.eure"]);
        assert!(schemas.schema.is_none());

        #[cfg(feature = "cli")]
        {
            let cli = config.cli.as_ref().unwrap();
            assert_eq!(cli.default_targets, vec!["adr", "schemas"]);
        }
    }

    #[test]
    fn test_empty_config() {
        let content = "";
        let config = EureConfig::parse_str(content).unwrap();
        assert!(config.targets.is_empty());
    }
}
