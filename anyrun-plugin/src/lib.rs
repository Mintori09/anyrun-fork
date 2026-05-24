/*!
The crate for building plugins for Anyrun.

Each plugin needs 4 functions defined, `init`, `info`, `get_matches` and the `handler`. Documentation
on what each of these should be is found in their respective attribute macros.
!*/

pub use anyrun_interface::{self, HandleResult, Match, PluginInfo};
pub use anyrun_macros::{get_matches, handler, info, init};
use std::process::{Child, Command, Stdio};

/// Spawn a detached child process for plugin launch actions.
///
/// On Unix this places the child in a new process group.
/// On all platforms stdio is disconnected from the parent.
pub fn spawn_detached(command: &mut Command) -> std::io::Result<Child> {
    command.stdin(Stdio::null());
    command.stdout(Stdio::null());
    command.stderr(Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    command.spawn()
}

/*
The macro to create a plugin, handles asynchronous execution of getting the matches and the boilerplate
for integrating with `stable_abi`.

# Arguments


* `$type`: The type of the shared data to be provided to various functions.
*/

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::spawn_detached;
    use std::process::Command;

    fn read_pgrp_from_proc(pid: u32) -> i32 {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap();
        let close_paren = stat.rfind(')').unwrap();
        let rest = stat[close_paren + 2..]
            .split_whitespace()
            .collect::<Vec<_>>();
        rest[2].parse::<i32>().unwrap()
    }

    #[test]
    fn detached_child_has_own_process_group() {
        let mut child = spawn_detached(Command::new("sleep").arg("1")).unwrap();
        let pid = child.id();
        let pgrp = read_pgrp_from_proc(pid);
        assert_eq!(pgrp, pid as i32);
        let _ = child.kill();
        let _ = child.wait();
    }

    #[test]
    fn detached_spawn_returns_error_for_missing_binary() {
        let mut command = Command::new("__anyrun_missing_command__");
        let err = spawn_detached(&mut command).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }
}
