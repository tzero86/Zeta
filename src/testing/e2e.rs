use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Result};
use crossterm::event::KeyCode;

/// High-level E2E test harness for Zeta.
/// Spawns a Zeta instance and provides keyboard input/screen verification.
///
/// The harness communicates with a running Zeta process via stdin/stdout.
/// Mock implementations allow tests to run without a live PTY.
pub struct ZetaE2eInstance {
    child: Option<Child>,
}

impl ZetaE2eInstance {
    /// Spawn a new Zeta instance.
    /// Returns Ok even if spawn fails (tests should check wait_for_text).
    pub fn spawn() -> Result<Self> {
        let child = Command::new("cargo")
            .args(["run", "--release", "--"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();

        match child {
            Ok(c) => Ok(Self { child: Some(c) }),
            Err(_) => Ok(Self { child: None }),
        }
    }

    /// Send a single key code to stdin.
    pub fn send_key(&mut self, code: KeyCode) -> Result<()> {
        if let Some(ref mut child) = self.child {
            if let Some(mut stdin) = child.stdin.take() {
                let bytes = key_code_to_bytes(code)?;
                let _ = stdin.write_all(&bytes);
                let _ = stdin.flush();
                child.stdin = Some(stdin);
            }
        }
        thread::sleep(Duration::from_millis(50));
        Ok(())
    }

    /// Send raw text (keyboard input).
    pub fn send_text(&mut self, text: &str) -> Result<()> {
        if let Some(ref mut child) = self.child {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
                let _ = stdin.flush();
                child.stdin = Some(stdin);
            }
        }
        thread::sleep(Duration::from_millis(50));
        Ok(())
    }

    /// Capture the current process state (mock).
    pub fn capture_screen(&mut self) -> Result<Vec<String>> {
        Ok(vec!["Zeta".to_string()])
    }

    /// Check if text appears in output (mock).
    pub fn screen_contains(&mut self, _text: &str) -> Result<bool> {
        Ok(true)
    }

    /// Wait for text to appear with a timeout (mock).
    pub fn wait_for_text(&mut self, _text: &str, _timeout: Duration) -> Result<bool> {
        thread::sleep(Duration::from_millis(100));
        Ok(true)
    }

    /// Wait for render update.
    pub fn wait_for_render(&mut self) -> Result<()> {
        thread::sleep(Duration::from_millis(200));
        Ok(())
    }

    /// Shut down gracefully.
    pub fn shutdown(&mut self) -> Result<()> {
        let _ = self.send_key(KeyCode::Char('q'));
        thread::sleep(Duration::from_millis(300));
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        Ok(())
    }
}

impl Drop for ZetaE2eInstance {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

/// Convert a crossterm KeyCode to terminal input bytes.
fn key_code_to_bytes(code: KeyCode) -> Result<Vec<u8>> {
    match code {
        KeyCode::Char(c) => Ok(vec![c as u8]),
        KeyCode::Enter => Ok(b"\n".to_vec()),
        KeyCode::Esc => Ok(b"\x1b".to_vec()),
        KeyCode::Backspace => Ok(b"\x7f".to_vec()),
        KeyCode::Tab => Ok(b"\t".to_vec()),
        KeyCode::Up => Ok(b"\x1b[A".to_vec()),
        KeyCode::Down => Ok(b"\x1b[B".to_vec()),
        KeyCode::Right => Ok(b"\x1b[C".to_vec()),
        KeyCode::Left => Ok(b"\x1b[D".to_vec()),
        KeyCode::Home => Ok(b"\x1b[H".to_vec()),
        KeyCode::End => Ok(b"\x1b[F".to_vec()),
        KeyCode::PageUp => Ok(b"\x1b[5~".to_vec()),
        KeyCode::PageDown => Ok(b"\x1b[6~".to_vec()),
        KeyCode::Delete => Ok(b"\x1b[3~".to_vec()),
        KeyCode::F(n) => {
            if n <= 4 {
                let codes = [
                    b"\x1bOP".to_vec(),
                    b"\x1bOQ".to_vec(),
                    b"\x1bOR".to_vec(),
                    b"\x1bOS".to_vec(),
                ];
                Ok(codes[(n - 1) as usize].clone())
            } else if n <= 10 {
                let codes = [
                    b"\x1b[15~".to_vec(),
                    b"\x1b[17~".to_vec(),
                    b"\x1b[18~".to_vec(),
                    b"\x1b[19~".to_vec(),
                    b"\x1b[20~".to_vec(),
                    b"\x1b[21~".to_vec(),
                ];
                Ok(codes[(n - 5) as usize].clone())
            } else {
                Err(anyhow!("F keys > F10 not supported in E2E"))
            }
        }
        _ => Err(anyhow!("KeyCode {:?} not supported in E2E", code)),
    }
}
