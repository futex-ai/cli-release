//! Linux process-argument privacy regression for generated shell installers.

#![cfg(target_os = "linux")]

use std::fs;

use self::{fixture::InstallerFixture, process::RunningInstaller};

#[path = "installer_handoff/fixture.rs"]
mod fixture;
#[path = "installer_handoff/process.rs"]
mod process;

const TOKEN: &str = "synthetic handoff token 'single' \"double\"";
const ARGUMENTS: &[&str] = &["install", "--token", TOKEN, "--", "", "$literal * ?"];
const EXPECTED_ARGV: &[u8] =
    b"install\0--token\0synthetic handoff token 'single' \"double\"\0--\0\0$literal * ?\0";

struct Observation {
    shell: &'static str,
    retained: bool,
    probe_cleared: bool,
    same_pid: bool,
    tmp_removed: bool,
    argv_exact: bool,
    output_exact: bool,
    private_output: bool,
    status: Option<i32>,
}

#[test]
fn generated_installer_replaces_shell_without_retaining_arguments() {
    let fixture = InstallerFixture::new();
    let mut observations = Vec::new();
    for (shell, program, shell_args) in [
        ("Bash", "bash", &[][..]),
        ("Dash", "dash", &[][..]),
        ("BusyBox sh", "busybox", &["sh"][..]),
    ] {
        let (command, paths) = fixture.command(program, shell_args, ARGUMENTS);
        let mut installer = RunningInstaller::start(command, shell);
        let installed_line = installer.next_line();
        let ready_line = installer.next_line();
        let probe_pid: u32 = ready_line
            .parse()
            .expect("probe readiness must contain only its PID");
        let installer_cmdline = fs::read(format!("/proc/{}/cmdline", installer.id()))
            .expect("live installer process arguments should read");
        let probe_cmdline = fs::read(format!("/proc/{probe_pid}/cmdline"))
            .expect("live native probe arguments should read");
        let recorded_argv = fs::read(&paths.arguments).expect("probe argv should read");
        let tmp_removed = fs::read_dir(&paths.tmp_root)
            .expect("runtime directory should read")
            .count()
            == 0;
        let same_pid = installer.id() == probe_pid;
        let (status, extra_lines) = installer.finish();
        let stderr = fs::read(&paths.stderr).expect("installer stderr should read");
        let stdout = format!("{installed_line}\n{ready_line}\n{}", extra_lines.join("\n"));
        let retained = contains_token(&installer_cmdline);
        let observation = Observation {
            shell,
            retained,
            probe_cleared: !probe_cmdline.is_empty() && probe_cmdline.iter().all(|byte| *byte == 0),
            same_pid,
            tmp_removed,
            argv_exact: recorded_argv == EXPECTED_ARGV,
            output_exact: installed_line
                == format!(
                    "installed example-cli 1.2.3 to {}/example-cli",
                    paths.install_dir.display()
                )
                && extra_lines.is_empty()
                && stderr
                    == format!("warning: {} is not on PATH\n", paths.install_dir.display())
                        .as_bytes(),
            private_output: !contains_token(stdout.as_bytes()) && !contains_token(&stderr),
            status: status.code(),
        };
        eprintln!(
            "{shell}: retained_installer_arguments={retained}, probe_arguments_cleared={}, same_pid={same_pid}, tmp_removed_before_release={tmp_removed}, exact_argv={}, exit_status={:?}",
            observation.probe_cleared, observation.argv_exact, observation.status
        );
        observations.push(observation);
    }
    let retained_shells: Vec<_> = observations
        .iter()
        .filter(|observation| observation.retained)
        .map(|observation| observation.shell)
        .collect();
    assert!(
        retained_shells.is_empty(),
        "installer process retained dummy token arguments under: {}",
        retained_shells.join(", ")
    );
    for observation in observations {
        let shell = observation.shell;
        assert!(observation.probe_cleared, "{shell}: probe must clear argv");
        assert!(
            observation.same_pid,
            "{shell}: binary must replace installer"
        );
        assert!(
            observation.tmp_removed,
            "{shell}: cleanup must precede handoff"
        );
        assert!(observation.argv_exact, "{shell}: argv must remain exact");
        assert!(
            observation.output_exact,
            "{shell}: output must remain exact"
        );
        assert!(
            observation.private_output,
            "{shell}: output must omit token"
        );
        assert_eq!(observation.status, Some(37), "{shell}: binary exit status");
    }
}

fn contains_token(bytes: &[u8]) -> bool {
    bytes
        .windows(TOKEN.len())
        .any(|window| window == TOKEN.as_bytes())
}
