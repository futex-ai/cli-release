//! HTTP routes for release metadata, archive downloads, and install scripts.

mod releases;
mod root;
mod state;

use std::sync::Arc;

use axum::{
    Json, Router,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Serialize;

use crate::config::ServerConfig;
use crate::github::GitHubReleaseProvider;
use crate::{DynReleaseProvider, Error};

use self::{
    releases::{download, index, latest},
    root::{install_script, root},
    state::ReleaseRouteState,
};

use root::validate_path_component;

/// Returns the full release-server HTTP application.
pub fn app_from_env() -> crate::Result<Router> {
    app(ServerConfig::from_env()?)
}

/// Returns the full release-server HTTP application.
pub fn app(config: ServerConfig) -> crate::Result<Router> {
    let provider = config
        .github
        .map(GitHubReleaseProvider::new)
        .transpose()?
        .map(|provider| Arc::new(provider) as DynReleaseProvider);
    Ok(app_with_provider(provider, config.install))
}

/// Returns the full release-server HTTP application for supplied dependencies.
pub fn app_with_provider(
    provider: Option<DynReleaseProvider>,
    install: crate::config::InstallConfig,
) -> Router {
    let state = ReleaseRouteState { provider, install };
    Router::new()
        .route("/", get(root))
        .route("/{binary}", get(install_script))
        .merge(release_routes())
        .with_state(state)
}

/// Returns release HTTP routes using environment-based GitHub configuration.
pub fn release_routes_from_env() -> crate::Result<Router> {
    let config = ServerConfig::from_env()?;
    app(config)
}

/// Returns release HTTP routes.
fn release_routes() -> Router<ReleaseRouteState> {
    Router::new()
        .route("/releases/latest", get(index))
        .route("/releases/latest/{binary}/{target}", get(latest))
        .route(
            "/releases/download/{binary}/{version}/{target}",
            get(download),
        )
}

type HttpResult<T> = std::result::Result<T, HttpError>;

#[derive(Debug)]
struct HttpError(Error);

impl From<Error> for HttpError {
    fn from(error: Error) -> Self {
        Self(error)
    }
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let status = match self.0 {
            Error::NotConfigured => StatusCode::SERVICE_UNAVAILABLE,
            Error::InvalidPathComponent { .. } | Error::InvalidReleaseTag { .. } => {
                StatusCode::BAD_REQUEST
            }
            Error::UnsupportedInstallBinary { .. } | Error::MissingAsset { .. } => {
                StatusCode::NOT_FOUND
            }
            Error::MissingDefaultInstallBinary
            | Error::MissingPublicReleaseServerUrl
            | Error::DefaultBinaryNotConfigured { .. }
            | Error::MissingReleaseBinaries
            | Error::InvalidEnvVarName { .. }
            | Error::InvalidInstallBinaryName { .. }
            | Error::IncompleteGitHubConfig { .. }
            | Error::InvalidAssetTemplate { .. } => StatusCode::BAD_REQUEST,
            Error::GitHubRequest { .. }
            | Error::GitHubClientBuild { .. }
            | Error::GitHubStatus { .. }
            | Error::GitHubDecode { .. }
            | Error::InvalidChecksumAsset { .. }
            | Error::InvalidAssetDigest { .. } => StatusCode::BAD_GATEWAY,
        };
        let body = Json(ErrorBody {
            error: self.0.to_string(),
        });
        (status, body).into_response()
    }
}

#[cfg(test)]
#[path = "_tests_/routes_tests.rs"]
mod routes_tests;
