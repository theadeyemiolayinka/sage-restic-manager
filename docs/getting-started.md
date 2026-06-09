# Getting Started

## Prerequisites

Before installing Sage Restic Manager, ensure the following are present on your system:

- **Rust toolchain** (if building from source): `cargo` 1.80 or later
- **Restic**: Version 0.16 or newer installed at `/usr/local/bin/restic` or another location on PATH
- **Systemd**: For timer-based scheduling
- **Secret service**: `gnome-keyring-daemon`, `kwalletd5`, or another freedesktop-secret-compatible provider
- **Docker** (optional): Only required if backing up Docker volumes

Verify Restic is available:

```bash
restic version
```

## Installation

### Install with curl (recommended)

The one-line installer detects your platform, downloads the correct release binary, verifies the SHA256 checksum, and installs to `/usr/local/bin` (or `~/.local/bin` if the directory is not writable):

```bash
curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/sage-restic-manager/main/install.sh | bash
```

Install a specific version:

```bash
curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/sage-restic-manager/main/install.sh | bash -s -- v0.1.1
```

**How it works:**

1. Detects your OS and architecture (Linux x86_64/aarch64, macOS x86_64/aarch64)
2. Fetches the latest release metadata from GitHub (or uses the version you specify)
3. Downloads the binary and its SHA256 checksum
4. Verifies the checksum
5. Installs the binary with `755` permissions
6. Warns if the target directory is not on your PATH

### Build from Source

Clone the repository and build the release binary:

```bash
git clone https://github.com/theadeyemiolayinka/sage-restic-manager.git
cd sage-restic-manager
cargo build --release
```

The binary is produced at `target/release/sage-restic-manager`. Move it to a location on PATH:

```bash
sudo cp target/release/sage-restic-manager /usr/local/bin/
sudo chmod 755 /usr/local/bin/sage-restic-manager
```

### Verify Installation

Run the binary to see the version and available subcommands:

```bash
sage-restic-manager --help
```

## Initial Setup

On first run, the application creates a configuration directory at `~/.config/sage-restic-manager/` with default files:

- `config.toml` - Repository, retention, and budget settings
- `sources.toml` - Backup sources and Docker volume path
- `schedules.toml` - Timer schedules
- `credentials.toml` - Credentials (only if keyring is unavailable)

The directory is created with `0o700` permissions so only your user can read it.

### Start the TUI

Launch the interactive interface:

```bash
sage-restic-manager
```

Without arguments, the application opens the TUI. Use `Tab` or number keys `1` through `8` to switch screens.

### Configure the Repository

Navigate to the **Settings** screen (press `8`) and edit the following fields:

1. **Repository URL** - The Restic repository location. Examples:
   - Local: `/backup/restic-repo`
   - S3: `s3:https://s3.amazonaws.com/my-bucket`
   - B2: `b2:my-bucket:my-prefix`
2. **Storage Budget** - Total allowed repository size in GiB
3. **Warning Threshold** - Size at which the TUI shows a warning
4. **Critical Threshold** - Size at which the TUI shows a critical alert

Press `Enter` on a field to edit it. Values are saved automatically.

### Set Credentials

The application stores credentials in your OS keyring by default. If no secret service is running, it falls back to `credentials.toml` with `0o600` permissions and logs a warning.

The credential fields are:

- `access_key_id` - For S3 or B2 (account ID for B2)
- `secret_access_key` - For S3 or B2 (application key for B2)
- `repository_password` - The Restic repository encryption password

You can set these in the TUI when prompted, or by editing `~/.config/sage-restic-manager/credentials.toml` directly if the keyring is unavailable.

### Initialize the Repository

If the repository is new, initialize it from the **Repository** screen (press `3`, then `i`). This runs `restic init` with the configured password and backend credentials.

## First Backup

### Discover Sources

Navigate to the **Sources** screen (press `2`). If Docker is installed, press `d` to discover volumes. The scanner reads `/var/lib/docker/volumes` by default (customizable in settings).

Discovered volumes appear with a `?` state, meaning they are unapproved.

### Approve and Select

- Press `a` to approve a source, changing its state from `?` to `-` (unselected)
- Press `Enter` to toggle selection. Selected sources show a `+` state
- Press `s` to save the source configuration

Only approved and selected sources are included in backups.

### Run the Backup

Go to the **Repository** screen (press `3`) and press `b` to run a backup immediately. The TUI shows real-time progress. Alternatively, run from the CLI:

```bash
sage-restic-manager backup --non-interactive
```

After completion, the **Sources** screen updates each source with its last backup time, snapshot ID, and status.

## Next Steps

- [Configure automated scheduling](scheduling.md) with systemd timers
- Read the [Docker volume backup guide](docker-volumes.md) for container-specific workflows
- Review [security practices](security.md) for credential management
