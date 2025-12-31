//! Asset locator for virtual paths and bundled content.

use eure_editor_support::assets::{TextFile, TextFileContent};
use query_flow::{AssetLocator, LocateResult};

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
            return LocateResult::Ready(TextFileContent::Content(META_SCHEMA.to_string()));
        }
        LocateResult::Pending
    }
}
