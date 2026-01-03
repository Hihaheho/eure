//! Asset definitions for query-flow.
//!
//! Assets are an abstract mechanism for external data sources.
//! Each consumer provides asset values differently:
//! - eure-ls: provides from `didOpen`/`didChange` notifications
//! - eure-cli: provides by reading from disk (query-flow caches)
//! - test-suite: provides from test case strings

use std::path::PathBuf;
use std::sync::Arc;

use query_flow::asset_key;

/// Asset key for text file content.
#[asset_key(asset = TextFileContent)]
pub struct TextFile {
    pub path: Arc<PathBuf>,
}

impl TextFile {
    pub fn from_path(path: PathBuf) -> Self {
        Self {
            path: Arc::new(path),
        }
    }

    pub fn new(path: Arc<PathBuf>) -> Self {
        Self { path }
    }
}

impl std::fmt::Display for TextFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path.display())
    }
}

/// Content of a text file.
#[derive(Clone, PartialEq, Debug)]
pub enum TextFileContent {
    Content(String),
    NotFound,
}

impl TextFileContent {
    pub fn get(&self) -> Option<&str> {
        match self {
            TextFileContent::Content(content) => Some(content),
            TextFileContent::NotFound => None,
        }
    }

    pub fn map<T, F>(&self, f: F) -> Option<T>
    where
        F: FnOnce(&str) -> T,
    {
        match self {
            TextFileContent::Content(content) => Some(f(content)),
            TextFileContent::NotFound => None,
        }
    }
}

/// Asset key for workspace information.
#[asset_key(asset = Workspace)]
pub struct WorkspaceId(pub String);

/// Workspace information.
#[derive(Clone, PartialEq)]
pub struct Workspace {
    pub path: PathBuf,
    pub config_path: PathBuf,
}
