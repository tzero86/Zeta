# Remote Filesystems (SSH/SFTP)

## Overview
Zeta now supports connecting to remote servers via SFTP and SSH. This allows seamless browsing, editing, and management of files on remote hosts while maintaining the familiar local experience.

### Connection Steps
1. **Initiate Connect:** Use `Ctrl+R` or navigate to the Network menu item (File > Network > Connect SSH).
2. **Input Details:** Provide the Host, Port, and User credentials when prompted.
3. **Authentication:** Zeta supports multiple methods:
    *   **SSH Agent (Recommended):** If available, Zeta will automatically use keys loaded via `ssh-add`, providing passwordless authentication.
    *   **Private Key File:** Specify an explicit path to a private key file (e.g., `~/.ssh/id_rsa`).
    *   **Password:** Fallback method for single passwords.

### Security & Best Practices
*   **Host Key Verification:** To prevent Man-in-the-Middle attacks, it is critical to use and verify host keys against the system's known hosts file (`~/.ssh/known_hosts`). If the remote host key changes unexpectedly, Zeta **MUST** refuse connection.
*   **Agent Support:** For secure, passwordless operation across multiple sessions, ensure your SSH Agent is running and that all necessary keys are loaded using `ssh-add`.

### Usage Details
Once connected, the active pane's path state will update to reflect a remote path (e.g., `sftp://user@host:/path/to/remote`). All file operations (Copy, Move, Delete) are executed via the SFTP backend and appear seamless within the UI.

### Error Codes
Connection failures are explicitly categorized:
*   **Authentication Failed:** Credentials (password or key) were rejected by the remote server.
*   **Connection Failed:** A network issue occurred (e.g., timeout, host unreachable).
