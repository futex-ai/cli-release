# cli-updater

`cli-updater` provides shared update checks and self-upgrade installation for
released CLI binaries that use a compatible `cli-release-server` deployment.

## Responsibilities

- Resolve the platform release target used by published CLI archives.
- Query a caller-supplied release server for latest release metadata.
- Compare the installed version with the latest published version.
- Download and install `.tar.gz` archives when an explicit upgrade command is
  run.
- Keep the update boundary trait-based through `ReleaseClient` and
  `BinaryInstaller`.

## What This Crate Does

CLI crates use this library to fetch matching archive metadata, compare
semantic versions, and replace the current executable. The production
implementation talks to `cli-release-server`; tests can swap in mocked release
and installer dependencies.

Release metadata is defined by `cli-release-interface`, so updater clients do
not depend on release-server internals.
`HttpReleaseClient` accepts absolute download URLs and server-relative download
paths with or without a leading `/`.

## Quick Start

```rust
use std::sync::Arc;

use cli_updater::{
    HttpReleaseClient, TarGzBinaryInstaller, UpdateCheck, UpdateRequest, Updater,
    current_target, resolve_release_server_url,
};

# #[tokio::main]
# async fn main() -> cli_updater::Result<()> {
let release_server = resolve_release_server_url(
    None,
    cli_updater::DEFAULT_RELEASE_SERVER_URL_ENV,
    "https://releases.example.com",
);
let updater = Updater::new(
    Arc::new(HttpReleaseClient::new(release_server)?),
    Arc::new(TarGzBinaryInstaller::new()),
);

let check = UpdateCheck {
    binary: "example-cli".to_owned(),
    current_version: env!("CARGO_PKG_VERSION").to_owned(),
    target: current_target()?.to_owned(),
};

let status = updater.status(check.clone()).await?;
println!("{}", cli_updater::format_status_summary(&status));

let request = UpdateRequest {
    check,
    current_exe: std::env::current_exe().map_err(|source| cli_updater::Error::CurrentExe {
        source,
    })?,
    force: false,
};

let _outcome = updater.upgrade(request).await?;
# Ok(())
# }
```

## Development

```bash
cargo test -p cli-updater
cargo clippy -p cli-updater --all-targets --all-features -- -D warnings
cargo xtask check
```

Unit tests should use the shared `ReleaseClientMock` and `BinaryInstallerMock`
APIs from this crate instead of handwritten concrete test doubles.
`HttpReleaseClient::new` builds a client with default request and connect
timeouts. Use `HttpReleaseClient::new_with_timeouts` when a caller needs a
different network policy.

### Key Code

- `src/lib.rs` - public exports for updater traits, installer, and target
  helpers.
- `src/manager.rs` - update checks, version comparison, URL resolution, and
  upgrade orchestration.
- `src/client.rs` - `ReleaseClient` plus production `HttpReleaseClient`.
- `src/installer.rs` - `BinaryInstaller` plus the production `.tar.gz`
  installer.
- `src/target.rs` - release target resolution used by published binaries.

### Related Docs

- [`../cli-release-interface/README.md`](../cli-release-interface/README.md)
- [`../cli-release-server/README.md`](../cli-release-server/README.md)
- [`../../docs/protocol/cli-releases.md`](../../docs/protocol/cli-releases.md)
