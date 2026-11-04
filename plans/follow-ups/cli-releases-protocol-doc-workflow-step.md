# Protocol doc holds workflow step and code names

- **Severity:** Low. **Effort:** Small. **Confidence:** High. **Harder later:** No.
- **Kind:** Outside the scope.
- **Added:** 2026-11-04, from the branch `conductor/agents-md-cli-release`.
- **Location:** `docs/protocol/cli-releases.md:201`, and the code names at
  `docs/protocol/cli-releases.md:24`, `:34`, `:36`, `:158`, `:159`, `:161`,
  and `:162`.
- **Reason:** The doc names code, so the user's rule of 2026-11-04 keeps its
  fix out of the agent-rules migration.

- **Terms:**
  - **Protocol doc:** A spec file under `docs/protocol/` that holds the
    requirements and the decisions in force for one area.
  - **`cargo xtask review`:** An `xtask` command that sends a summary of the
    local diff to a read-only Codex reviewer, `gpt-5.5` at effort `xhigh`,
    with its own prompt.
  - **Agent-rules migration:** A change that reduces `AGENTS.md` to the
    rules of its repository and leaves general rules to the shared skills of
    `futex-ai/skills`.
- **Background:**
  - `docs/protocol/cli-releases.md` has 202 lines. Its last section,
    "Validation", lists the areas that need focused tests. It ends with this
    sentence at lines 201 and 202: "Before completion, run relevant package
    tests, `cargo fmt --all -- --check`, clippy, `cargo xtask check`, then
    `cargo xtask review`."
  - On 2026-11-04 the user chose the `implementation-review` skill of
    `futex-ai/skills` as the review step. That skill runs the review after
    the push, with its own prompt. `AGENTS.md` names `cargo xtask check` as
    the complete gate. `README.md` no longer names `cargo xtask review` as a
    completion step.
  - The `documentation` skill says that a protocol doc holds requirements:
    observable behaviour at a boundary. It does not name source files,
    functions, hooks, or modules. When a change touches a protocol doc that
    still names code, the change moves that content to its home in the same
    change.
  - On 2026-11-04 the user set a rule for agent-rules migrations. Such a
    migration fixes a protocol doc only when the doc has fewer than 250
    lines and names no code. Otherwise a follow-up item records the fix.
  - The doc names the Rust types `ReleaseAssetInfo` and `ReleaseIndex`, the
    traits `ReleaseClient` and `BinaryInstaller`, and the methods
    `Updater::status` and `Updater::upgrade`.
  - The crate READMEs hold the entry points under `### Key Code`.
    `crates/cli-release-interface/README.md` names `ReleaseAssetInfo` and
    `ReleaseIndex`. `crates/cli-updater/README.md` names `ReleaseClient` and
    `BinaryInstaller`. Its Quick Start example calls `status` and `upgrade`
    on an `Updater`.
- **Problem:** The sentence is a workflow step, not a requirement. It names
  a review command that the rules no longer use. The code names describe
  the implementation, not the boundary.
- **Impact if no action:** An agent that reads the doc runs
  `cargo xtask review` as an extra completion step, with an older model and
  its own prompt. A review of the doc against the `documentation` skill
  reports the same problems again.
- **Root question:** How does the CLI releases protocol doc hold only the
  requirements and the decisions in force?
- **Options:**
  - **A.** Delete only the workflow sentence. Keep the code names. The
    `documentation` skill then still finds code names in a doc that the
    change touched. Effort: Small.
  - **B.** Delete the workflow sentence. Restate the metadata and updater
    requirements without the type, trait, and method names. Keep the JSON
    field names, the routes, and the environment variables, because they
    are the boundary. The crate READMEs keep the code names. Effort: Small.
  - **C.** Apply B. Also delete the test list of "Validation", because the
    tests hold the cases. This deletes doc text that no rule replaced. It
    therefore needs the user's approval. Effort: Small.
- **Left out:**
  - Move the sentence to `AGENTS.md`. `AGENTS.md` already names the
    complete gate. The `implementation-review` skill holds the review step.
  - Keep the doc as it is. The doc then contradicts the review rule in
    force.
- **Wider fix:** None. The `documentation` skill already keeps workflow
  steps and code names out of protocol docs.
- **Previous examples:** None found in this repository.
- **Recommendation:** B. It removes the workflow sentence and the code names
  in one small change, and the crate READMEs already hold the names.
- **Action:** fix after merge, apply B, because the doc names code and the
  user's rule of 2026-11-04 defers such fixes.
