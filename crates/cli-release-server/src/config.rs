//! Environment-backed release-server configuration.

use crate::{Error, Result};

/// Server bind environment variable.
pub const BIND_ENV: &str = "CLI_RELEASE_SERVER_BIND";
/// GitHub repository environment variable.
pub const GITHUB_REPOSITORY_ENV: &str = "CLI_RELEASE_GITHUB_REPOSITORY";
/// GitHub token environment variable.
pub const GITHUB_TOKEN_ENV: &str = "CLI_RELEASE_GITHUB_TOKEN";
/// GitHub API URL environment variable.
pub const GITHUB_API_URL_ENV: &str = "CLI_RELEASE_GITHUB_API_URL";
/// Public release-server URL environment variable.
pub const PUBLIC_RELEASE_SERVER_URL_ENV: &str = "CLI_RELEASE_PUBLIC_RELEASE_SERVER_URL";
/// Install host allowlist environment variable.
pub const INSTALL_HOSTS_ENV: &str = "CLI_RELEASE_INSTALL_HOSTS";
/// Default binary environment variable.
pub const DEFAULT_BINARY_ENV: &str = "CLI_RELEASE_DEFAULT_BINARY";
/// Binary allowlist environment variable.
pub const BINARIES_ENV: &str = "CLI_RELEASE_BINARIES";
/// Release tag prefix environment variable.
pub const TAG_PREFIX_ENV: &str = "CLI_RELEASE_TAG_PREFIX";
/// GitHub asset template environment variable.
pub const ASSET_TEMPLATE_ENV: &str = "CLI_RELEASE_ASSET_TEMPLATE";
/// Client release-server override env-name environment variable.
pub const CLIENT_SERVER_URL_ENV_ENV: &str = "CLI_RELEASE_CLIENT_SERVER_URL_ENV";
/// Install directory override env-name environment variable.
pub const INSTALL_DIR_ENV_ENV: &str = "CLI_RELEASE_INSTALL_DIR_ENV";

/// Default bind address.
pub const DEFAULT_BIND: &str = "0.0.0.0:8080";
/// Default GitHub API base URL.
pub const DEFAULT_GITHUB_API_URL: &str = "https://api.github.com";
/// Default release tag prefix.
pub const DEFAULT_TAG_PREFIX: &str = "v";
/// Default archive asset template.
pub const DEFAULT_ASSET_TEMPLATE: &str = "{binary}-{version}-{target}.tar.gz";
/// Default client override variable emitted by install scripts.
pub const DEFAULT_CLIENT_SERVER_URL_ENV: &str = "CLI_RELEASE_SERVER_URL";
/// Default install directory variable emitted by install scripts.
pub const DEFAULT_INSTALL_DIR_ENV: &str = "CLI_RELEASE_INSTALL_DIR";

/// Complete release-server configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServerConfig {
    /// HTTP bind address.
    pub bind: String,
    /// Optional GitHub-backed release provider configuration.
    pub github: Option<GitHubConfig>,
    /// Install-script route configuration.
    pub install: InstallConfig,
}

/// GitHub provider configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitHubConfig {
    /// Repository in `owner/repo` form.
    pub repository: String,
    /// GitHub API base URL.
    pub api_url: String,
    /// Token that can read private release assets.
    pub token: String,
    /// Tag prefix before the release version.
    pub tag_prefix: String,
    /// Archive asset template.
    pub asset_template: String,
}

/// Install-script route configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstallConfig {
    /// Hostnames that serve the default install script at `/`.
    pub install_hosts: Vec<String>,
    /// Binary installed from `/` on install hosts.
    pub default_binary: Option<String>,
    /// Supported binary install-script names.
    pub binaries: Vec<String>,
    /// Public release-server URL embedded in install scripts.
    pub public_release_server_url: Option<String>,
    /// Client-side release-server override variable name.
    pub client_server_url_env: String,
    /// Client-side install-directory override variable name.
    pub install_dir_env: String,
}

impl ServerConfig {
    /// Reads configuration from process environment.
    pub fn from_env() -> Result<Self> {
        let bind = env_value(BIND_ENV).unwrap_or_else(|| DEFAULT_BIND.to_owned());
        let tag_prefix = env_value(TAG_PREFIX_ENV).unwrap_or_else(|| DEFAULT_TAG_PREFIX.to_owned());
        let asset_template =
            env_value(ASSET_TEMPLATE_ENV).unwrap_or_else(|| DEFAULT_ASSET_TEMPLATE.to_owned());
        let github = github_from_env(&tag_prefix, &asset_template)?;
        let install = InstallConfig::from_env()?;
        let config = Self {
            bind,
            github,
            install,
        };
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        if self.github.is_some() && self.install.binaries.is_empty() {
            return Err(Error::MissingReleaseBinaries);
        }
        Ok(())
    }
}

impl InstallConfig {
    /// Reads install configuration from process environment.
    pub fn from_env() -> Result<Self> {
        let install_hosts = env_list(INSTALL_HOSTS_ENV);
        let binaries = env_list(BINARIES_ENV);
        let default_binary = env_value(DEFAULT_BINARY_ENV);
        let public_release_server_url = env_value(PUBLIC_RELEASE_SERVER_URL_ENV);
        let client_server_url_env = env_value(CLIENT_SERVER_URL_ENV_ENV)
            .unwrap_or_else(|| DEFAULT_CLIENT_SERVER_URL_ENV.to_owned());
        let install_dir_env =
            env_value(INSTALL_DIR_ENV_ENV).unwrap_or_else(|| DEFAULT_INSTALL_DIR_ENV.to_owned());
        validate_env_name(&client_server_url_env)?;
        validate_env_name(&install_dir_env)?;
        let config = Self {
            install_hosts,
            default_binary,
            binaries,
            public_release_server_url,
            client_server_url_env,
            install_dir_env,
        };
        config.validate()?;
        Ok(config)
    }

    /// Returns a config useful for tests and examples.
    pub fn new(
        install_hosts: Vec<String>,
        default_binary: Option<String>,
        binaries: Vec<String>,
        public_release_server_url: Option<String>,
        client_server_url_env: String,
        install_dir_env: String,
    ) -> Result<Self> {
        validate_env_name(&client_server_url_env)?;
        validate_env_name(&install_dir_env)?;
        let config = Self {
            install_hosts,
            default_binary,
            binaries,
            public_release_server_url,
            client_server_url_env,
            install_dir_env,
        };
        config.validate()?;
        Ok(config)
    }

    /// Returns the default binary if install-host routing is configured.
    pub fn default_binary(&self) -> Result<&str> {
        self.default_binary
            .as_deref()
            .ok_or(Error::MissingDefaultInstallBinary)
    }

    /// Returns whether `binary` has an install script.
    pub fn supports_binary(&self, binary: &str) -> bool {
        self.binaries.iter().any(|candidate| candidate == binary)
    }

    /// Returns the public release-server URL.
    pub fn public_release_server_url(&self) -> Result<&str> {
        self.public_release_server_url
            .as_deref()
            .ok_or(Error::MissingPublicReleaseServerUrl)
    }

    fn validate(&self) -> Result<()> {
        for binary in &self.binaries {
            validate_install_binary_name(binary)?;
        }
        if let Some(default_binary) = &self.default_binary {
            validate_install_binary_name(default_binary)?;
        }
        if !self.install_hosts.is_empty() && self.default_binary.is_none() {
            return Err(Error::MissingDefaultInstallBinary);
        }
        if let Some(default_binary) = &self.default_binary
            && !self.supports_binary(default_binary)
        {
            return Err(Error::DefaultBinaryNotConfigured {
                binary: default_binary.clone(),
            });
        }
        if (!self.install_hosts.is_empty() || !self.binaries.is_empty())
            && self.public_release_server_url.is_none()
        {
            return Err(Error::MissingPublicReleaseServerUrl);
        }
        Ok(())
    }
}

fn github_from_env(tag_prefix: &str, asset_template: &str) -> Result<Option<GitHubConfig>> {
    github_from_values(
        env_value(GITHUB_REPOSITORY_ENV),
        env_value(GITHUB_TOKEN_ENV),
        env_value(GITHUB_API_URL_ENV).unwrap_or_else(|| DEFAULT_GITHUB_API_URL.to_owned()),
        tag_prefix,
        asset_template,
    )
}

fn github_from_values(
    repository: Option<String>,
    token: Option<String>,
    api_url: String,
    tag_prefix: &str,
    asset_template: &str,
) -> Result<Option<GitHubConfig>> {
    match (repository, token) {
        (None, None) => Ok(None),
        (Some(repository), Some(token)) => Ok(Some(GitHubConfig {
            repository,
            api_url,
            token,
            tag_prefix: tag_prefix.to_owned(),
            asset_template: asset_template.to_owned(),
        })),
        _ => Err(Error::IncompleteGitHubConfig {
            repository_env: GITHUB_REPOSITORY_ENV,
            token_env: GITHUB_TOKEN_ENV,
        }),
    }
}

fn env_value(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

fn env_list(name: &str) -> Vec<String> {
    env_value(name)
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn validate_install_binary_name(binary: &str) -> Result<()> {
    if !binary.is_empty()
        && binary
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
    {
        return Ok(());
    }
    Err(Error::InvalidInstallBinaryName {
        binary: binary.to_owned(),
    })
}

fn validate_env_name(value: &str) -> Result<()> {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return Err(Error::InvalidEnvVarName {
            value: value.to_owned(),
        });
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(Error::InvalidEnvVarName {
            value: value.to_owned(),
        });
    }
    if chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
        return Ok(());
    }
    Err(Error::InvalidEnvVarName {
        value: value.to_owned(),
    })
}

#[cfg(test)]
#[path = "_tests_/config_tests.rs"]
mod config_tests;
