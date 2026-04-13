# Zeta File Manager - SSH/SFTP Remote Filesystems Enhancements (Wave 7B)

**Goal:** Harden the remote filesystem capabilities implemented in Wave 7A by integrating robust security features and improving system integration. The primary focus is on proper host key verification, utilizing an SSH agent for passwordless authentication, and documenting these new features.

**Dependencies & Constraints:**
*   The `ssh2 = "0.9"` dependency is already added (per original plan).
*   All changes must maintain API compatibility with the existing local filesystem module (`src/fs/local.rs`).
*   New security checks **MUST NOT** degrade performance noticeably; they should fail fast and provide clear user feedback.

## Phase 1: Documentation & User Experience (Low Risk)
1.  **Update `README.md`:** Add a new section detailing SSH/SFTP support, including connection instructions and listing the required host key setup (`~/.ssh/known_hosts`).
2.  **Improve UI Feedback:** Enhance error handling in the SSH dialog (`src/ui/ssh.rs`) to differentiate between "Authentication Failed" (wrong password/key) and "Connection Failed" (host unreachable, protocol error).

## Phase 2: Host Key Verification (Medium Risk - Security Hotfix)
1.  **Target Files:** `src/fs/backend.rs` (trait definition), `src/fs/sftp.rs` (implementation).
2.  **Action:** Modify the `SftpBackend::connect` method to take an optional path to a known hosts file (`~/.ssh/known_hosts`).
3.  **Logic:** Use the `ssh2` library's mechanisms (or equivalent system calls) to verify the host key fingerprint against the provided list *before* completing the handshake. If verification fails, abort connection and provide an informative error message ("WARNING: Host key changed! Please investigate manually.").

## Phase 3: SSH Agent Support (High Risk - Authentication Refactor)
1.  **Target Files:** `src/action.rs`, `src/state/ssh.rs`, `src/fs/sftp.rs`.
2.  **Action:** Modify the connection process to check for an available SSH agent socket (`SSH_AUTH_SOCK`).
3.  **Implementation Detail:** If detected, prioritize using the Agent's public key authentication mechanism over passwords or files, falling back to Password/KeyFile only if the agent is unavailable. This requires updating `SshConnectionState` and the connection logic in `SftpBackend::connect`.

## Acceptance Criteria
*   The main documentation file (`README.md`) reflects the new capabilities.
*   A test case exists (or pseudo-test plan) demonstrating successful key verification failure, leading to connection refusal.
*   The connection flow can successfully authenticate using keys provided by a running SSH Agent.