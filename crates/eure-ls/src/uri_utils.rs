//! URI utility functions for converting between LSP URIs and TextFile.
//!
//! These functions are used by the WASM API but are defined separately
//! to enable unit testing on native builds.

// Allow dead code on native builds since these functions are only used by wasm_api
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

use std::path::PathBuf;

use eure::query::TextFile;

/// Convert a URI string to a TextFile.
///
/// Handles both file:// URIs and https:// URLs.
pub fn uri_to_text_file(uri: &str) -> TextFile {
    if uri.starts_with("https://") {
        // Remote URL
        TextFile::parse(uri)
    } else {
        // Local file
        let path = uri_to_path(uri);
        TextFile::from_path(PathBuf::from(path))
    }
}

/// Extract path from a file:// URI.
///
/// Handles both Unix-style (file:///path) and Windows-style (file:///C:/path) URIs.
pub fn uri_to_path(uri: &str) -> String {
    let path = if let Some(stripped) = uri.strip_prefix("file:///") {
        // Could be Unix (/path) or Windows (C:/path)
        // For Unix: file:///path -> /path (we need to add back the /)
        // For Windows: file:///C:/path -> C:/path
        if stripped.chars().nth(1) == Some(':') {
            // Windows path (e.g., C:/...)
            stripped.to_string()
        } else {
            // Unix path - restore the leading /
            format!("/{}", stripped)
        }
    } else if let Some(stripped) = uri.strip_prefix("file://") {
        // file://path (non-standard, but handle gracefully)
        stripped.to_string()
    } else {
        uri.to_string()
    };
    percent_decode(&path)
}

/// Percent-decode a URI path.
pub fn percent_decode(s: &str) -> String {
    percent_encoding::percent_decode_str(s)
        .decode_utf8_lossy()
        .into_owned()
}

/// Convert a TextFile to a URI string.
///
/// - Local files return file:// URIs
/// - Remote files return https:// URLs
///
/// Note: This function assumes local paths are absolute (standard for LSP).
/// Relative paths like `./schema.eure` will produce `file:///./schema.eure`
/// which may not be correctly interpreted by all clients.
pub fn text_file_to_uri(file: &TextFile) -> String {
    match file {
        TextFile::Local(path) => {
            let path_str = path.display().to_string();
            if path_str.starts_with('/') {
                format!("file://{}", path_str)
            } else {
                format!("file:///{}", path_str)
            }
        }
        TextFile::Remote(url) => url.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    mod uri_to_path_tests {
        use super::*;

        #[test]
        fn unix_absolute_path() {
            let path = uri_to_path("file:///home/user/file.eure");
            assert_eq!(path, "/home/user/file.eure");
        }

        #[test]
        fn unix_root_path() {
            let path = uri_to_path("file:///file.eure");
            assert_eq!(path, "/file.eure");
        }

        #[test]
        fn windows_absolute_path() {
            let path = uri_to_path("file:///C:/Users/user/file.eure");
            assert_eq!(path, "C:/Users/user/file.eure");
        }

        #[test]
        fn percent_encoded_spaces() {
            let path = uri_to_path("file:///home/user/my%20file.eure");
            assert_eq!(path, "/home/user/my file.eure");
        }

        #[test]
        fn percent_encoded_unicode() {
            let path = uri_to_path("file:///home/user/%E6%97%A5%E6%9C%AC%E8%AA%9E.eure");
            assert_eq!(path, "/home/user/日本語.eure");
        }

        #[test]
        fn non_uri_passthrough() {
            let path = uri_to_path("/direct/path");
            assert_eq!(path, "/direct/path");
        }
    }

    mod uri_to_text_file_tests {
        use super::*;

        #[test]
        fn file_uri_returns_local() {
            let file = uri_to_text_file("file:///home/user/file.eure");
            assert!(file.as_local_path().is_some());
            assert_eq!(
                file.as_local_path().unwrap(),
                Path::new("/home/user/file.eure")
            );
        }

        #[test]
        fn https_url_returns_remote() {
            let file = uri_to_text_file("https://example.com/schema.eure");
            assert!(file.as_url().is_some());
            assert_eq!(
                file.as_url().unwrap().as_str(),
                "https://example.com/schema.eure"
            );
        }

        #[test]
        fn windows_file_uri() {
            let file = uri_to_text_file("file:///C:/Users/test.eure");
            assert!(file.as_local_path().is_some());
            assert_eq!(
                file.as_local_path().unwrap(),
                Path::new("C:/Users/test.eure")
            );
        }
    }

    mod text_file_to_uri_tests {
        use super::*;

        #[test]
        fn local_unix_path() {
            let file = TextFile::from_path(PathBuf::from("/home/user/file.eure"));
            assert_eq!(text_file_to_uri(&file), "file:///home/user/file.eure");
        }

        #[test]
        fn local_windows_path() {
            let file = TextFile::from_path(PathBuf::from("C:/Users/file.eure"));
            assert_eq!(text_file_to_uri(&file), "file:///C:/Users/file.eure");
        }

        #[test]
        fn remote_url() {
            let file = TextFile::parse("https://example.com/schema.eure");
            assert_eq!(text_file_to_uri(&file), "https://example.com/schema.eure");
        }
    }
}
