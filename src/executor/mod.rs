pub mod native;
pub mod system;
pub mod user;

use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::process::ExitStatusExt;
use std::process::Command;
use std::sync::atomic::{AtomicI32, Ordering};
use nix::sys::signal::{self, SigHandler, Signal};
use nix::unistd::Pid;

static RUNNING_CHILD_PID: AtomicI32 = AtomicI32::new(0);

extern "C" fn forward_signal_handler(sig: libc::c_int) {
    let pid = RUNNING_CHILD_PID.load(Ordering::SeqCst);
    if pid > 0 {
        if let Ok(signal_enum) = Signal::try_from(sig) {
            let _ = signal::kill(Pid::from_raw(pid), signal_enum);
        }
    }
}

struct SignalGuard;

impl SignalGuard {
    fn setup() -> Self {
        unsafe {
            let _ = signal::signal(Signal::SIGINT, SigHandler::Handler(forward_signal_handler));
            let _ = signal::signal(Signal::SIGTERM, SigHandler::Handler(forward_signal_handler));
        }
        SignalGuard
    }
}

impl Drop for SignalGuard {
    fn drop(&mut self) {
        RUNNING_CHILD_PID.store(0, Ordering::SeqCst);
        unsafe {
            let _ = signal::signal(Signal::SIGINT, SigHandler::SigDfl);
            let _ = signal::signal(Signal::SIGTERM, SigHandler::SigDfl);
        }
    }
}

fn extract_process_exit_code(status: std::process::ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        code
    } else if let Some(sig) = status.signal() {
        128 + sig
    } else {
        1
    }
}

pub fn spawn_and_wait(mut command: Command) -> io::Result<i32> {
    let _guard = SignalGuard::setup();
    let mut child = command.spawn()?;
    RUNNING_CHILD_PID.store(child.id() as i32, Ordering::SeqCst);

    let status = child.wait()?;
    Ok(extract_process_exit_code(status))
}

pub fn spawn_and_wait_guest(mut command: Command, is_interactive: bool) -> io::Result<i32> {
    if is_interactive {
        return spawn_and_wait(command);
    }

    command.stdout(std::process::Stdio::piped());
    let _guard = SignalGuard::setup();
    let mut child = command.spawn()?;
    RUNNING_CHILD_PID.store(child.id() as i32, Ordering::SeqCst);

    let mut captured_exit_code: Option<i32> = None;

    if let Some(stdout) = child.stdout.take() {
        let mut reader = BufReader::new(stdout);
        let mut out = io::stdout();
        let mut line_buf = Vec::new();
        let exit_tag = b"[qemulbench_exit_code=";

        loop {
            line_buf.clear();
            match reader.read_until(b'\n', &mut line_buf) {
                Ok(0) => break,
                Ok(_) => {
                    if let Some(pos) = line_buf.windows(exit_tag.len()).position(|w| w == exit_tag) {
                        let rem = &line_buf[pos + exit_tag.len()..];
                        if let Some(end_pos) = rem.iter().position(|&b| b == b']') {
                            if let Ok(code_str) = std::str::from_utf8(&rem[..end_pos]) {
                                if let Ok(code) = code_str.trim().parse::<i32>() {
                                    captured_exit_code = Some(code);
                                }
                            }
                        }
                    } else {
                        let _ = out.write_all(&line_buf);
                        let _ = out.flush();
                    }
                }
                Err(_) => break,
            }
        }
    }

    let status = child.wait()?;
    if let Some(guest_code) = captured_exit_code {
        Ok(guest_code)
    } else {
        Ok(extract_process_exit_code(status))
    }
}
