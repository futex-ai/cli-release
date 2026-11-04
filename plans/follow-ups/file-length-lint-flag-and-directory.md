# File-length lint ignores its flag and directory

- **Severity:** Low. **Effort:** Small. **Confidence:** High. **Harder later:** No.
- **Kind:** Outside the scope.
- **Added:** 2026-11-04, from the branch `conductor/agents-md-cli-release`.
- **Location:** `xtask/src/main.rs:208`, `xtask/src/main.rs:27`,
  `xtask/README.md:8`.
- **Reason:** The fix changes `xtask` code and its README, and the branch
  that found the problem changes only Markdown files.

- **Terms:**
  - **File-length lint:** The command `cargo xtask rust-file-length-lint`,
    which fails when a Rust file under `crates/` or `xtask/` has more than
    300 lines.
- **Background:**
  - `cargo xtask check` runs the lint after the format check, the workspace
    tests, and Clippy.
  - CI runs `cargo xtask check` on each pull request and on each push to
    `main`.
  - The lint accepts one optional flag, `--all`. Its usage text is
    `usage: cargo xtask rust-file-length-lint [--all]`.
  - The lint reads `crates/` and `xtask/` relative to the current
    directory. It skips a directory that does not exist.
  - The top-level usage text of `cargo xtask` is
    `usage: cargo xtask <check|review>`. It does not name the lint.
  - Before the branch `conductor/agents-md-cli-release`, `AGENTS.md` said
    that the lint checked the changed files and that `--all` checked every
    file.
- **Problem:** The lint ignores the flag: each run reads every `.rs` file
  under `crates/` and `xtask/`. A run from a subdirectory checks no file and
  passes. On 2026-11-02, a 301-line file under `crates/` gave exit 1 from
  the root and exit 0 from `crates/`.
- **Impact if no action:** A run of `cargo xtask check` from a subdirectory
  passes the lint without a check. A reader of the usage text expects a
  smaller audit without `--all`.
- **Root question:** How does the file-length lint find the Rust files of
  the repository, and how does its interface show what it checks?
- **Options:**
  - **A.** Keep the lint behaviour. Say in `xtask/README.md` and in the
    usage text that the lint always checks every file. Say also that
    `--all` has no effect and that the lint runs from the repository root.
    Effort: Small.
  - **B.** Resolve `crates/` and `xtask/` from the repository root, for
    example from the `CARGO_MANIFEST_DIR` of `xtask`. Remove the `--all`
    flag and its use in `check`. Add the lint to the top-level usage text.
    Effort: Small.
  - **C.** Apply B, but keep `--all`. Make a run without `--all` check only
    the Rust files that differ from `origin/main`. Effort: Medium.
- **Left out:** Fail when `crates/` is missing. Option B removes the wrong
  case, so a guard adds nothing.
- **Wider fix:** B. The lint then finds the same files from any directory,
  so no working directory can skip the check.
- **Previous examples:** `futex-ai/ai` recorded the same flag problem in
  `plans/follow-ups/file-length-lint-all-flag.md` on 2026-11-01.
- **Recommendation:** B. The lint then checks the same files from any
  directory, and no flag without an effect misleads a reader.
- **Action:** ask the user, because option B removes a command-line option,
  and a command interface on `main` is functionality.
