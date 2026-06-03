use super::*;

use crate::Error;

#[test]
fn install_config_requires_default_binary_to_be_allowed() {
    let error = InstallConfig::new(
        vec!["install.example.com".to_owned()],
        Some("missing-cli".to_owned()),
        vec!["example-cli".to_owned()],
        Some("https://releases.example.com".to_owned()),
        DEFAULT_CLIENT_SERVER_URL_ENV.to_owned(),
        DEFAULT_INSTALL_DIR_ENV.to_owned(),
    )
    .expect_err("default binary should be rejected");

    assert!(matches!(
        error,
        Error::DefaultBinaryNotConfigured { binary } if binary == "missing-cli"
    ));
}

#[test]
fn install_config_requires_public_url_for_scripts() {
    let error = InstallConfig::new(
        vec!["install.example.com".to_owned()],
        Some("example-cli".to_owned()),
        vec!["example-cli".to_owned()],
        None,
        DEFAULT_CLIENT_SERVER_URL_ENV.to_owned(),
        DEFAULT_INSTALL_DIR_ENV.to_owned(),
    )
    .expect_err("missing public URL should be rejected");

    assert!(matches!(error, Error::MissingPublicReleaseServerUrl));
}

#[test]
fn install_config_requires_default_binary_for_install_hosts() {
    let error = InstallConfig::new(
        vec!["install.example.com".to_owned()],
        None,
        vec!["example-cli".to_owned()],
        Some("https://releases.example.com".to_owned()),
        DEFAULT_CLIENT_SERVER_URL_ENV.to_owned(),
        DEFAULT_INSTALL_DIR_ENV.to_owned(),
    )
    .expect_err("install hosts should require a default binary");

    assert!(matches!(error, Error::MissingDefaultInstallBinary));
}

#[test]
fn install_config_rejects_invalid_env_names() {
    let error = InstallConfig::new(
        Vec::new(),
        None,
        Vec::new(),
        None,
        "invalid-name".to_owned(),
        DEFAULT_INSTALL_DIR_ENV.to_owned(),
    )
    .expect_err("invalid env name should be rejected");

    assert!(matches!(
        error,
        Error::InvalidEnvVarName { value } if value == "invalid-name"
    ));
}

#[test]
fn install_config_rejects_invalid_binary_names() {
    let error = InstallConfig::new(
        Vec::new(),
        None,
        vec!["example-cli\"$(touch marker)".to_owned()],
        Some("https://releases.example.com".to_owned()),
        DEFAULT_CLIENT_SERVER_URL_ENV.to_owned(),
        DEFAULT_INSTALL_DIR_ENV.to_owned(),
    )
    .expect_err("invalid binary name should be rejected");

    assert!(matches!(
        error,
        Error::InvalidInstallBinaryName { binary } if binary == "example-cli\"$(touch marker)"
    ));
}

#[test]
fn server_config_requires_binary_allowlist_for_github_provider() {
    let install = InstallConfig::new(
        Vec::new(),
        None,
        Vec::new(),
        None,
        DEFAULT_CLIENT_SERVER_URL_ENV.to_owned(),
        DEFAULT_INSTALL_DIR_ENV.to_owned(),
    )
    .expect("install config should allow install-script-only empty binaries");
    let config = ServerConfig {
        bind: DEFAULT_BIND.to_owned(),
        github: Some(github_config()),
        install,
    };

    let error = config
        .validate()
        .expect_err("GitHub-backed config should require binaries");

    assert!(matches!(error, Error::MissingReleaseBinaries));
}

#[test]
fn github_config_rejects_partial_configuration() {
    let error = github_from_values(
        Some("owner/repo".to_owned()),
        None,
        DEFAULT_GITHUB_API_URL.to_owned(),
        DEFAULT_TAG_PREFIX,
        DEFAULT_ASSET_TEMPLATE,
    )
    .expect_err("partial GitHub config should fail");

    assert!(matches!(error, Error::IncompleteGitHubConfig { .. }));

    let error = github_from_values(
        None,
        Some("token".to_owned()),
        DEFAULT_GITHUB_API_URL.to_owned(),
        DEFAULT_TAG_PREFIX,
        DEFAULT_ASSET_TEMPLATE,
    )
    .expect_err("partial GitHub config should fail");

    assert!(matches!(error, Error::IncompleteGitHubConfig { .. }));
}

fn github_config() -> GitHubConfig {
    GitHubConfig {
        repository: "owner/repo".to_owned(),
        api_url: DEFAULT_GITHUB_API_URL.to_owned(),
        token: "token".to_owned(),
        tag_prefix: DEFAULT_TAG_PREFIX.to_owned(),
        asset_template: DEFAULT_ASSET_TEMPLATE.to_owned(),
    }
}
