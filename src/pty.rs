//! Platform PTY abstraction.
//!
//! On Windows we use the `conpty` crate (direct ConPTY API) because
//! `portable-pty`'s `try_clone_reader()` does not reliably deliver output
//! on Windows ConPTY.  On Unix we keep `portable-pty` which works fine.

use std::io::{self, Read, Write};
use std::path::Path;
#[cfg(not(windows))]
use std::sync::{Arc, Mutex};

/// A running pseudo-terminal session.
pub struct PtySession {
    inner: PlatformPty,
}

impl PtySession {
    /// Spawn a shell inside a new PTY of the given size.
    pub fn spawn(cwd: &Path, cols: u16, rows: u16) -> io::Result<Self> {
        let inner = PlatformPty::spawn(cwd, cols, rows)?;
        Ok(Self { inner })
    }

    /// Take the output reader (moves ownership — call once).
    pub fn take_reader(&mut self) -> io::Result<Box<dyn Read + Send>> {
        self.inner.take_reader()
    }

    /// Take the input writer (moves ownership — call once).
    pub fn take_writer(&mut self) -> io::Result<Box<dyn Write + Send>> {
        self.inner.take_writer()
    }

    /// Resize the PTY.
    pub fn resize(&mut self, cols: u16, rows: u16) -> io::Result<()> {
        self.inner.resize(cols, rows)
    }

    /// Return a closure that blocks until the child process exits.
    /// The closure is `Send` so it can run on a dedicated watcher thread.
    pub fn exit_waiter(&self) -> io::Result<Box<dyn FnOnce() + Send>> {
        self.inner.exit_waiter()
    }
}

// ---------------------------------------------------------------------------
// Windows implementation — conpty
// ---------------------------------------------------------------------------
#[cfg(windows)]
struct PlatformPty {
    proc: conpty::Process,
    reader_taken: bool,
    writer_taken: bool,
}

#[cfg(windows)]
fn which_shell() -> String {
    // 1. pwsh.exe (PowerShell 7+) — modern, works well with ConPTY
    if let Ok(output) = std::process::Command::new("where").arg("pwsh.exe").output() {
        if output.status.success() {
            return "pwsh.exe".to_string();
        }
    }
    // 2. COMSPEC (usually cmd.exe) — always available
    std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
}

#[cfg(windows)]
impl PlatformPty {
    fn spawn(cwd: &Path, cols: u16, rows: u16) -> io::Result<Self> {
        use std::process::Command;

        let shell = which_shell();
        let mut cmd = Command::new(&shell);
        cmd.current_dir(cwd);

        // conpty's execProc calls `command.get_envs()` which only returns
        // explicitly-set vars and passes them as a non-NULL lpEnvironment to
        // CreateProcess. A non-NULL block replaces the parent env entirely, so
        // we must seed the Command with the full parent environment first.
        for (k, v) in std::env::vars_os() {
            cmd.env(k, v);
        }
        cmd.env("TERM", "xterm-256color");
        cmd.env("ZETA_TERMINAL", "1");

        let proc = conpty::ProcessOptions::default()
            .set_console_size(Some((cols as i16, rows as i16)))
            .spawn(cmd)
            .map_err(|e| io::Error::other(e.to_string()))?;

        Ok(Self {
            proc,
            reader_taken: false,
            writer_taken: false,
        })
    }

    fn take_reader(&mut self) -> io::Result<Box<dyn Read + Send>> {
        if self.reader_taken {
            return Err(io::Error::other("reader already taken"));
        }
        self.reader_taken = true;
        let pipe = self
            .proc
            .output()
            .map_err(|e| io::Error::other(e.to_string()))?;
        Ok(Box::new(pipe))
    }

    fn take_writer(&mut self) -> io::Result<Box<dyn Write + Send>> {
        if self.writer_taken {
            return Err(io::Error::other("writer already taken"));
        }
        self.writer_taken = true;
        let pipe = self
            .proc
            .input()
            .map_err(|e| io::Error::other(e.to_string()))?;
        Ok(Box::new(pipe))
    }

    fn resize(&mut self, cols: u16, rows: u16) -> io::Result<()> {
        self.proc
            .resize(cols as i16, rows as i16)
            .map_err(|e| io::Error::other(e.to_string()))
    }

    fn exit_waiter(&self) -> io::Result<Box<dyn FnOnce() + Send>> {
        let pid = self.proc.pid();
        Ok(Box::new(move || {
            // Wait for the child process to exit using Win32 API.
            const PROCESS_SYNCHRONIZE: u32 = 0x0010_0000;
            const INFINITE: u32 = 0xFFFF_FFFF;
            extern "system" {
                fn OpenProcess(access: u32, inherit: i32, pid: u32) -> isize;
                fn WaitForSingleObject(handle: isize, ms: u32) -> u32;
                fn CloseHandle(handle: isize) -> i32;
            }
            unsafe {
                let h = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
                if h != 0 {
                    WaitForSingleObject(h, INFINITE);
                    CloseHandle(h);
                }
            }
        }))
    }
}

// ---------------------------------------------------------------------------
// Unix implementation — portable-pty
// ---------------------------------------------------------------------------
#[cfg(not(windows))]
struct PlatformPty {
    master: Option<Box<dyn portable_pty::MasterPty + Send>>,
    _child: Option<Arc<Mutex<Box<dyn portable_pty::Child + Send>>>>,
}

#[cfg(not(windows))]
impl PlatformPty {
    fn spawn(cwd: &Path, cols: u16, rows: u16) -> io::Result<Self> {
        use portable_pty::{native_pty_system, CommandBuilder, PtySize};

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: if rows == 0 { 24 } else { rows },
                cols: if cols == 0 { 80 } else { cols },
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| io::Error::other(e.to_string()))?;

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let mut cmd = CommandBuilder::new(&shell);
        cmd.cwd(cwd);
        cmd.env("TERM", "xterm-256color");
        cmd.env("ZETA_TERMINAL", "1");

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| io::Error::other(e.to_string()))?;

        // Drop the slave so the child is the sole console owner.
        drop(pair.slave);

        Ok(Self {
            master: Some(pair.master),
            _child: Some(Arc::new(Mutex::new(child))),
        })
    }

    fn take_reader(&mut self) -> io::Result<Box<dyn Read + Send>> {
        let master = self
            .master
            .as_ref()
            .ok_or_else(|| io::Error::other("master already consumed"))?;
        master
            .try_clone_reader()
            .map_err(|e| io::Error::other(e.to_string()))
    }

    fn take_writer(&mut self) -> io::Result<Box<dyn Write + Send>> {
        let master = self
            .master
            .as_mut()
            .ok_or_else(|| io::Error::other("master already consumed"))?;
        master
            .take_writer()
            .map_err(|e| io::Error::other(e.to_string()))
    }

    fn resize(&mut self, cols: u16, rows: u16) -> io::Result<()> {
        let master = self
            .master
            .as_ref()
            .ok_or_else(|| io::Error::other("no master"))?;
        master
            .resize(portable_pty::PtySize {
                rows: if rows == 0 { 24 } else { rows },
                cols: if cols == 0 { 80 } else { cols },
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| io::Error::other(e.to_string()))
    }

    fn exit_waiter(&self) -> io::Result<Box<dyn FnOnce() + Send>> {
        let child_arc_clone = self
            ._child
            .as_ref()
            .ok_or_else(|| io::Error::other("no child process"))?
            .clone();

        Ok(Box::new(move || {
            let _ = child_arc_clone.lock().unwrap().wait();
        }))
    }
}
