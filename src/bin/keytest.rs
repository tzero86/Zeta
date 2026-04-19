//! Tiny key-event diagnostic tool.
//!
//! Run with: `cargo run --bin keytest`
//! Press any key to see what crossterm reports. Ctrl+C quits.

use std::io::{self, Write};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

fn main() -> io::Result<()> {
    enable_raw_mode()?;

    let stdout = io::stdout();
    let mut out = stdout.lock();

    writeln!(
        out,
        "=== Zeta key diagnostic — press keys to inspect, Ctrl+C to quit ===\r"
    )?;
    writeln!(out, "Try: Ctrl+G, Alt+-, Alt+=, Shift+Left, Shift+Right\r")?;
    writeln!(out, "\r")?;
    out.flush()?;

    loop {
        let ev = event::read()?;
        if let Event::Key(k) = ev {
            if k.kind != KeyEventKind::Press {
                continue;
            }

            writeln!(out, "code={:?}  modifiers={:?}\r", k.code, k.modifiers)?;
            out.flush()?;

            if k.code == KeyCode::Char('c') && k.modifiers == KeyModifiers::CONTROL {
                break;
            }
        }
    }

    disable_raw_mode()?;
    writeln!(io::stdout(), "\nDone.")?;
    Ok(())
}
