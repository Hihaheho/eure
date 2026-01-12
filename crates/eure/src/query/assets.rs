//! Asset definitions for query-flow.
//!
//! Assets are an abstract mechanism for external data sources.
//! Each consumer provides asset values differently:
//! - eure-ls: provides from `didOpen`/`didChange` notifications
//! - eure-cli: provides by reading from disk (query-flow caches)
//! - test-suite: provides from test case strings

use std::path::{Path, PathBuf};
use std::sync::Arc;

use query_flow::asset_key;
use url::Url;

/// Asset key for text file content.
#[asset_key(asset = TextFileContent)]
pub enum TextFile {
    /// Local file path.
    Local(Arc<PathBuf>),
    /// Remote URL (HTTPS).
    Remote(Url),
}

impl TextFile {
    /// Create a TextFile from a local path.
    pub fn from_path(path: PathBuf) -> Self {
        Self::Local(Arc::new(path))
    }

    /// Create a TextFile from a URL.
    pub fn from_url(url: Url) -> Self {
        Self::Remote(url)
    }

    /// Parse a string as either a URL (if starts with https://) or a local path.
    pub fn parse(s: &str) -> Self {
        if s.starts_with("https://") {
            Self::from_url(Url::parse(s).expect("valid URL"))
        } else {
            Self::from_path(PathBuf::from(s))
        }
    }

    /// Resolve a schema/file reference relative to a base directory.
    ///
    /// - If `target` starts with "https://", returns a `TextFile::Remote`
    /// - Otherwise, joins `target` with `base_dir` and returns a `TextFile::Local`
    pub fn resolve(target: &str, base_dir: &Path) -> Self {
        if target.starts_with("https://") {
            Self::parse(target)
        } else {
            Self::from_path(base_dir.join(target))
        }
    }

    /// Create a TextFile from an Arc<PathBuf> (for backward compatibility).
    pub fn new(path: Arc<PathBuf>) -> Self {
        Self::Local(path)
    }

    /// Get the local path if this is a local file.
    pub fn as_local_path(&self) -> Option<&Path> {
        match self {
            Self::Local(p) => Some(p),
            Self::Remote(_) => None,
        }
    }

    /// Get the URL if this is a remote file.
    pub fn as_url(&self) -> Option<&Url> {
        match self {
            Self::Local(_) => None,
            Self::Remote(url) => Some(url),
        }
    }

    /// Check if this is a local file (not a remote URL).
    pub fn is_local(&self) -> bool {
        matches!(self, Self::Local(_))
    }

    /// Check if the file path/URL ends with the given suffix.
    pub fn ends_with(&self, suffix: &str) -> bool {
        match self {
            Self::Local(path) => path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().ends_with(suffix)),
            Self::Remote(url) => url.path().ends_with(suffix),
        }
    }
}

impl std::fmt::Display for TextFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local(path) => write!(f, "{}", path.display()),
            Self::Remote(url) => write!(f, "{}", url),
        }
    }
}

/// Content of a text file.
#[derive(Clone, PartialEq, Debug)]
pub struct TextFileContent(pub String);

impl TextFileContent {
    pub fn get(&self) -> &str {
        &self.0
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

/// Asset key for glob pattern expansion.
///
/// Resolves a glob pattern relative to a base directory into a list of matching files.
/// Each platform implements this differently:
/// - CLI: uses `glob::glob()` on the filesystem
/// - LSP (wasm32): returns empty or queries client for file list
/// - test-suite: pre-resolves with test files
#[asset_key(asset = GlobResult)]
pub struct Glob {
    pub base_dir: PathBuf,
    pub pattern: String,
}

impl Glob {
    pub fn new(base_dir: impl Into<PathBuf>, pattern: impl Into<String>) -> Self {
        Self {
            base_dir: base_dir.into(),
            pattern: pattern.into(),
        }
    }

    /// Returns the full pattern path (base_dir joined with pattern).
    pub fn full_pattern(&self) -> PathBuf {
        self.base_dir.join(&self.pattern)
    }
}

/// Result of glob pattern expansion.
#[derive(Clone, PartialEq, Debug)]
pub struct GlobResult(pub Vec<TextFile>);

/// Asset key for open documents list.
///
/// Used by LSP to track currently open documents.
/// Collection queries depend on this asset to invalidate when documents open/close.
#[asset_key(asset = OpenDocumentsList)]
pub struct OpenDocuments;

/// List of currently open documents.
#[derive(Clone, PartialEq, Debug)]
pub struct OpenDocumentsList(pub Vec<TextFile>);

#[cfg(test)]
mod tests {
    use super::*;

    mod text_file_parse {
        use super::*;

        #[test]
        fn parses_https_url() {
            let file = TextFile::parse("https://example.com/schema.eure");
            assert!(file.as_url().is_some());
            assert!(file.as_local_path().is_none());
            assert_eq!(
                file.as_url().unwrap().as_str(),
                "https://example.com/schema.eure"
            );
        }

        #[test]
        fn parses_local_path() {
            let file = TextFile::parse("/path/to/file.eure");
            assert!(file.as_local_path().is_some());
            assert!(file.as_url().is_none());
            assert_eq!(
                file.as_local_path().unwrap(),
                Path::new("/path/to/file.eure")
            );
        }

        #[test]
        fn parses_relative_path() {
            let file = TextFile::parse("relative/path.eure");
            assert!(file.as_local_path().is_some());
            assert_eq!(
                file.as_local_path().unwrap(),
                Path::new("relative/path.eure")
            );
        }

        #[test]
        fn http_without_s_is_local_path() {
            // http:// (without s) is treated as a local path, not a URL
            let file = TextFile::parse("http://example.com");
            assert!(file.as_local_path().is_some());
        }
    }

    mod text_file_ends_with {
        use super::*;

        #[test]
        fn local_file_ends_with_extension() {
            let file = TextFile::from_path(PathBuf::from("/path/to/file.schema.eure"));
            assert!(file.ends_with(".schema.eure"));
            assert!(file.ends_with(".eure"));
            assert!(!file.ends_with(".json"));
        }

        #[test]
        fn local_file_ends_with_filename() {
            let file = TextFile::from_path(PathBuf::from("/path/to/config.eure"));
            assert!(file.ends_with("config.eure"));
            assert!(!file.ends_with("other.eure"));
        }

        #[test]
        fn remote_url_ends_with_extension() {
            let file = TextFile::parse("https://example.com/schemas/user.schema.eure");
            assert!(file.ends_with(".schema.eure"));
            assert!(file.ends_with(".eure"));
            assert!(!file.ends_with(".json"));
        }

        #[test]
        fn remote_url_ignores_query_params() {
            // For Remote URLs, ends_with uses url.path() which excludes query params.
            // So "https://example.com/file.eure?version=1" has path "/file.eure"
            let file = TextFile::parse("https://example.com/file.eure?version=1");
            assert!(file.ends_with(".eure"));
            assert!(!file.ends_with("?version=1"));
        }

        #[test]
        fn remote_url_ignores_fragment() {
            let file = TextFile::parse("https://example.com/file.eure#section");
            assert!(file.ends_with(".eure"));
            assert!(!file.ends_with("#section"));
        }
    }
}
