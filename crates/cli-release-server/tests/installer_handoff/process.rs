//! Child-process lifetime and pipe handshake for installer integration tests.

use std::{
    io::{BufRead, BufReader, Write},
    os::unix::process::CommandExt,
    process::{Child, Command, ExitStatus, Stdio},
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
    time::Duration,
};

const HANG_GUARD: Duration = Duration::from_secs(30);

/// Keeps the installer and its children bounded during failed assertions.
pub(super) struct RunningInstaller {
    child: Child,
    lines: Receiver<String>,
    reader: Option<JoinHandle<()>>,
    guard_stop: Sender<()>,
    guard: Option<JoinHandle<()>>,
    finished: bool,
}

impl RunningInstaller {
    /// Starts one required shell with pipes and its own process group.
    pub(super) fn start(mut command: Command, shell: &str) -> Self {
        let mut child = command
            .process_group(0)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap_or_else(|error| panic!("required {shell} must start: {error}"));
        let stdout = child.stdout.take().expect("installer stdout pipe");
        let (line_tx, lines) = mpsc::channel();
        let reader = thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if line_tx.send(line).is_err() {
                    break;
                }
            }
        });
        let pid = child.id();
        let (guard_stop, guard_rx) = mpsc::channel();
        let guard = thread::spawn(move || {
            if guard_rx.recv_timeout(HANG_GUARD) == Err(mpsc::RecvTimeoutError::Timeout) {
                kill_group(pid);
            }
        });
        Self {
            child,
            lines,
            reader: Some(reader),
            guard_stop,
            guard: Some(guard),
            finished: false,
        }
    }

    /// Returns the PID of the shell that launched the installer.
    pub(super) fn id(&self) -> u32 {
        self.child.id()
    }

    /// Receives a complete output line without polling or timing assertions.
    pub(super) fn next_line(&self) -> String {
        self.lines
            .recv_timeout(HANG_GUARD)
            .expect("installer must report readiness before the hang guard")
    }

    /// Releases the ready probe and returns the shell's final exit status.
    pub(super) fn finish(&mut self) -> (ExitStatus, Vec<String>) {
        self.child
            .stdin
            .take()
            .expect("probe release pipe")
            .write_all(b"x")
            .expect("probe release should write");
        let status = self.child.wait().expect("installer should exit");
        self.finished = true;
        let _ = self.guard_stop.send(());
        self.reader
            .take()
            .expect("stdout reader")
            .join()
            .expect("stdout reader should finish");
        (status, self.lines.try_iter().collect())
    }
}

impl Drop for RunningInstaller {
    fn drop(&mut self) {
        let _ = self.child.stdin.take();
        if !self.finished {
            kill_group(self.child.id());
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        let _ = self.guard_stop.send(());
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        if let Some(guard) = self.guard.take() {
            let _ = guard.join();
        }
    }
}

fn kill_group(pid: u32) {
    let _ = Command::new("kill")
        .args(["-KILL", "--", &format!("-{pid}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}
