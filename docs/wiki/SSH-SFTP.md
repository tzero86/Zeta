# SSH / SFTP Remote Filesystems

Zeta lets you browse and operate on remote servers via SSH/SFTP directly from the file pane — no separate SFTP client needed.

---

## Connecting

Open the SSH Connect dialog from:
- **Command palette** (`Shift+P`) → "SSH Connect"
- **File menu** → Connect to Remote

Fill in the host, port (default 22), username, and authentication method, then press Enter.

---

## Authentication Methods

Zeta respects your explicit authentication choice in the dialog:

### SSH Agent (Recommended)

Select **Agent** to use `SSH_AUTH_SOCK`. The dialog shows `[Agent: Available]` or `[Agent: Not Available]` so you know before connecting.

```bash
# Start SSH Agent (typically in ~/.bashrc or ~/.profile)
eval "$(ssh-agent)"
# Add your key
ssh-add ~/.ssh/id_rsa
# Verify the socket is set
echo $SSH_AUTH_SOCK
```

Benefits:
- Keys never leave the agent process
- No re-entering passwords for multiple connections
- Passphrases cached for the session

### Password

Select **Password** and enter your credentials. SSH Agent is skipped entirely.

### Key File

Select **Key File** and provide the path to your private key. SSH Agent is skipped entirely.

**Permissions:** SSH key files must be readable only by you:

```bash
chmod 700 ~/.ssh
chmod 600 ~/.ssh/id_rsa
```

---

## Host Key Verification

On the **first connection** to a host, Zeta shows a trust prompt with both fingerprints:

- **SHA256** (preferred, modern): `SHA256:` prefix followed by base64-encoded hash
- **MD5** (legacy, for compatibility): hex colon-separated hash

**Verification workflow:**
1. Compare the displayed fingerprints with those from your server admin (out-of-band)
2. Press Enter / Y to trust the host
3. The key is written to `~/.ssh/known_hosts` for future connections
4. If the key changes on a future connection, Zeta shows a **red security warning** and refuses to connect

**Verify fingerprints manually:**

```bash
# SHA256
ssh-keyscan -H example.com 2>/dev/null | ssh-keygen -lf - -E SHA256

# MD5 (legacy)
ssh-keyscan -H example.com 2>/dev/null | ssh-keygen -lf - -E MD5
```

---

## Remote File Operations

Once connected, the active pane shows the remote filesystem. All standard file operations work transparently on the remote:

- Browse, copy, move, rename, delete files and directories
- Copy files **between local and remote** (cross-filesystem copy)
- The opposite pane can be local or another remote connection

---

## Troubleshooting

| Issue | Likely cause | Solution |
|---|---|---|
| "SSH Agent not available" | Agent not running or `SSH_AUTH_SOCK` not set | Run `eval "$(ssh-agent)"` and `ssh-add ~/.ssh/id_rsa` |
| "No matching identity in SSH Agent" | Agent is running but your key isn't loaded | Run `ssh-add ~/.ssh/id_rsa` |
| "Authentication failed" | Wrong password, wrong key, or bad permissions | Test with `ssh user@host` in your terminal first |
| "Host key changed" (red error) | Key mismatch — possible security issue | Run `ssh-keygen -R host.example.com`, then reconnect and re-verify |
| "Connection timeout" | Host unreachable | Check with `ping` or `ssh` from terminal |
| "Host key not recognized" | New host, needs manual verification | Verify fingerprints with server admin, then press Enter |
| "Permission denied" (key file) | Key file permissions too broad | Run `chmod 600 ~/.ssh/id_rsa` |

---

## Requirements

- **OpenSSH client** installed (standard on Linux/macOS; download from [openssh.com](https://www.openssh.com) for Windows)
- **`~/.ssh/known_hosts`** file (created automatically after first `ssh` connection on any host)
- Authentication credentials: password, key file, or SSH Agent

---

*See also: [File Management](File-Management) · [Key Bindings](Key-Bindings) · [Configuration](Configuration)*
