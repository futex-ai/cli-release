# cli-release-interface

`cli-release-interface` defines the JSON contract shared by released CLI
binaries, updater clients, and `cli-release-server`.

## Responsibilities

- Define one release asset record with binary, version, target, archive name,
  download URL, and optional SHA-256 checksum.
- Define the latest-release index returned by the release server.
- Keep the release metadata contract stable and serialization-friendly across
  CLI binaries, the release server, and tests.

## What This Crate Does

This crate intentionally owns only typed metadata payloads. It has no HTTP,
filesystem, GitHub, or installer dependencies, so app-specific CLI crates can
depend on it without pulling in release-server runtime code.

## Quick Start

```rust
use cli_release_interface::{ReleaseAssetInfo, ReleaseIndex};

let asset = ReleaseAssetInfo {
    binary: "example-cli".to_owned(),
    version: "1.2.3".to_owned(),
    target: "x86_64-unknown-linux-gnu".to_owned(),
    asset_name: "example-cli-1.2.3-x86_64-unknown-linux-gnu.tar.gz".to_owned(),
    download_url: "/releases/download/example-cli/1.2.3/x86_64-unknown-linux-gnu".to_owned(),
    sha256: Some("0123456789abcdef".repeat(4)),
};

let index = ReleaseIndex {
    releases: vec![asset.clone()],
};

assert_eq!(index.releases[0], asset);
```

## Development

```bash
cargo test -p cli-release-interface
cargo clippy -p cli-release-interface --all-targets --all-features -- -D warnings
cargo xtask check
```

When release metadata changes, update the release-server README and protocol
docs that describe the endpoint contract.

### Key Code

- `src/lib.rs` - `ReleaseAssetInfo`, `ReleaseIndex`, and serde derives.

### Related Docs

- [`../cli-updater/README.md`](../cli-updater/README.md)
- [`../cli-release-server/README.md`](../cli-release-server/README.md)
- [`../../docs/protocol/cli-releases.md`](../../docs/protocol/cli-releases.md)
