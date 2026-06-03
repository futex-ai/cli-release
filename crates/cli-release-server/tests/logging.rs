use std::io::{BufRead, BufReader};
use std::process::{ChildStderr, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const STARTUP_MESSAGE: &str = "\"message\":\"starting release server\"";
const STARTUP_LOG_TIMEOUT: Duration = Duration::from_secs(5);

#[test]
fn startup_uses_json_logging_from_env() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cli-release-server"))
        .arg("serve")
        .arg("--bind")
        .arg("127.0.0.1:0")
        .env("CLI_RELEASE_LOG_FORMAT", "json")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn cli-release-server");

    let stderr = child.stderr.take().expect("stderr pipe");
    let (stderr_rx, stderr_reader) = read_stderr_lines(stderr);
    let stderr = wait_for_startup_log(&stderr_rx);
    let _ = child.kill();
    child.wait().expect("wait for cli-release-server");
    stderr_reader.join().expect("join stderr reader");

    assert!(stderr.contains(STARTUP_MESSAGE));
    assert!(
        stderr
            .lines()
            .any(|line| line.starts_with('{') && line.contains("\"level\":\"INFO\""))
    );
}

#[test]
fn cli_flag_overrides_json_env_format() {
    let output = Command::new(env!("CARGO_BIN_EXE_cli-release-server"))
        .args(["--log-format", "pretty", "serve", "--bind", "not-an-addr"])
        .env("CLI_RELEASE_LOG_FORMAT", "json")
        .output()
        .expect("run cli-release-server");
    let stderr = String::from_utf8(output.stderr).expect("utf-8 stderr");
    let first_line = stderr
        .lines()
        .find(|line| !line.trim().is_empty())
        .expect("stderr line");

    assert!(!output.status.success());
    assert!(!first_line.starts_with('{'));
}

fn read_stderr_lines(stderr: ChildStderr) -> (mpsc::Receiver<String>, thread::JoinHandle<()>) {
    let (stderr_tx, stderr_rx) = mpsc::channel();
    let stderr_reader = thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            let line = line.expect("read stderr line");
            if stderr_tx.send(line).is_err() {
                break;
            }
        }
    });

    (stderr_rx, stderr_reader)
}

fn wait_for_startup_log(stderr_rx: &mpsc::Receiver<String>) -> String {
    let deadline = Instant::now() + STARTUP_LOG_TIMEOUT;
    let mut stderr = String::new();

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return stderr;
        }
        let line = stderr_rx
            .recv_timeout(remaining)
            .expect("startup log line before timeout");
        stderr.push_str(&line);
        stderr.push('\n');
        if stderr.contains(STARTUP_MESSAGE) {
            return stderr;
        }
    }
}
