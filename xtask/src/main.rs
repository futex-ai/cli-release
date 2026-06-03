//! Developer task runner for the cli-release workspace.

#![warn(unreachable_pub)]

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};

const RUST_FILE_LIMIT: usize = 300;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        return Err("usage: cargo xtask <check|review>".to_owned());
    };
    let rest = args.collect::<Vec<_>>();
    if command != "rust-file-length-lint" && !rest.is_empty() {
        return Err("xtask commands do not accept extra arguments".to_owned());
    }
    match command.as_str() {
        "check" => check(),
        "review" => review(),
        "rust-file-length-lint" => rust_file_length_lint(&rest),
        _ => Err(format!("unknown xtask command `{command}`")),
    }
}

fn check() -> Result<(), String> {
    run_command("cargo", ["fmt", "--all", "--", "--check"])?;
    run_command("cargo", ["test", "--workspace"])?;
    run_command(
        "cargo",
        [
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    rust_file_length_lint(&["--all".to_owned()])?;
    Ok(())
}

fn review() -> Result<(), String> {
    let context = review_context()?;
    if context.trim().is_empty() {
        println!("No changes to review relative to origin/main or in the working tree.");
        return Ok(());
    }
    run_codex_review(&review_prompt(&context))
}

fn review_context() -> Result<String, String> {
    let branch_stat = command_output("git", ["diff", "--stat", "origin/main...HEAD"])?;
    let staged_stat = command_output("git", ["diff", "--stat", "--cached"])?;
    let unstaged_stat = command_output("git", ["diff", "--stat"])?;
    let untracked = command_output("git", ["ls-files", "--others", "--exclude-standard"])?;
    let sections = [
        ("branch diff stat", branch_stat),
        ("staged diff stat", staged_stat),
        ("unstaged diff stat", unstaged_stat),
        ("untracked files", bounded_untracked_files(&untracked)),
    ];
    let mut context = String::new();
    for (label, value) in sections {
        if value.trim().is_empty() {
            continue;
        }
        context.push_str(label);
        context.push('\n');
        context.push_str(value.trim());
        context.push_str("\n\n");
    }
    Ok(context)
}

fn bounded_untracked_files(files: &str) -> String {
    const MAX_UNTRACKED_FILES: usize = 200;
    let paths = files.lines().collect::<Vec<_>>();
    let mut output = paths
        .iter()
        .take(MAX_UNTRACKED_FILES)
        .copied()
        .collect::<Vec<_>>()
        .join("\n");
    let omitted = paths.len().saturating_sub(MAX_UNTRACKED_FILES);
    if omitted > 0 {
        output.push_str(&format!(
            "\n... {omitted} additional untracked files omitted"
        ));
    }
    output
}

fn review_prompt(context: &str) -> String {
    format!(
        r#"You are Codex, an AI code reviewer performing a read-only review of this local change set.

Review scope:
- Base ref: origin/main.
- Include committed branch changes, staged changes, unstaged changes, and untracked files.
- The repository is available at the current working directory.

Local change summary:
{context}

Instructions:
- Inspect the changed files and nearby dependencies before reporting findings.
- Focus on bugs, behavioral regressions, missing tests, stale docs, security, and operational risks.
- Do not make edits.
- If there are no material issues, say so clearly and mention any residual test or review risk.
- Report findings first, ordered by severity, with file and line references when available.
"#
    )
}

fn run_codex_review(prompt: &str) -> Result<(), String> {
    let command_text = "codex --ask-for-approval never exec --ephemeral --ignore-rules --model gpt-5.5 --config model_reasoning_effort=\"xhigh\" --sandbox read-only --skip-git-repo-check --cd . -";
    let mut child = Command::new("codex")
        .args([
            "--ask-for-approval",
            "never",
            "exec",
            "--ephemeral",
            "--ignore-rules",
            "--model",
            "gpt-5.5",
            "--config",
            "model_reasoning_effort=\"xhigh\"",
            "--sandbox",
            "read-only",
            "--skip-git-repo-check",
            "--cd",
            ".",
            "-",
        ])
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|source| format!("failed to start `{command_text}`: {source}"))?;
    let write_result = {
        let Some(mut stdin) = child.stdin.take() else {
            return Err(format!("failed to open stdin for `{command_text}`"));
        };
        stdin.write_all(prompt.as_bytes())
    };
    if let Err(source) = write_result
        && source.kind() != ErrorKind::BrokenPipe
    {
        return Err(format!(
            "failed to write prompt to `{command_text}`: {source}"
        ));
    }
    let status = child
        .wait()
        .map_err(|source| format!("failed to wait for `{command_text}`: {source}"))?;
    if status.success() {
        return Ok(());
    }
    Err(format!("`{command_text}` exited with {status}"))
}

fn run_command<I, S>(program: &str, args: I) -> Result<(), String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|source| format!("failed to run `{program}`: {source}"))?;
    if status.success() {
        return Ok(());
    }
    Err(format!("`{program}` exited with {status}"))
}

fn command_output<I, S>(program: &str, args: I) -> Result<String, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|source| format!("failed to run `{program}`: {source}"))?;
    if !output.status.success() {
        return Err(format!("`{program}` exited with {}", output.status));
    }
    String::from_utf8(output.stdout).map_err(|source| format!("stdout was not utf-8: {source}"))
}

fn rust_file_length_lint(args: &[String]) -> Result<(), String> {
    if !args.is_empty() && (args.len() != 1 || args[0] != "--all") {
        return Err("usage: cargo xtask rust-file-length-lint [--all]".to_owned());
    }
    let mut violations = Vec::new();
    for root in ["crates", "xtask"] {
        collect_rust_file_violations(Path::new(root), &mut violations)?;
    }
    if violations.is_empty() {
        println!("rust_file_length_lint=ok");
        return Ok(());
    }
    for violation in &violations {
        eprintln!("{violation}");
    }
    Err(format!(
        "{} Rust file(s) exceeded {RUST_FILE_LIMIT} lines",
        violations.len()
    ))
}

fn collect_rust_file_violations(path: &Path, violations: &mut Vec<String>) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    for entry in
        fs::read_dir(path).map_err(|source| format!("read {}: {source}", path.display()))?
    {
        let entry = entry.map_err(|source| format!("read {} entry: {source}", path.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_rust_file_violations(&path, violations)?;
            continue;
        }
        if path.extension().and_then(OsStr::to_str) != Some("rs") {
            continue;
        }
        let contents = fs::read_to_string(&path)
            .map_err(|source| format!("read {}: {source}", path.display()))?;
        let line_count = contents.lines().count();
        if line_count > RUST_FILE_LIMIT {
            violations.push(format!(
                "{} has {line_count} lines; limit is {RUST_FILE_LIMIT}",
                path.display()
            ));
        }
    }
    Ok(())
}
