# `cargo xtask review` uses an unavailable model

- **Severity:** Low. **Effort:** Small. **Confidence:** High. **Harder later:** No.
- **Kind:** Outside the scope.
- **Added:** 2026-11-04, from the branch `conductor/agents-md-cli-release`.
- **Location:** `xtask/src/main.rs:134`, `xtask/src/main.rs:143`,
  `xtask/README.md:9`, `README.md:46`.
- **Reason:** The fix changes `xtask` code, and the branch that found the
  problem changes only Markdown files.

- **Terms:**
  - **`cargo xtask review`:** An `xtask` command that sends a summary of the
    local diff to `codex exec`, with the fixed model `gpt-5.5` at effort
    `xhigh`, in a read-only sandbox.
  - **Conductor cloud workspace:** A cloud machine on which Conductor runs
    coding-agent sessions for one branch of the repository.
  - **Model provider:** The `model_provider` setting of the Codex CLI,
    which names the API service that runs the models of `codex exec`.
- **Background:**
  - `xtask/src/main.rs` runs this command. The model id is a fixed string
    in the code.

    ```sh
    codex --ask-for-approval never exec --ephemeral --ignore-rules --model gpt-5.5 --config model_reasoning_effort="xhigh" --sandbox read-only --skip-git-repo-check --cd . -
    ```

  - On 2026-11-04 the user chose the `implementation-review` skill as the
    review step. That skill takes its reviewer from the `models` skill. The
    `models` skill names each model in one table. On the same day, the user
    chose to keep `cargo xtask review` as an optional manual review.
    `README.md` now says so.
  - In a Conductor cloud workspace, the Codex CLI 0.164.0 uses the model
    provider `cliproxyapi`. On 2026-11-04, `codex exec --model gpt-5.5`
    there failed with this error:

    ```text
    {"error":{"code":"model_not_found","message":"unknown provider for model gpt-5.5","param":"model","type":"invalid_request_error"}}
    ```

    An agent in a second cloud workspace got the same error twice from
    `cargo xtask review` on the same day.
  - `conductor model` lists `gpt-5.5` among the Codex models. So the error
    depends on the model provider of the machine.
- **Problem:** `cargo xtask review` fails at once on a machine whose model
  provider has no `gpt-5.5`. The model id is fixed in the code. A caller
  cannot choose another model.
- **Impact if no action:** The optional manual review fails in Conductor
  cloud workspaces. The review of the `implementation-review` skill still
  works, because it runs as a Conductor session with the model of the
  `models` skill.
- **Root question:** How does `cargo xtask review` use a model that the
  machine has?
- **Options:**
  - **A.** Change the fixed id to a model that the model provider of each
    machine offers today, for example `gpt-6.1-sol`. The new id can go
    stale in the same way. Effort: Small.
  - **B.** Remove the fixed model. Let the caller set a model, for example
    with an environment variable. When the caller sets none, use the
    default model of the local Codex configuration. Update
    `xtask/README.md`. Effort: Small.
  - **C.** Remove `cargo xtask review` and its README lines, because the
    `implementation-review` skill is the review step. This removes a
    command on `main`. The user chose on 2026-11-04 to keep the command. It
    therefore needs the user's approval. Effort: Small.
  - **D.** Keep `gpt-5.5` as the default. Let the caller set another model
    with an environment variable. A cloud workspace must then set the
    variable, and the default can still go stale. Effort: Small.
- **Left out:** Keep the command as it is. It then fails on each machine
  whose model provider has no `gpt-5.5`.
- **Wider fix:** B. The command then names no model id that can go stale.
- **Previous examples:** None found in this repository.
- **Recommendation:** B. It keeps the manual review that the user chose,
  and it works with the model of each machine. Option D is the smallest
  change that the agent can make without the user.
- **Action:** ask the user, because option B also changes the model on
  machines where `gpt-5.5` works today. That overrides behaviour on `main`,
  which needs the user's approval under the `git-commit` skill.
