# cli-release

`cli-release` is a standalone Rust workspace for shared CLI release
infrastructure: release metadata types, updater logic, packaging helpers, and a
configurable release server for private GitHub Release assets.

## Key Features

- Shared JSON release metadata contract for CLI update checks.
- Trait-backed updater library for status checks and explicit self-upgrades.
- Generic HTTP release server for install scripts, latest metadata, and
  proxied archive downloads.
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

## Developer Get Started

```bash
cargo test --workspace
scripts/test-cli-release.sh
cargo run -p cli-release-server -- serve --bind 127.0.0.1:8080
```

Run the full local check suite before completion:

```bash
cargo xtask check
cargo xtask review
```

## Key Code

- `crates/cli-release-interface` - shared release metadata JSON types.
- `crates/cli-updater` - updater client and installer library.
- `crates/cli-release-server` - HTTP install and release metadata service.
- `scripts/cli-release.sh` - generic tag validation and archive packaging.
- `helm/cli-release-server` - reusable release-server chart.
- `plans/` - active and completed implementation plans.
- `docs/protocol/` - release protocol and deployment contracts.

## Links

- [Plans](./plans/README.md)
- [Documentation](./docs/README.md)
- [CLI release protocol](./docs/protocol/cli-releases.md)
