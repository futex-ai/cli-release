# cli-release

`cli-release` is a standalone Rust workspace for shared CLI release
infrastructure: release metadata types, updater logic, packaging helpers, and a
configurable release server for private GitHub Release assets.

## Key Features

- Shared JSON release metadata contract for CLI update checks.
- Trait-backed updater library for status checks and explicit self-upgrades.
- Generic HTTP release server for install scripts, latest metadata, and
  proxied archive downloads. Each deployment serves a `stable` or a
  `prerelease` release channel.
- Release packaging script, Docker image target, main-branch image publishing,
  Helm chart, and release-plz configuration for this workspace's shared crates.

## Interface

The release server exposes:

```text
GET /
GET /{binary}
GET /releases/latest
GET /releases/latest/{binary}/{target}
GET /releases/download/{binary}/{version}/{target}
```

App-specific repositories publish their own CLI binaries and deploy this
release server with `CLI_RELEASE_*` environment variables pointing at the app
repository's GitHub Releases.

`CLI_RELEASE_CHANNEL` selects the release behind the two latest routes. The
default, `stable`, serves GitHub's latest full release. `prerelease` also
serves prereleases. A preview deployment uses it, so that its install scripts
and updater checks get preview builds. The download route serves the exact
version in its path for both channels. See
[Release Channels](./docs/protocol/cli-releases.md#release-channels).

## Developer Get Started

```bash
cargo test --workspace
scripts/test-cli-release.sh
cargo run -p cli-release-server -- serve --bind 127.0.0.1:8080
```

On Linux, the server tests need extra shells. See the
[server test prerequisites](./crates/cli-release-server/README.md#test-prerequisites).

`cargo xtask check` is the complete gate. Run it before completion:

```bash
cargo xtask check
```

`cargo xtask review` is an optional manual review. It sends a summary of
the local diff to a read-only Codex reviewer.

## Key Code

- `crates/cli-release-interface` - shared release metadata JSON types.
- `crates/cli-updater` - updater client and installer library.
- `crates/cli-release-server` - HTTP install and release metadata service.
- `scripts/cli-release.sh` - generic tag validation and archive packaging.
- `helm/cli-release-server` - reusable release-server chart.
- `docs/protocol/` - release protocol and deployment contracts.

## Links

- [Documentation](./docs/README.md)
- [CLI release protocol](./docs/protocol/cli-releases.md)
