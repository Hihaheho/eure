use std::{fs, io, path::Path};
use zed_extension_api::{self as zed, Architecture, Os};

// Publish this release's six native archives before distributing the extension.
const EURE_VERSION: &str = "0.2.0";
const REPOSITORY: &str = "Hihaheho/eure";

#[derive(Debug, thiserror::Error)]
enum InstallError {
    #[error("unsupported platform: {0:?}/{1:?}")]
    UnsupportedPlatform(Os, Architecture),
    #[error("Zed API: {0}")]
    Api(String),
    #[error("missing release asset: {0}")]
    MissingAsset(String),
    #[error("expected a nonempty executable file: {0}")]
    InvalidBinary(String),
    #[error(transparent)]
    Io(#[from] io::Error),
}

struct Platform {
    target: &'static str,
    windows: bool,
}

impl Platform {
    fn new(os: Os, arch: Architecture) -> Result<Self, InstallError> {
        let target = match (os, arch) {
            (Os::Mac, Architecture::Aarch64) => "aarch64-apple-darwin",
            (Os::Mac, Architecture::X8664) => "x86_64-apple-darwin",
            (Os::Linux, Architecture::Aarch64) => "aarch64-unknown-linux-gnu",
            (Os::Linux, Architecture::X8664) => "x86_64-unknown-linux-gnu",
            (Os::Windows, Architecture::Aarch64) => "aarch64-pc-windows-msvc",
            (Os::Windows, Architecture::X8664) => "x86_64-pc-windows-msvc",
            _ => return Err(InstallError::UnsupportedPlatform(os, arch)),
        };
        Ok(Self {
            target,
            windows: os == Os::Windows,
        })
    }

    fn archive_stem(&self) -> String {
        format!("eure-v{EURE_VERSION}-{}", self.target)
    }

    fn asset_name(&self) -> String {
        format!(
            "{}.{}",
            self.archive_stem(),
            if self.windows { "zip" } else { "tar.gz" }
        )
    }

    fn binary_path(&self, directory: &str) -> String {
        // Release archives contain a top-level directory named after the archive.
        format!(
            "{directory}/{}/eurels{}",
            self.archive_stem(),
            if self.windows { ".exe" } else { "" }
        )
    }
}

fn binary_exists(path: &str) -> Result<bool, InstallError> {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() && metadata.len() > 0 => Ok(true),
        Ok(_) => Err(InstallError::InvalidBinary(path.into())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn install(id: &zed::LanguageServerId) -> Result<String, InstallError> {
    let (os, arch) = zed::current_platform();
    let platform = Platform::new(os, arch)?;
    let directory = format!("eure-ls-{EURE_VERSION}-{}", platform.target);
    let binary = platform.binary_path(&directory);
    if binary_exists(&binary)? {
        return Ok(binary);
    }

    zed::set_language_server_installation_status(
        id,
        &zed::LanguageServerInstallationStatus::CheckingForUpdate,
    );
    let release = zed::github_release_by_tag_name(REPOSITORY, &format!("v{EURE_VERSION}"))
        .map_err(InstallError::Api)?;
    let asset_name = platform.asset_name();
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == asset_name)
        .ok_or(InstallError::MissingAsset(asset_name))?;

    // Only promote a fully extracted, executable installation to the cache.
    let staging = format!("{directory}.download");
    if Path::new(&staging).try_exists()? {
        fs::remove_dir_all(&staging)?;
    }
    zed::set_language_server_installation_status(
        id,
        &zed::LanguageServerInstallationStatus::Downloading,
    );
    zed::download_file(
        &asset.download_url,
        &staging,
        if platform.windows {
            zed::DownloadedFileType::Zip
        } else {
            zed::DownloadedFileType::GzipTar
        },
    )
    .map_err(InstallError::Api)?;
    let staged_binary = platform.binary_path(&staging);
    if !binary_exists(&staged_binary)? {
        return Err(InstallError::InvalidBinary(staged_binary));
    }
    if !platform.windows {
        zed::make_file_executable(&staged_binary).map_err(InstallError::Api)?;
    }
    if Path::new(&directory).try_exists()? {
        fs::remove_dir_all(&directory)?;
    }
    fs::rename(staging, directory)?;
    Ok(binary)
}

struct EureExtension;

impl zed::Extension for EureExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        let command = match worktree.which("eurels") {
            Some(path) => Ok(path),
            None => install(id),
        };
        match command {
            Ok(command) => {
                zed::set_language_server_installation_status(
                    id,
                    &zed::LanguageServerInstallationStatus::None,
                );
                Ok(zed::Command {
                    command,
                    args: vec![],
                    env: vec![],
                })
            }
            Err(error) => {
                let message = error.to_string();
                zed::set_language_server_installation_status(
                    id,
                    &zed::LanguageServerInstallationStatus::Failed(message.clone()),
                );
                Err(message)
            }
        }
    }
}

zed::register_extension!(EureExtension);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platforms_match_release_workflow() {
        let workflow = include_str!("../../../.github/workflows/release-binaries.yml");
        let mut targets = Vec::new();
        for os in [Os::Mac, Os::Linux, Os::Windows] {
            for arch in [Architecture::Aarch64, Architecture::X8664] {
                let platform = Platform::new(os, arch).unwrap();
                assert!(workflow.contains(&format!("target: {}", platform.target)));
                assert!(!targets.contains(&platform.target));
                targets.push(platform.target);
                let extension = if os == Os::Windows { "zip" } else { "tar.gz" };
                assert_eq!(
                    platform.asset_name(),
                    format!("eure-v{EURE_VERSION}-{}.{extension}", platform.target)
                );
                let executable = if os == Os::Windows {
                    "eurels.exe"
                } else {
                    "eurels"
                };
                assert_eq!(
                    platform.binary_path("cache"),
                    format!(
                        "cache/eure-v{EURE_VERSION}-{}/{executable}",
                        platform.target
                    )
                );
            }
            assert!(matches!(
                Platform::new(os, Architecture::X86),
                Err(InstallError::UnsupportedPlatform(..))
            ));
        }
        assert_eq!(workflow.matches("target: ").count(), targets.len());
    }

    #[test]
    fn cache_rejects_empty_files_and_directories() {
        let directory = std::env::temp_dir().join(format!("eure-zed-test-{}", std::process::id()));
        fs::create_dir(&directory).unwrap();
        let path = directory.join("eurels");
        let binary = path.to_str().unwrap();
        assert!(!binary_exists(binary).unwrap());
        fs::write(&path, []).unwrap();
        assert!(matches!(
            binary_exists(binary),
            Err(InstallError::InvalidBinary(_))
        ));
        fs::write(&path, b"server").unwrap();
        assert!(binary_exists(binary).unwrap());
        assert!(matches!(
            binary_exists(directory.to_str().unwrap()),
            Err(InstallError::InvalidBinary(_))
        ));
        fs::remove_dir_all(directory).unwrap();
    }
}
