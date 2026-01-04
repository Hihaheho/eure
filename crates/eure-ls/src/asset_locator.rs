//! Asset locator for virtual paths and bundled content.

use eure::query::{TextFile, TextFileContent};
use query_flow::{AssetLocator, Db, DurabilityLevel, LocateResult, QueryError};

/// Bundled meta-schema content.
const META_SCHEMA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/schemas/eure-schema.schema.eure"
));

/// Asset locator that handles virtual `$eure/` paths.
///
/// Returns `Ready` for bundled assets, `Pending` for files that need disk IO.
pub struct EureAssetLocator;

impl AssetLocator<TextFile> for EureAssetLocator {
    fn locate(
        &self,
        _db: &impl Db,
        key: &TextFile,
    ) -> Result<LocateResult<TextFileContent>, QueryError> {
        // Only local files can be virtual $eure/ paths
        if let Some(path) = key.as_local_path() {
            let path_str = path.to_string_lossy();
            if path_str == "$eure/meta-schema.eure" {
                return Ok(LocateResult::Ready {
                    value: TextFileContent::Content(META_SCHEMA.to_string()),
                    durability: DurabilityLevel::Static,
                });
            }
        }
        Ok(LocateResult::Pending)
    }
}
