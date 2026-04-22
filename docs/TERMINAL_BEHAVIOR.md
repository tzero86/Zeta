# Zeta Integrated Terminal Behavior Guide

## Overview

The Zeta file manager includes an embedded terminal emulator that runs your default shell directly within the application. Instead of dropping out to an external terminal, you can interact with a shell in a dedicated panel, toggle it on and off, and even expand it to fullscreen when needed.

### Key Characteristics

- **Lightweight PTY integration**: Runs your configured shell (`$SHELL` on Unix/macOS, `cmd.exe` or `PowerShell` on Windows) in a pseudo-terminal
- **Non-blocking I/O**: Terminal input/output operates on dedicated worker threads, ensuring the file manager UI remains responsive
- **Direct key passthrough**: Most keys are sent directly to the shell without interpretation
- **ANSI rendering**: Colors, cursor positioning, and other ANSI escape sequences are rendered correctly
- **Focus-aware keybindings**: When the terminal has focus, app-level shortcuts still work (F2 to close, Ctrl+\ to toggle, F11 for fullscreen)

---

## Keyboard Behavior

### Key Routing Summary

When the terminal is open and focused, keyboard input is routed to the shell except for three app-level overrides:

| Key(s) | Behavior | Direction |
|--------|----------|-----------|
| **F2** | Toggle terminal panel on/off | App-level override |
| **Ctrl+\** | Toggle terminal panel on/off (alternate) | App-level override |
| **F11** | Toggle fullscreen mode | App-level override |
| **All other keys** | Pass through to shell | Shell passthrough |

### App-Level Overrides (Not Sent to Shell)

These keybindings are always intercepted by Zeta and do not reach the shell:

1. **F2** — Closes the terminal panel and returns focus to file management
2. **Ctrl+\** — Alternative binding to close the terminal (useful if F2 is consumed by your terminal or shell)
3. **F11** — Toggles fullscreen mode for the terminal, expanding it to fill the viewport

### Shell Passthrough Keys

All other keys are converted to appropriate terminal control sequences and sent to the shell:

#### Printable Characters
- `a–z`, `A–Z`, `0–9`, space, punctuation: sent as UTF-8 bytes
- Example: typing `ls` sends three bytes: `0x6c 0x73` (ASCII 108, 115)

#### Control Characters (Ctrl+<key>)
- **Ctrl+A through Ctrl+Z**: mapped to ASCII 1–26
  - Ctrl+A = `0x01` (SOH, start of heading)
  - Ctrl+Z = `0x1a` (SUB, substitute)
- **Special control combinations**:
  - Ctrl+[ = `0x1b` (ESC, escape)
  - Ctrl+\ = `0x1c` (FS, file separator) *before the app-level override is checked*
  - Ctrl+] = `0x1d` (GS, group separator)
  - Ctrl+^ = `0x1e` (RS, record separator)
  - Ctrl+_ = `0x1f` (US, unit separator)

#### Special Keys

| Key | Bytes Sent | VT100 Name |
|-----|-----------|-----------|
| **Enter** | Platform-dependent (see below) | Carriage Return |
| **Backspace** | `0x7f` (DEL) | Delete character |
| **Tab** | `0x09` (TAB) | Horizontal tabulation |
| **Escape** | `0x1b` (ESC) | Escape |
| **Up arrow** | `0x1b 0x5b 0x41` (ESC [ A) | ANSI cursor up |
| **Down arrow** | `0x1b 0x5b 0x42` (ESC [ B) | ANSI cursor down |
| **Right arrow** | `0x1b 0x5b 0x43` (ESC [ C) | ANSI cursor right |
| **Left arrow** | `0x1b 0x5b 0x44` (ESC [ D) | ANSI cursor left |

#### Platform-Specific Line Endings

- **Windows (ConPTY)**: Enter sends `0x0d 0x0a` (CRLF — carriage return + line feed)
- **Unix/Linux/macOS**: Enter sends `0x0d` (CR — carriage return only)

These differences reflect the platform's native line ending conventions.

### Unsupported Keys

The following keys are **not** passed to the shell and return no action:

- F1, F3, F4, F5, F6, F7, F8, F9, F10 (F1, F3–F10 are used by Zeta)
- Home, End, Page Up, Page Down (no standard ANSI sequences)
- Alt+<key> combinations (modifiers other than Ctrl are typically not transmitted)
- Insert, Delete, Print Screen

These limitations reflect the terminal emulator's current scope and can be extended in future releases.

---

## Shell Configuration for Autocompletion

### Enabling Tab Completion

Tab completion (pressing **Tab** for filename/command suggestions) depends on:

1. **Shell type**: Must be **bash**, **zsh**, or **PowerShell** (not `sh`)
2. **Completion packages**: Must be installed and configured
3. **Terminal width**: Menu rendering requires sufficient horizontal space

#### Bash

Bash completion requires the `bash-completion` package:

```bash
# Install bash-completion (Ubuntu/Debian)
sudo apt-get install bash-completion

# Install bash-completion (macOS via Homebrew)
brew install bash-completion

# Verify: reload your shell and try Tab completion
bash
# Press Tab twice for file/command suggestions
```

Ensure your `~/.bashrc` includes:
```bash
if [ -f /etc/bash_completion ]; then
    . /etc/bash_completion
fi
```

#### Zsh

Zsh has built-in completion. Configure in `~/.zshrc`:

```bash
# Enable completion
autoload -U compinit && compinit

# Optional: use bash-completion package
if [ -f /usr/local/share/zsh/site-functions ]; then
    fpath=(/usr/local/share/zsh/site-functions $fpath)
fi
```

Reload your shell:
```bash
exec zsh
```

#### PowerShell (Windows)

PowerShell 7+ has tab completion built-in. For earlier versions, use PSReadLine:

```powershell
# Install PSReadLine (if not already present)
Install-Module -Name PSReadLine -AllowClobbered -Force

# Add to your PowerShell profile
if ($PROFILE) {
    Add-Content -Path $PROFILE -Value "Import-Module PSReadLine"
}
```

### Debugging Tab Completion Issues

If **Tab** does not show suggestions:

1. **Verify shell type**:
   ```bash
   echo $SHELL
   # Should output: /bin/bash, /bin/zsh, or /usr/bin/pwsh
   # If it shows /bin/sh, reconfigure to use bash or zsh
   ```

2. **Check completion package**:
   ```bash
   # Bash
   rpm -q bash-completion       # RedHat/CentOS
   dpkg -l | grep bash-completion  # Debian/Ubuntu
   
   # Zsh
   compaudit                    # Check Zsh setup
   ```

3. **Verify terminal width**:
   - Zeta's terminal panel must be at least 40 columns wide for completion menus
   - If your terminal is too narrow, resize the pane or toggle fullscreen (F11)

4. **Manual test**:
   ```bash
   # Test completion by typing a command prefix and pressing Tab
   l<Tab>  # Should suggest: ls, ln, locate, etc.
   ```

5. **Enable debug output** (if needed):
   ```bash
   # Bash: set -x to trace completion
   set -x
   # Type command and Tab again to see completion logs
   set +x
   ```

---

## Platform-Specific Behavior

### Windows (ConPTY)

Zeta uses **ConPTY** (Windows Pseudo Console) to run shells on Windows.

**Characteristics**:
- Automatically wraps `cmd.exe` or `PowerShell` as the default shell
- Line endings are CRLF (`\r\n`)
- ANSI escape sequences are partially supported (colors work, cursor positioning works)
- Ctrl+C sends SIGINT correctly

**Shell Selection**:
- Priority 1: `PowerShell` (if available)
- Priority 2: `cmd.exe` (fallback)

**Potential Issues**:
- Some legacy `cmd.exe` commands may not support ANSI colors; switch to PowerShell for better rendering
- Very long command lines (>2048 chars) may not be handled correctly by the ConPTY layer

### Unix/Linux

Zeta spawns your configured shell from `$SHELL` environment variable.

**Characteristics**:
- Line endings are CR only (`\r`)
- Full ANSI escape sequence support (including 256-color and true-color palettes)
- Standard Unix signal handling (Ctrl+C = SIGINT, Ctrl+Z = SIGSTOP)

**Typical shells**:
- `bash` (most common)
- `zsh` (preferred by many developers)
- `fish` (alternative with enhanced completion)

### macOS

Zeta uses the same PTY mechanism as Linux.

**Additional notes**:
- Default shell is determined by `$SHELL` variable
- macOS 10.15+ uses `zsh` by default (Apple deprecated `bash` 3.2)
- Full ANSI color support
- Homebrew-installed shells (e.g., `bash@5`, `zsh@5`) work without issue

---

## Opening Terminal in Fullscreen

### Toggle Terminal On/Off

**F2** or **Ctrl+\** — Opens or closes the terminal panel.

When closed, the terminal state is preserved but no longer renders; switching back to the terminal resumes from where you left off.

### Fullscreen Mode

**F11** (while terminal has focus) — Expands the terminal to fill the viewport, hiding the file manager temporarily.

Useful for:
- Running interactive TUI programs (vim, htop, less, etc.)
- Viewing long command output without scrollback limitation
- Giving the shell maximum screen real estate

Press **F11** again to return to the split view.

---

## Known Limitations

### Unsupported Key Combinations

The following keys cannot be transmitted to the shell due to terminal emulation constraints:

- **Function keys F1, F3–F10**: Reserved by Zeta for application features
- **Alt+<key>** modifiers: Not currently mapped to ANSI sequences (this is a future enhancement)
- **Home, End, Page Up, Page Down**: No standard terminal code defined for these in the current implementation
- **Insert/Delete**: Treated as unhandled keys

### Tab Completion Edge Cases

- **Narrow terminal**: If the terminal panel is fewer than 40 columns wide, completion menus may not render correctly
- **Missing completion data**: If `bash-completion` or shell configuration is missing, no suggestions will appear
- **Custom completion scripts**: User-defined completion functions in `~/.bashrc` or `~/.zshrc` are respected but may have performance impact on slow systems

### Rendering Limitations

- **Scrollback buffer**: Limited by available memory (typically 1000–10000 lines depending on system RAM)
- **Very wide lines**: Lines exceeding terminal width will wrap; some ANSI rendering may not align correctly
- **Simultaneous output**: If the shell produces very high-volume output, rendering may lag (this is expected on slower systems)

---

## Future Enhancements

Planned improvements to terminal integration include:

1. **Alt+<key> support**: Map Alt modifier to ANSI Alt sequences (ESC-prefixed codes)
2. **Configurable shell selection**: Allow users to choose their preferred shell in settings
3. **Terminal scrollback buffer**: Make the buffer size configurable
4. **Mouse support**: Enable mouse clicks to select text and interact with TUI programs
5. **Custom keybindings**: Allow users to rebind F2, Ctrl+\, and F11 in the settings panel
6. **Session recording**: Optional replay/recording of terminal sessions for documentation

---

## Shell Selection (Windows)

### Shell Selection Priority

On Windows, Zeta automatically selects the best available shell in this order:

1. **PowerShell 7+** (`pwsh.exe`) — if found via `where pwsh.exe`
2. **Command Prompt** (`cmd.exe`) — via `COMSPEC` environment variable (always available as fallback)

**Important**: The `SHELL` environment variable is **not** checked on Windows. Setting `$env:SHELL` will have no effect on which shell Zeta uses.

### Using a Different Shell

If you want to use a different shell (like Git Bash, bash from WSL, or another alternative), you have these options:

1. **Run your preferred shell inside the terminal**:
   ```powershell
   # Inside Zeta's terminal, simply type the command to launch your shell
   bash                    # If Git Bash or WSL bash is in PATH
   wsl                     # To enter Windows Subsystem for Linux
   ```

2. **Use Git Bash separately**: Open Git Bash as your shell application, then run Zeta from within it

3. **Set PowerShell as default**: If you prefer PowerShell, ensure `pwsh.exe` is in your PATH and Zeta will auto-select it (it has priority over `cmd.exe`)

4. **Add bash to PATH**: If you have Git Bash or another bash installation, add its `bin` directory to your system PATH so it's accessible from any shell

### Implementation Note

Currently, custom shell selection is not configurable in Zeta on Windows. If you need the ability to specify a custom shell, consider:
- Launching your preferred shell and running Zeta from within it
- Filing a GitHub issue with your use case for future enhancement

---

## Debugging Tips

### Check Terminal Status

Verify the terminal is running and responding:

```bash
# Inside the terminal, type:
echo "Terminal is working"
date
whoami
```

If you see output, the terminal is functioning correctly.

### Verify Tab Completion

1. Type a partial filename and press **Tab**:
   ```bash
   l<Tab>  # Should suggest: ls, ln, locale, etc.
   ```

2. If no suggestions appear:
   - Ensure `bash-completion` is installed
   - Check shell type: `echo $SHELL`
   - Verify terminal width: resize the panel or toggle fullscreen

3. Test in a new shell session:
   ```bash
   bash --login  # Load fresh config
   l<Tab>
   ```

### Isolate Rendering Issues

If text appears corrupted or colors are wrong:

1. **Try a simple command**:
   ```bash
   echo "test"
   ls -la
   ```

2. **Check terminal width**:
   ```bash
   tput cols   # Should return your terminal width (e.g., 80, 120)
   ```

3. **Test ANSI colors**:
   ```bash
   echo -e "\x1b[31mRed\x1b[0m"  # Should print "Red" in red
   ```

4. **Restart the terminal**:
   - Press **F2** to close the terminal
   - Press **F2** again to reopen
   - This resets the emulation state

### Performance Debugging

If the terminal is slow or laggy:

1. **Reduce terminal width**: Fewer columns = less rendering work
2. **Avoid high-volume output**: Test with commands that don't produce thousands of lines
3. **Check system load**: Use `top` or `Task Manager` to verify CPU/RAM availability
4. **Use fullscreen mode**: F11 maximizes rendering efficiency by removing file manager rendering

### Known Issues & Workarounds

| Symptom | Cause | Solution |
|---------|-------|----------|
| Tab completion shows no suggestions | Completion package missing or shell is `sh` | Install `bash-completion`, verify shell is `bash`/`zsh` |
| Backspace doesn't delete characters | Terminal not recognizing DEL (ASCII 127) | Update shell config; try Ctrl+H instead |
| Arrow keys not working in vim/less | ANSI sequences not being received | Ensure terminal width is sufficient; toggle fullscreen |
| Colors appear wrong or missing | ANSI color mapping issue | Verify shell supports colors: `echo $TERM` |
| Terminal freezes | PTY buffer full or blocking I/O | Close other applications; restart terminal (F2, F2) |

---

## Additional Resources

- **VT100 Terminal Emulation**: https://en.wikipedia.org/wiki/VT100
- **ANSI Escape Codes**: https://en.wikipedia.org/wiki/ANSI_escape_code
- **Bash Completion**: https://github.com/scop/bash-completion
- **Zsh Completion**: https://zsh.sourceforge.io/Doc/Release/Completion.html
- **PowerShell PSReadLine**: https://github.com/PowerShell/PSReadLine

---

*Last Updated: April 2026*
*Zeta File Manager v1.0+*
