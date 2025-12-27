use std::{path::PathBuf, sync::Arc};

use query_flow::asset_key;

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

#[derive(Clone, PartialEq, Debug)]
pub enum TextFileContent {
    Content(String),
    NotFound,
}

impl TextFileContent {
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

#[asset_key(asset = Workspace)]
pub struct WorkspaceId(pub String);

#[derive(Clone, PartialEq)]
pub struct Workspace {
    pub path: PathBuf,
    pub config_path: PathBuf,
}
