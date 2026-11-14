//! Real FPS: runs Intel's PresentMon CLI (path chosen by the user) and streams
//! frame times from its CSV output. Needs administrator rights or membership
//! of the Performance Log Users group.

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;

use si_core::frames::{FrameTimeParser, FrameWindow};

use crate::ProbeError;

/// Event-trace session name, so a stale session can be replaced.
const SESSION_NAME: &str = "SpeedImprover";

/// PresentMon 2.x arguments to capture one process and print CSV to stdout.
pub fn presentmon_args(process_name: &str) -> Vec<String> {
    [
        "--process_name",
        process_name,
        "--output_stdout",
        "--no_console_stats",
        "--stop_existing_session",
        "--session_name",
        SESSION_NAME,
        "--terminate_on_proc_exit",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

/// A running capture. Dropping it stops PresentMon.
pub struct PresentMonSession {
    child: Child,
    frames: Receiver<f64>,
}

impl PresentMonSession {
    pub fn start(exe: &Path, process_name: &str) -> Result<Self, ProbeError> {
        let mut cmd = Command::new(exe);
        cmd.args(presentmon_args(process_name));
        Self::start_command(cmd)
    }

    /// Runs any command that prints PresentMon-style CSV (used by tests).
    pub fn start_command(mut cmd: Command) -> Result<Self, ProbeError> {
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let mut child = cmd.spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ProbeError::Io(std::io::Error::other("no stdout pipe")))?;
        let (tx, frames) = mpsc::channel();
        thread::spawn(move || {
            let mut parser = FrameTimeParser::new();
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if let Some(ms) = parser.feed_line(&line) {
                    if tx.send(ms).is_err() {
                        break;
                    }
                }
            }
        });
        Ok(Self { child, frames })
    }

    /// Moves every frame received so far into `window`; returns how many.
    pub fn drain_into(&self, window: &mut FrameWindow) -> usize {
        self.frames.try_iter().map(|ms| window.push(ms)).count()
    }

    pub fn is_running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }
}

impl Drop for PresentMonSession {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
