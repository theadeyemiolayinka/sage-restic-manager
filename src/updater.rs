use reqwest::Client;
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::env;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};

use crate::config::app::UpdateChannel;
use crate::error::{AppError, Result};

const GITHUB_API_RELEASES: &str = "https://api.github.com/repos/your-org/sage-restic-manager/releases";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub prerelease: bool,
    pub draft: bool,
    pub assets: Vec<GitHubAsset>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

pub struct Updater {
    client: Client,
    channel: UpdateChannel,
}

impl Updater {
    pub fn new(channel: UpdateChannel) -> Self {
        let client = Client::builder()
            .user_agent(format!("sage-restic-manager/{}", CURRENT_VERSION))
            .build()
            .unwrap();
        Self { client, channel }
    }

    pub fn current_version() -> Version {
        Version::parse(CURRENT_VERSION).unwrap()
    }

    pub async fn check_for_update(&self) -> Result<Option<GitHubRelease>> {
        let releases: Vec<GitHubRelease> = self.client
            .get(GITHUB_API_RELEASES)
            .send()
            .await?
            .json()
            .await?;

        let current = Self::current_version();

        for release in releases {
            if release.draft {
                continue;
            }
            if self.channel == UpdateChannel::Stable && release.prerelease {
                continue;
            }
            let tag = release.tag_name.trim_start_matches('v');
            if let Ok(v) = Version::parse(tag) {
                if v > current {
                    return Ok(Some(release));
                }
            }
        }

        Ok(None)
    }

    pub async fn perform_update(&self, release: &GitHubRelease) -> Result<()> {
        let binary_name = self.platform_binary_name();
        let checksum_name = format!("{}.sha256", binary_name);

        let binary_asset = release.assets.iter()
            .find(|a| a.name == binary_name)
            .ok_or_else(|| AppError::Update(format!("No binary asset found for platform: {}", binary_name)))?;

        let checksum_asset = release.assets.iter()
            .find(|a| a.name == checksum_name);

        info!("Downloading {} ({})", binary_asset.name, bytesize::ByteSize(binary_asset.size).to_string_as(true));

        let bytes = self.client
            .get(&binary_asset.browser_download_url)
            .send()
            .await?
            .bytes()
            .await?;

        if let Some(cs_asset) = checksum_asset {
            let cs_text = self.client
                .get(&cs_asset.browser_download_url)
                .send()
                .await?
                .text()
                .await?;
            let expected = cs_text.split_whitespace().next()
                .unwrap_or("")
                .to_lowercase();
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let actual = hex::encode(hasher.finalize());
            if actual != expected {
                return Err(AppError::ChecksumMismatch { expected, actual });
            }
            info!("Checksum verified");
        } else {
            warn!("No checksum asset found, skipping verification");
        }

        let current_exe = env::current_exe()
            .map_err(|e| AppError::Update(format!("Cannot determine current binary: {}", e)))?;

        let backup_path = current_exe.with_extension("bak");
        tokio::fs::copy(&current_exe, &backup_path).await?;

        let temp_path = current_exe.with_extension("tmp");
        let mut file = tokio::fs::File::create(&temp_path).await?;
        file.write_all(&bytes).await?;
        file.flush().await?;
        drop(file);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o755);
            tokio::fs::set_permissions(&temp_path, perms).await?;
        }

        tokio::fs::rename(&temp_path, &current_exe).await
            .map_err(|e| {
                let _ = tokio::fs::rename(&backup_path, &current_exe);
                AppError::Update(format!("Failed to replace binary: {}", e))
            })?;

        let _ = tokio::fs::remove_file(&backup_path).await;
        info!("Update complete. New version: {}", release.tag_name);
        Ok(())
    }

    fn platform_binary_name(&self) -> String {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        match (os, arch) {
            ("linux", "x86_64") => "sage-restic-manager-x86_64-unknown-linux-musl".into(),
            ("linux", "aarch64") => "sage-restic-manager-aarch64-unknown-linux-musl".into(),
            ("macos", "x86_64") => "sage-restic-manager-x86_64-apple-darwin".into(),
            ("macos", "aarch64") => "sage-restic-manager-aarch64-apple-darwin".into(),
            _ => format!("sage-restic-manager-{}-{}", arch, os),
        }
    }
}
