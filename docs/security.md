# Security

Sage Restic Manager handles sensitive credentials and executes privileged commands. This page documents the security model, threat mitigations, and recommended practices.

## Credential Storage

### Primary: OS Keyring

The application stores credentials in the platform secret service via the `keyring` crate. The service name is `sage-restic-manager` and the account is `credentials`. This is compatible with:

- GNOME Keyring
- KDE Wallet
- KeePassXC (with secret service integration enabled)
- macOS Keychain
- Windows Credential Manager

Advantages over files:

- Credentials are not written to disk as plaintext
- Access is gated by the user's session login
- Centralized management through the platform's credential UI

### Fallback: Restricted File

If the keyring is unavailable (for example, on a headless server without a secret service), credentials fall back to `~/.config/sage-restic-manager/credentials.toml`.

Security properties of the fallback:

- Written atomically to avoid corruption during power loss
- Permissions set to `0o600` (owner read/write only)
- Parent directory set to `0o700`
- A warning is logged on every load and save indicating that the keyring is not in use

**Recommendation:** Install `gnome-keyring-daemon` or configure KeePassXC secret service integration even on headless servers.

### In-Memory Handling

Credentials are held in `String` fields during runtime. The application does not implement secure memory zeroing on drop. This is standard for Rust applications but means credentials may persist in memory until the page is reused by the allocator. For production environments with strict memory-dump policies, consider running on encrypted swap.

### Debug Output

The `Debug` implementation for `CredentialsConfig` redacts all credential fields, printing `<redacted>` instead of the actual values. This prevents accidental credential exposure in:

- Log files
- Crash reports
- TUI debug overlays
- Structured logging with `tracing`

## Process Security

### Child Process Environment

Before spawning Restic, the application clears the entire environment and repopulates only essential variables:

- `PATH`
- `HOME`
- `TMPDIR`
- `XDG_CACHE_HOME`
- `XDG_CONFIG_HOME`
- `USER`
- Backend-specific credentials (S3 file path, B2 env vars, endpoint URL)

This reduces the attack surface by preventing Restic from inheriting unrelated secrets or modifying behavior through unexpected environment variables.

### S3 Credentials

For S3 backends, `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` are written to a temporary `AWS_SHARED_CREDENTIALS_FILE` with `0o600` permissions. The environment variable `AWS_SHARED_CREDENTIALS_FILE` points Restic to this file.

Advantages over direct env vars:

- Avoids exposure in `/proc/<pid>/environ`
- File is deleted automatically when the `tempfile::NamedTempFile` drops

### B2 Credentials

Backblaze B2 requires `B2_ACCOUNT_ID` and `B2_ACCOUNT_KEY` as environment variables. There is no shared-credentials-file mechanism for B2. These credentials are passed directly to the Restic child process and are visible in `/proc/<pid>/environ` while the backup runs. This is an inherent limitation of the B2 backend.

**Mitigation:** Run backups on systems where you trust all local users, or use S3-compatible gateways instead of native B2.

### Restic Binary Validation

The configured `restic_binary` path is validated before every command execution. Paths containing shell metacharacters (`;`, `&`, `|`, backtick, `$`, parentheses, redirection operators, or null bytes) are rejected with an error. This prevents command injection if an attacker controls the config file.

**Recommendation:** Always use absolute paths for `restic_binary` when systemd scheduling is enabled.

## Self-Update Security

The built-in updater fetches release metadata from GitHub. Every update goes through two verification stages:

1. **SHA256 Checksum** - The downloaded archive hash must match the published `.sha256` asset
2. **Minisign Signature** - The archive is verified against an Ed25519 public key embedded in the binary

If either asset is missing from a release, the update is aborted with an error. There is no option to skip verification.

The updater also:

- Creates a `.bak` backup of the current binary before replacement
- Attempts rollback if replacement fails
- Surfaces both the original failure and any rollback failure in the error message

### Update Public Key

The embedded public key is compiled into the binary at build time. If you fork or rebuild the project, replace the placeholder with your own minisign public key. Updates will fail signature verification until this is done.

## File Permissions

| Path | Permissions | Purpose |
|------|-------------|---------|
| `~/.config/sage-restic-manager/` | `0o700` | Config directory |
| `config.toml` | `0o600` | Repository, retention, budget settings |
| `sources.toml` | `0o600` | Source paths and states |
| `schedules.toml` | `0o600` | Schedule definitions |
| `credentials.toml` | `0o600` | Fallback credential storage |
| `AWS_SHARED_CREDENTIALS_FILE` (temp) | `0o600` | Temporary S3 credentials |
| `password-file` (temp) | `0o600` | Temporary Restic repository password |

## Network Security

- **TLS**: The updater uses `reqwest` with the `rustls-tls` backend. GitHub API and release asset downloads use HTTPS.
- **Timeouts**: API requests have a 10-second connection timeout and 120-second total timeout, preventing slowloris-style hangs.
- **User-Agent**: Requests identify as `sage-restic-manager/<version>`.

## Threat Model

### In Scope

- Local attackers reading config files or process memory
- Supply-chain attacks via compromised GitHub releases
- Credential leakage through logs or debug output
- Command injection via manipulated config values

### Out of Scope

- Root-level attackers who can read arbitrary process memory or kernel memory
- Network-level attackers intercepting Restic traffic to the backend (mitigated by TLS at the backend level, not this application)
- Physical attacks on the machine while logged in

## Recommendations

1. Run the OS secret service and migrate away from `credentials.toml`
2. Use absolute paths for `restic_binary`
3. Prefer S3 over B2 if local process inspection by other users is a concern
4. Enable full-disk encryption so swap and temp files are encrypted at rest
5. Review systemd unit files after installation to ensure `ExecStart` points to the expected binary
