//! Asset locator for virtual paths and bundled content.

use eure::query::{TextFile, TextFileContent};
use query_flow::{AssetLocator, DurabilityLevel, LocateResult};

/// Bundled meta-schema content.
const META_SCHEMA: &str = include_str!("../../../assets/schemas/eure-schema.schema.eure");

/// Asset locator that handles virtual `$eure/` paths.
///
/// Returns `Ready` for bundled assets, `Pending` for files that need disk IO.
pub struct EureAssetLocator;

impl AssetLocator<TextFile> for EureAssetLocator {
    fn locate(&self, key: &TextFile) -> LocateResult<TextFileContent> {
        let path_str = key.path.to_string_lossy();
        if path_str == "$eure/meta-schema.eure" {
            return LocateResult::Ready {
                value: TextFileContent::Content(META_SCHEMA.to_string()),
                durability: DurabilityLevel::Static,
            };
        }
        LocateResult::Pending
    }
}
