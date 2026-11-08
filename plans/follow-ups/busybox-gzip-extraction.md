# BusyBox tar cannot extract the installer archive

- **Severity:** Medium. **Likelihood:** Rare. **Effort:** Small. **Confidence:** High. **Harder later:** No.
- **Kind:** Outside the scope.
- **Added:** 2026-11-08, from PR #8 on the branch
  `conductor/installer-forward-arguments`, user review round 1, outside the
  scope.
- **Location:** `crates/cli-release-server/src/install.rs:176`,
  `crates/cli-release-server/src/install.rs:147`,
  `crates/cli-release-server/src/_tests_/install_tests.rs:62`,
  `crates/cli-release-server/tests/installer_handoff/fixture.rs:108`, and
  `crates/cli-release-server/README.md:169`.
- **Reason:** On 2026-11-08 the user excluded this fix from the installer
  argument change and asked for a separate follow-up item.

- **Terms:**
  - **Generated install script:** The POSIX `sh` script that
    `crates/cli-release-server/src/install.rs` renders. The server returns
    it from `GET /` on install hosts and from `GET /{binary}`.
  - **BusyBox:** One program that holds small versions of many Unix tools,
    such as `sh` and `tar`, as applets. Debian ships a static build in the
    package `busybox-static`.
  - **GNU tar:** The `tar` program of Debian, Ubuntu, and most other Linux
    systems.
- **Background:**
  - Users run the generated install script with
    `curl -fsSL <install URL> | sh`. The script downloads the release
    archive and checks its SHA-256 digest.
  - A release archive is a gzip-compressed tar file. The server serves it as
    `application/gzip`. The default asset name ends in `.tar.gz`.
  - The script lists the archive members with `tar -tzf "$archive"` at
    `install.rs:147`. This command passes `-z`, the gzip option of `tar`.
  - The script extracts the selected binary at `install.rs:176` with this
    command. It does not pass `-z`:

    ```sh
    tar -xOf "$archive" -- "$binary_member" >"$binary_path"
    ```

  - `origin/main` has the same command at the same line. The installer
    argument change of 2026-11-08 did not change it.
  - GNU tar detects gzip compression when it reads an archive file. So the
    command works where `tar` is GNU tar.
  - BusyBox `sh` from `busybox-static` runs its own `tar` applet before it
    searches `PATH`. So the script uses BusyBox tar under that shell, also
    when GNU tar is installed. A system whose only `tar` is BusyBox tar uses
    it too.
  - On 2026-11-08, a user review and an independent reproduction found the
    failure with the Debian 12 package `busybox-static`, version
    `1:1.35.0-4+deb12u1+b1`, which holds BusyBox 1.35. Under BusyBox `sh`,
    the generated script stopped at the extraction step with
    `tar: short read`.
  - This reproduction needs Debian 12 with GNU tar and the package
    `busybox-static`, for example a `debian:12` container after
    `apt-get update && apt-get install -y busybox-static`:

    ```sh
    repro_dir=$(mktemp -d)
    printf 'probe payload\n' > "$repro_dir/example-cli"
    tar -czf "$repro_dir/archive.tar.gz" -C "$repro_dir" example-cli
    busybox tar -xOf "$repro_dir/archive.tar.gz" -- example-cli
    busybox tar -xzOf "$repro_dir/archive.tar.gz" -- example-cli
    rm -rf "$repro_dir"
    ```

    With version `1:1.35.0-4+deb12u1+b1`, the first BusyBox command prints
    `tar: short read`, writes nothing to standard output, and exits with
    status 1. The second prints `probe payload` and exits with status 0.
    GNU tar extracts the same kind of archive without `-z`. Under BusyBox
    `sh`, a `tar` shell function that runs GNU tar,
    `tar() { /usr/bin/tar "$@"; }`, makes the unchanged command succeed.
  - The reproduction archive is smaller than one 512-byte tar block. The
    error text for a full-size release archive was not checked.
  - `install_tests.rs:62` asserts the exact extraction command string.
  - The Linux test `crates/cli-release-server/tests/installer_handoff.rs`
    runs the generated script under `bash`, `dash`, and BusyBox `sh`. At
    `tests/installer_handoff/fixture.rs:108`, its fixture adds a `tar()`
    shell function to the script. The function runs GNU tar with the
    unchanged flags. Without it, this problem stops the BusyBox case before
    the test reaches its checks. A paragraph under "Test Prerequisites" in
    `crates/cli-release-server/README.md` explains the function.
- **Problem:** The extraction command omits `-z`. BusyBox tar 1.35 from
  Debian then does not decompress the archive. With the reproduction
  archive, it prints `tar: short read` and exits with status 1. The script
  then prints `error: failed to extract <binary> from archive`, exits with
  status 1, and installs nothing.
- **Impact if no action:** An install under BusyBox `sh` from
  `busybox-static`, or on a system whose only `tar` is BusyBox tar, fails
  before it installs the binary. The workaround is to run the script with
  `bash`, `dash`, or another shell that finds GNU tar on `PATH`, for
  example `curl -fsSL <install URL> | bash -s -- <arguments>`. The Linux
  test keeps its `tar()` function, so it does not cover BusyBox tar.
- **Root question:** How does the generated install script extract the
  selected binary from a gzip release archive with every `tar` that it can
  meet, including BusyBox tar?
- **Options:**
  - **A.** Add `-z` to the extraction command:
    `tar -xzOf "$archive" -- "$binary_member"`. It then matches the listing
    command and the gzip archive format. Change the expected string at
    `install_tests.rs:62` to the new command. Remove the line of
    `fixture.rs` that adds the `tar()` function, so that the BusyBox case
    of the Linux test runs BusyBox tar. Delete the README paragraph that
    explains the function. Effort: Small.
  - **B.** Decompress the archive once with `gzip -dc` into a temporary
    `.tar` file. Then list and extract that file without `-z`, so that no
    `tar` call needs gzip support. The script must then check for `gzip`.
    That check adds a new failure on a system without `gzip`, and a new
    rejection needs the user's approval. The step also writes a second
    temporary file of the uncompressed size. Effort: Small.
- **Left out:**
  - Select GNU tar by path in the script, as the test does. It fails on a
    system where BusyBox provides the only `tar`.
  - Detect BusyBox tar and stop with a clear error. It adds a new
    rejection, and it fails the goal.
  - Document BusyBox as unsupported and change nothing. It fails the goal.
  - Serve an uncompressed tar file to installers. It breaks the requirement
    that release archives are gzip files.
  - Route both archive reads through one helper that adds `-z`. The script
    has only two reads, and a helper cannot stop a direct `tar` call.
    Option A with its BusyBox test case gives the same protection with
    less code.
- **Wider fix:** None. The script has only two archive reads. After option
  A, both pass `-z`, and the BusyBox case of the Linux test runs both.
- **Previous examples:** None found. The other archive reads in the
  repository pass `-z`: `install.rs:147` and
  `scripts/test-cli-release.sh:157`.
- **Recommendation:** A. One flag aligns the extraction with the listing
  command and the gzip archive format, and the BusyBox test case then
  guards it.
- **Action:** fix after merge, apply A, because the user excluded this fix
  from the installer argument change on 2026-11-08.
