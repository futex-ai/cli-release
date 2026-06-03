//! Release-server clients.

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use cli_release_interface::ReleaseAssetInfo;

use crate::{Error, Result};

/// Default total request timeout for release-server HTTP calls.
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Default TCP connect timeout for release-server HTTP calls.
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Shared release-client trait object.
pub type DynReleaseClient = Arc<dyn ReleaseClient + Send + Sync>;

/// Release-server API used by the updater.
#[unimock::unimock(api = ReleaseClientMock)]
#[async_trait]
pub trait ReleaseClient {
    /// Returns the latest release asset for a binary and target.
    async fn latest(&self, binary: &str, target: &str) -> Result<ReleaseAssetInfo>;

    /// Downloads a release archive.
    async fn download(&self, release: &ReleaseAssetInfo) -> Result<Vec<u8>>;
}

/// HTTP implementation for a compatible release server.
#[derive(Clone, Debug)]
pub struct HttpReleaseClient {
    server_url: String,
    client: reqwest::Client,
}

impl HttpReleaseClient {
    /// Creates a release client for a release-server URL.
    pub fn new(server_url: impl Into<String>) -> Result<Self> {
        Self::new_with_timeouts(server_url, DEFAULT_REQUEST_TIMEOUT, DEFAULT_CONNECT_TIMEOUT)
    }

    /// Creates a release client with explicit request and connect timeouts.
    pub fn new_with_timeouts(
        server_url: impl Into<String>,
        request_timeout: Duration,
        connect_timeout: Duration,
    ) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(request_timeout)
            .connect_timeout(connect_timeout)
            .build()
            .map_err(|source| Error::ReleaseServerClientBuild { source })?;
        Ok(Self {
            server_url: server_url.into(),
            client,
        })
    }

    fn endpoint(&self, path: &str) -> Result<String> {
        let base = self.server_url.trim_end_matches('/');
        if base.is_empty() {
            return Err(Error::InvalidReleaseServerUrl {
                url: self.server_url.clone(),
            });
        }
        let path = if path.starts_with('/') {
            path.to_owned()
        } else {
            format!("/{path}")
        };
        Ok(format!("{base}{path}"))
    }
}

#[async_trait]
impl ReleaseClient for HttpReleaseClient {
    async fn latest(&self, binary: &str, target: &str) -> Result<ReleaseAssetInfo> {
        let url = self.endpoint(&format!("/releases/latest/{binary}/{target}"))?;
        let response =
            self.client
                .get(&url)
                .send()
                .await
                .map_err(|source| Error::ReleaseServerRequest {
                    url: url.clone(),
                    source,
                })?;
        let status = response.status();
        if !status.is_success() {
            return Err(Error::ReleaseServerStatus {
                url,
                status: status.as_u16(),
            });
        }
        response
            .json::<ReleaseAssetInfo>()
            .await
            .map_err(|source| Error::ReleaseServerDecode { url, source })
    }

    async fn download(&self, release: &ReleaseAssetInfo) -> Result<Vec<u8>> {
        let url = if release.download_url.starts_with("http://")
            || release.download_url.starts_with("https://")
        {
            release.download_url.clone()
        } else {
            self.endpoint(&release.download_url)?
        };
        let response =
            self.client
                .get(&url)
                .send()
                .await
                .map_err(|source| Error::ReleaseServerRequest {
                    url: url.clone(),
                    source,
                })?;
        let status = response.status();
        if !status.is_success() {
            return Err(Error::ReleaseServerStatus {
                url,
                status: status.as_u16(),
            });
        }
        response
            .bytes()
            .await
            .map(|bytes| bytes.to_vec())
            .map_err(|source| Error::ReleaseServerDecode { url, source })
    }
}

#[cfg(test)]
#[path = "_tests_/client_tests.rs"]
mod client_tests;
