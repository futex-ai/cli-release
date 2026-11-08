# CLI Releases

This document defines the target contract for the standalone `cli-release`
workspace. The shared server and updater code must stay free of
product-specific defaults.

## Goals

- Share one release metadata contract across CLI binaries, updater clients, and
  the release server.
- Allow one release-server deployment to serve one GitHub repository and one or
  more CLI binaries from that repository.
- Configure repository, hostnames, public release URL, binary allowlist, tag
  convention, and asset naming through environment-backed configuration.
- Let each deployment choose a release channel: full releases only, or
  prereleases too.
- Keep GitHub credentials in the server deployment only. CLI users and updater
  clients must not need GitHub credentials.
- Keep deployment-specific choices in infrastructure values or environment
  variables, not in Rust code.

## Release Metadata

`cli-release-interface` owns the JSON response types.

`ReleaseAssetInfo` includes:

- `binary`: CLI executable name.
- `version`: release version without a tag prefix.
- `target`: release target triple.
- `asset_name`: archive asset name in the upstream release.
- `download_url`: absolute URL or release-server-relative archive URL. Updater
  clients accept server-relative paths with or without a leading `/`.
- `sha256`: optional archive SHA-256 digest.

`ReleaseIndex` wraps:

- `releases`: array of `ReleaseAssetInfo` records.

The server returns metadata as JSON. Archive downloads remain binary
`application/gzip` responses.

## Server Endpoints

The release server exposes:

```text
GET /
GET /{binary}
GET /releases/latest
GET /releases/latest/{binary}/{target}
GET /releases/download/{binary}/{version}/{target}
```

`GET /` returns service metadata unless the request host matches a configured
install host. On an install host, `GET /` returns the installer for the
configured default binary.

`GET /{binary}` returns an installer only for configured binaries. Unknown
binaries return `404`. Release metadata and download routes are also scoped to
`CLI_RELEASE_BINARIES`; `GET /releases/latest` filters out assets for
unconfigured binaries, and binary-specific release routes return `404` for
unconfigured binaries.

Release path components must be non-empty and limited to ASCII alphanumeric
characters plus `.`, `_`, and `-`.

## Server Configuration

The shared server must not hardcode product names, GitHub repositories, binary
names, install hosts, release hosts, or public release URLs.

Primary environment variables:

- `CLI_RELEASE_GITHUB_REPOSITORY`: GitHub repository, for example
  `owner/repo`.
- `CLI_RELEASE_GITHUB_TOKEN`: token that can read private GitHub Release
  assets.
- `CLI_RELEASE_GITHUB_API_URL`: optional GitHub API base URL. Defaults to
  `https://api.github.com`.
- `CLI_RELEASE_SERVER_BIND`: bind address. Defaults to `0.0.0.0:8080`.
- `CLI_RELEASE_PUBLIC_RELEASE_SERVER_URL`: URL embedded in install scripts.
- `CLI_RELEASE_INSTALL_HOSTS`: comma-separated hostnames that should serve
  install scripts from `/`.
- `CLI_RELEASE_DEFAULT_BINARY`: binary installed from `/` on install hosts.
  Required when `CLI_RELEASE_INSTALL_HOSTS` is configured.
- `CLI_RELEASE_BINARIES`: comma-separated binary allowlist. Required for
  GitHub-backed deployments.
- `CLI_RELEASE_TAG_PREFIX`: release tag prefix. Defaults to `v`.
- `CLI_RELEASE_ASSET_TEMPLATE`: archive naming template. Defaults to
  `{binary}-{version}-{target}.tar.gz`.
- `CLI_RELEASE_CHANNEL`: release channel for the latest metadata routes,
  `stable` or `prerelease`. Defaults to `stable`.
- `CLI_RELEASE_CLIENT_SERVER_URL_ENV`: environment variable name emitted in
  install scripts for client-side release-server overrides. Defaults to
  `CLI_RELEASE_SERVER_URL`.
- `CLI_RELEASE_LOG_LEVEL`: log level for the server process.
- `CLI_RELEASE_LOG_FORMAT`: log format for the server process.

The shared implementation must use the `CLI_RELEASE_*` variables above. It must
not add product-specific compatibility aliases such as `JUNO_*`.
GitHub-backed deployments must set both `CLI_RELEASE_GITHUB_REPOSITORY` and
`CLI_RELEASE_GITHUB_TOKEN`, or neither. Partial GitHub configuration is invalid
so a misconfigured release server fails before serving routes. GitHub-backed
deployments must also set a non-empty `CLI_RELEASE_BINARIES` allowlist.
Install-host deployments must set `CLI_RELEASE_DEFAULT_BINARY` to a binary
listed in `CLI_RELEASE_BINARIES`; otherwise startup fails before serving
installer traffic. Configured binary names must be valid release path
components: non-empty ASCII alphanumeric strings plus `.`, `_`, and `-`.

## GitHub Release Contract

The GitHub provider reads releases from the configured repository. A release
tag must be `CLI_RELEASE_TAG_PREFIX` followed by a non-empty release version.
This is the tag rule.

Archive assets are matched with `CLI_RELEASE_ASSET_TEMPLATE`. The default
template supports names such as:

```text
example-cli-1.2.3-aarch64-apple-darwin.tar.gz
```

Checksum metadata is resolved from the matching `.sha256` companion asset when
present. If no companion exists, the server may use GitHub's asset digest when
it is a valid `sha256:<hex>` value.
For release indexes, the provider skips assets whose binary is not in
`CLI_RELEASE_BINARIES` before checksum resolution. Invalid checksum metadata for
unconfigured binaries must not fail the index route.

Versioned download routes must fetch the GitHub release identified by
`CLI_RELEASE_TAG_PREFIX` plus the requested version, not the repository's
current latest release. The release channel does not apply to these routes.
Both channels serve each published version, prereleases included. A channel
filter for downloads was rejected, because an exact tag names one release and
clients get the channel's choice from the latest routes.
GitHub API and asset HTTP requests must use explicit request and connect
timeouts so stalled upstream calls cannot hang server route handlers
indefinitely.

## Release Channels

`CLI_RELEASE_CHANNEL` selects one release for `GET /releases/latest` and
`GET /releases/latest/{binary}/{target}`. Both routes read only that release,
also when it has no asset for a binary or target.

- `stable` selects the release that GitHub reports as the latest release of
  the repository. GitHub never reports a draft or a prerelease as latest.
- `prerelease` reads the GitHub release list in the order that GitHub
  returns it. It selects the first release that is not a draft and matches
  the tag rule. Prereleases and full releases both qualify.

`prerelease` reads later pages of the list until it finds a match or reaches
the end. A draft never matches, also when the GitHub token can list drafts.
When no release matches, both latest routes return `404`. A semantic-version
sort was rejected, because the tag rule does not require semantic versions.
Full releases qualify, because preview clients must also get a newer full
release.

With `stable`, a latest release that breaks the tag rule fails the latest
routes. In both channels, a failed GitHub request returns `502`. For `stable`,
this includes GitHub's `404` for a repository without a full release.

An unset or empty value selects `stable`. A value other than `stable` or
`prerelease` logs a startup warning that names the value. The server then
uses `stable`. A startup failure was rejected, because graceful handling keeps
the server up and `stable` keeps prereleases opt-in.

## Installer Contract

Install scripts:

- Support Linux and macOS on `x86_64` and `aarch64`.
- Resolve the release server from the configured client override env var, then
  from `CLI_RELEASE_PUBLIC_RELEASE_SERVER_URL`.
- Fetch `/releases/latest/{binary}/{target}`.
- Download the archive from `download_url`.
- Require a SHA-256 checksum in metadata before installing.
- Reject archive members with absolute paths, parent-directory traversal, or
  option-like leading dashes before extraction.
- Extract only the selected binary member to a fixed temporary path instead of
  unpacking the full archive.
- Install into `CLI_RELEASE_INSTALL_DIR` when set, `/usr/local/bin` when
  writable, or `$HOME/.local/bin` otherwise.

An install script installs the release that the channel of its server
selects. It has no channel option of its own.

Install scripts must not embed product-specific domains or binary names except
for values supplied by server configuration.

### Script Arguments

Script arguments are the arguments that the shell passes to the install script.
In `sh -s -- install --token '<token>'`, they are `install`, `--token`, and
`<token>`. The `--` ends the options of `sh`, so it is not a script argument.

- Without script arguments, the script installs the binary and does not run it.
- With script arguments, the script runs the installed binary once after the
  install.
- The binary receives exactly the script arguments: the same number, in the
  same order, with the same bytes.
- The script exits with the exit status of the binary.
- The script removes its temporary directory for any exit status of the binary.
- The script and the server must not log script argument values or put them in
  error messages, because the values can carry secrets such as tokens. The
  installed binary owns its own output.
- The rule applies to every configured binary, with no setting and no
  product-specific default.

## Updater Contract

`cli-updater` checks for updates and installs them:

- It fetches release metadata and archive bytes from the release server.
- It verifies the archive checksum when the metadata has one. It then
  replaces the current executable.
- It compares the installed and latest versions as semantic versions.
- An upgrade installs only a newer version, unless the caller forces it.
- Its HTTP client uses explicit request and connect timeouts. Callers can set
  custom timeout values.

The updater gets the channel's release from the latest metadata route. It
has no channel option of its own. In semantic-version order, a prerelease
such as `1.3.0-rc.1` is older than `1.3.0`. A client on a newer prerelease
keeps it after its server moves back to `stable`, unless the caller forces an
upgrade. The server accepts a version that is not a semantic version, but the
update check fails for it.

The shared updater must not hardcode a product release-server URL. Consuming CLI
crates pass their preferred default URL and override env var name at their
composition boundary.

## Release Automation Ownership

App-specific repositories own CLI binary releases. They run release-plz, create
version tags, build their CLI binaries, upload archive assets to their GitHub
Release, and deploy the shared release server with `CLI_RELEASE_*` values that
point back to that app repository.

This repository owns reusable release tooling: packaging scripts, validation
tests, documentation, optional reusable workflow templates, the generic
release-server image, and deployment chart examples. It also runs release-plz
for versioning and releasing this workspace's shared crates, including
`cli-release-interface`, `cli-updater`, and `cli-release-server`.
On pushes to `main`, the shared release workflow publishes the generic
`cli-release-server` runtime image after workspace checks pass.

The in-repo release-plz workflow must only release shared workspace artifacts.
It must not copy an app repository's `.github/workflows/release.yml` as an
active workflow that publishes app CLI assets from this repository.

## Validation

Implementation changes must include focused tests for:

- server configuration parsing and validation;
- install-host routing and installer rendering;
- binary allowlist behavior;
- release tag prefix handling;
- asset template matching;
- checksum sidecar and GitHub digest handling;
- updater URL resolution and upgrade behavior.

Before completion, run relevant package tests, `cargo fmt --all -- --check`,
clippy, `cargo xtask check`, then `cargo xtask review`.
