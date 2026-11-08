# CLI Installers

This document defines the contract for the install scripts that the release
server in [CLI Releases](./cli-releases.md) renders.

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
- With script arguments, the installed binary replaces the script process after
  the install. The binary runs once, in the process that ran the script.
- The binary receives exactly the script arguments: the same number, in the
  same order, with the same bytes.
- The caller gets the exit status of the binary.
- The script removes its temporary directory before the binary starts. When the
  script does not start the binary, it removes the directory as it exits.
- No installer process waits for the binary or keeps the script arguments while
  the binary runs. Running the binary as a child process was rejected, because
  the waiting script kept the arguments, such as a token, in the process list
  until the binary exited.
- The script and the server must not log script argument values or put them in
  error messages, because the values can carry secrets such as tokens.
- The installed binary owns its own arguments and output. The binary decides
  whether its arguments stay visible to other processes.
- The rule applies to every configured binary, with no setting and no
  product-specific default.
