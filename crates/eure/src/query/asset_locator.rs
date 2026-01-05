//! Asset locator with URL host validation.
//!
//! Provides URL host validation against an allowlist:
//! - `eure.dev` is always allowed (default trusted host)
//! - Additional hosts can be configured via `@ security.allowed-hosts` in Eure.eure

use std::sync::Arc;

use query_flow::{Db, LocateResult, QueryError, asset_locator};
use url::Url;

use super::assets::{TextFile, TextFileContent, Workspace, WorkspaceId};
use super::config::GetConfig;
use super::error::EureQueryError;

/// The default allowed host (always trusted).
const DEFAULT_ALLOWED_HOST: &str = "eure.dev";

/// Asset locator that validates URL hosts before allowing fetches.
///
/// This locator:
/// 1. Validates remote URLs against the host allowlist
/// 2. Returns `Pending` for allowed URLs (to be fetched by platform)
/// 3. Returns `UserError` for disallowed hosts
///
/// Local files are always allowed (no validation needed).
///
/// # Allowlist Resolution
/// 1. `eure.dev` and `*.eure.dev` are always allowed
/// 2. Additional hosts come from `@ security.allowed-hosts` in workspace config
/// 3. If no config is available, only `eure.dev` is allowed
#[asset_locator]
pub fn text_file_locator(
    db: &impl Db,
    key: &TextFile,
) -> Result<LocateResult<TextFileContent>, QueryError> {
    match key {
        TextFile::Local(_) => {
            // Local files are always allowed
            Ok(LocateResult::Pending)
        }
        TextFile::Remote(url) => {
            // Validate remote URL host
            validate_url_host(db, url)?;
            // Host is allowed - let platform fetch
            Ok(LocateResult::Pending)
        }
    }
}

/// Validate that a URL's host is in the allowlist.
///
/// Returns `Ok(())` if the host is allowed, or `Err(QueryError::UserError)` if not.
fn validate_url_host(db: &impl Db, url: &Url) -> Result<(), QueryError> {
    let host = url.host_str().unwrap_or("");

    // eure.dev is always allowed (including subdomains)
    if host == DEFAULT_ALLOWED_HOST || host.ends_with(".eure.dev") {
        return Ok(());
    }

    // Try to get config from workspace
    let allowed_hosts = get_allowed_hosts_from_workspace(db)?;

    // Check if host is in allowlist
    if allowed_hosts
        .iter()
        .any(|allowed| host_matches(host, allowed))
    {
        return Ok(());
    }

    // Host not allowed
    Err(EureQueryError::HostNotAllowed {
        url: url.clone(),
        host: host.to_string(),
    }
    .into())
}

/// Check if a host matches an allowed pattern.
///
/// Supports:
/// - Exact match: "example.com" matches "example.com"
/// - Wildcard subdomain: "*.example.com" matches "sub.example.com" and "example.com"
fn host_matches(host: &str, pattern: &str) -> bool {
    if let Some(suffix) = pattern.strip_prefix("*.") {
        // Wildcard pattern: *.example.com matches sub.example.com and example.com
        host == suffix || host.ends_with(&format!(".{}", suffix))
    } else {
        // Exact match
        host == pattern
    }
}

/// Get allowed hosts from workspace config.
///
/// Returns an empty vec if no workspace or config is available.
fn get_allowed_hosts_from_workspace(db: &impl Db) -> Result<Vec<String>, QueryError> {
    // Get workspace - if none exists, return empty (only eure.dev allowed)
    let workspace_ids = db.list_asset_keys::<WorkspaceId>();
    let Some(workspace_id) = workspace_ids.into_iter().next() else {
        return Ok(Vec::new());
    };

    // Get workspace asset
    let workspace: Arc<Workspace> = db.asset(workspace_id)?.suspend()?;

    // Create a TextFile for the config
    let config_file = TextFile::from_path(workspace.config_path.clone());

    // Query the config - if it fails or doesn't exist, return empty
    let config_opt = match db.query(GetConfig::new(config_file)) {
        Ok(config_opt) => config_opt,
        Err(QueryError::Suspend { asset }) => return Err(QueryError::Suspend { asset }),
        Err(_) => return Ok(Vec::new()), // Config parse error - use defaults
    };

    // Extract allowed hosts from security config
    match config_opt.as_ref() {
        Some(config) => Ok(config.allowed_hosts().to_vec()),
        None => Ok(Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod host_matches_tests {
        use super::*;

        #[test]
        fn exact_match() {
            assert!(host_matches("example.com", "example.com"));
            assert!(!host_matches("other.com", "example.com"));
            assert!(!host_matches("sub.example.com", "example.com"));
        }

        #[test]
        fn wildcard_match_subdomain() {
            assert!(host_matches("sub.example.com", "*.example.com"));
            assert!(host_matches("a.b.example.com", "*.example.com"));
        }

        #[test]
        fn wildcard_match_base() {
            // *.example.com also matches example.com
            assert!(host_matches("example.com", "*.example.com"));
        }

        #[test]
        fn wildcard_no_match() {
            assert!(!host_matches("other.com", "*.example.com"));
            assert!(!host_matches("exampleXcom", "*.example.com"));
        }

        #[test]
        fn empty_pattern() {
            assert!(!host_matches("example.com", ""));
            assert!(host_matches("", ""));
        }
    }
}
