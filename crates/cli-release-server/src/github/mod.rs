//! GitHub-backed release provider.

mod assets;
mod client;
mod releases;

use std::time::Duration;

use async_trait::async_trait;
use cli_release_interface::{ReleaseAssetInfo, ReleaseIndex};

use assets::{AssetTemplate, sha256_from_asset_digest};
use client::download_asset;
use releases::{GitHubAsset, GitHubRelease};

use crate::config::GitHubConfig;
use crate::{Error, ReleaseDownload, ReleaseProvider, Result};

/// Default total request timeout for GitHub API and asset calls.
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Default TCP connect timeout for GitHub API and asset calls.
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Release provider backed by private GitHub Releases.
#[derive(Clone, Debug)]
pub struct GitHubReleaseProvider {
    repository: String,
    api_url: String,
    token: String,
    tag_prefix: String,
    asset_template: AssetTemplate,
    client: reqwest::Client,
}

impl GitHubReleaseProvider {
    /// Creates a provider from explicit GitHub release configuration.
    pub fn new(config: GitHubConfig) -> Result<Self> {
        Self::new_with_timeouts(config, DEFAULT_REQUEST_TIMEOUT, DEFAULT_CONNECT_TIMEOUT)
    }

    /// Creates a provider with explicit GitHub request and connect timeouts.
    pub fn new_with_timeouts(
        config: GitHubConfig,
        request_timeout: Duration,
        connect_timeout: Duration,
    ) -> Result<Self> {
        let asset_template = AssetTemplate::new(config.asset_template.clone())?;
        let client = reqwest::Client::builder()
            .timeout(request_timeout)
            .connect_timeout(connect_timeout)
            .build()
            .map_err(|source| Error::GitHubClientBuild { source })?;
        Ok(Self {
            repository: config.repository,
            api_url: config.api_url,
            token: config.token,
            tag_prefix: config.tag_prefix,
            asset_template,
            client,
        })
    }

    async fn latest_release(&self) -> Result<GitHubRelease> {
        releases::latest_release(self).await
    }

    async fn release_for_tag(&self, tag: &str) -> Result<GitHubRelease> {
        releases::release_for_tag(self, tag).await
    }

    async fn download_asset(&self, asset: &GitHubAsset) -> Result<Vec<u8>> {
        download_asset(self, asset).await
    }

    async fn latest_versioned_asset(
        &self,
        binary: &str,
        target: &str,
    ) -> Result<(GitHubRelease, String, GitHubAsset, String)> {
        let release = self.latest_release().await?;
        let version = self.version_from_tag(&release.tag_name)?;
        let asset_name = self.asset_template.archive_name(binary, &version, target);
        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .cloned()
            .ok_or_else(|| Error::MissingAsset {
                asset_name: asset_name.clone(),
            })?;
        Ok((release, version, asset, asset_name))
    }

    async fn asset_for_version(
        &self,
        binary: &str,
        version: &str,
        target: &str,
    ) -> Result<(GitHubAsset, String)> {
        let release = self
            .release_for_tag(&format!("{}{}", self.tag_prefix, version))
            .await?;
        let asset_name = self.asset_template.archive_name(binary, version, target);
        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .cloned()
            .ok_or_else(|| Error::MissingAsset {
                asset_name: asset_name.clone(),
            })?;
        Ok((asset, asset_name))
    }

    async fn checksum_for(
        &self,
        release: &GitHubRelease,
        asset: &GitHubAsset,
    ) -> Result<Option<String>> {
        let checksum_name = format!("{}.sha256", asset.name);
        let Some(checksum_asset) = release
            .assets
            .iter()
            .find(|asset| asset.name == checksum_name)
        else {
            return sha256_from_asset_digest(asset);
        };
        let bytes = self.download_asset(checksum_asset).await?;
        let text = String::from_utf8_lossy(&bytes);
        let Some(checksum) = text.split_whitespace().next() else {
            return Err(Error::InvalidChecksumAsset {
                asset_name: checksum_name,
            });
        };
        if checksum.len() == 64 && checksum.chars().all(|ch| ch.is_ascii_hexdigit()) {
            return Ok(Some(checksum.to_owned()));
        }
        Err(Error::InvalidChecksumAsset {
            asset_name: checksum_name,
        })
    }

    fn version_from_tag(&self, tag: &str) -> Result<String> {
        tag.strip_prefix(&self.tag_prefix)
            .filter(|version| !version.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| Error::InvalidReleaseTag {
                tag: tag.to_owned(),
            })
    }
}

#[async_trait]
impl ReleaseProvider for GitHubReleaseProvider {
    async fn index(&self, binaries: &[String]) -> Result<ReleaseIndex> {
        let release = self.latest_release().await?;
        let version = self.version_from_tag(&release.tag_name)?;
        let mut releases = Vec::new();
        for asset in &release.assets {
            let Some(mut info) =
                self.asset_template
                    .asset_info_from_name(&asset.name, &version, None)?
            else {
                continue;
            };
            if !binaries.iter().any(|binary| binary == &info.binary) {
                continue;
            }
            info.sha256 = self.checksum_for(&release, asset).await?;
            releases.push(info);
        }
        Ok(ReleaseIndex { releases })
    }

    async fn latest(&self, binary: &str, target: &str) -> Result<ReleaseAssetInfo> {
        let (release, version, asset, asset_name) =
            self.latest_versioned_asset(binary, target).await?;
        let checksum = self.checksum_for(&release, &asset).await?;
        self.asset_template
            .asset_info_from_name(&asset.name, &version, checksum)?
            .ok_or(Error::MissingAsset { asset_name })
    }

    async fn download(&self, binary: &str, version: &str, target: &str) -> Result<ReleaseDownload> {
        let (asset, asset_name) = self.asset_for_version(binary, version, target).await?;
        let bytes = self.download_asset(&asset).await?;
        Ok(ReleaseDownload { asset_name, bytes })
    }
}

#[cfg(test)]
#[path = "_tests_/github/mod.rs"]
mod github_tests;
