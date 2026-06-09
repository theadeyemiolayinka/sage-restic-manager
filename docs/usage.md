# Usage

Sage Restic Manager operates in two modes: an interactive TUI for daily operations and a CLI for automation and scripting.

## TUI

Running the binary without arguments starts the TUI:

```bash
sage-restic-manager
```

### Navigation

The interface is divided into eight screens, accessed via the tab bar or number keys:

| Key | Screen | Purpose |
|-----|--------|---------|
| `1` | Dashboard | Storage gauge, budget status, and summary |
| `2` | Sources | Backup source discovery, approval, and selection |
| `3` | Repository | Init, check, backup, prune, and statistics |
| `4` | Snapshots | Browse and search snapshots |
| `5` | Restore | Restore snapshots to local paths |
| `6` | Scheduler | Install, enable, disable, and view timer status |
| `7` | Logs | Scroll through application and backup logs |
| `8` | Settings | Edit repository URL, budget, and binary path |

Press `Tab` to cycle forward through screens. Press `q` to quit.

### Screen-Specific Controls

#### Dashboard

The dashboard displays a storage usage gauge that changes color based on budget thresholds:

- **Green** - Below warning threshold
- **Yellow** - Above warning threshold
- **Red** - Above critical threshold

It also shows repository statistics, next scheduled run time, and systemd timer status.

#### Sources

| Key | Action |
|-----|--------|
| `d` | Discover Docker volumes at the configured path |
| `Enter` | Toggle selection of the highlighted source |
| `a` | Approve an unapproved source |
| `i` | Ignore a source (excluded from discovery) |
| `s` | Save source configuration to disk |
| `/` | Search source labels |
| `+` | Add a custom flat path |

Sources cycle through states: `?` (unapproved) -> `-` (unselected) -> `+` (selected) -> `-`.

Only approved (`-` or `+`) sources are considered. Only selected (`+`) sources are included in backups.

#### Repository

| Key | Action |
|-----|--------|
| `i` | Initialize a new Restic repository |
| `c` | Run `restic check` |
| `b` | Run backup of all selected sources |
| `p` | Run `restic forget --prune` with configured retention |
| `r` | Refresh repository statistics |

During backup, the TUI shows a progress overlay with file count, byte count, and elapsed time. Press `Ctrl+q` to send `SIGTERM` to the backup process.

#### Snapshots

| Key | Action |
|-----|--------|
| `Up/Down` | Navigate snapshot list |
| `r` | Refresh snapshot list |
| `R` | Jump to Restore screen with selected snapshot |

#### Restore

| Key | Action |
|-----|--------|
| `t` | Set target directory for restore |
| `p` | Set source path within the snapshot (leave empty for full restore) |
| `Enter` | Execute restore |

The restore operation runs `restic restore` for the chosen snapshot ID, extracting to the target path.

#### Scheduler

| Key | Action |
|-----|--------|
| `i` | Generate and install systemd service and timer units |
| `e` | Enable and start the timer |
| `d` | Disable the timer |
| `f` | Set frequency (daily, twice weekly, weekly) |
| `c` | Set a custom `OnCalendar` expression |

The install action writes files to `~/.config/systemd/user/`. After installing, you must enable the timer with `e`.

#### Logs

| Key | Action |
|-----|--------|
| `Up/Down` | Scroll log entries |
| `g` | Jump to top |
| `G` | Jump to bottom |
| `c` | Clear in-memory log entries |
| `e` | Export logs to a timestamped file in the log directory |

#### Settings

| Key | Action |
|-----|--------|
| `Up/Down` | Navigate settings |
| `Enter` | Edit the highlighted setting |

Editable fields: repository URL, storage budget, warning threshold, critical threshold. Other settings must be edited in `config.toml` directly.

### Input Modes

When editing a value, the TUI switches to input mode. Type the value and press `Enter` to confirm, or `Esc` to cancel.

For destructive actions (prune, removing sources), a confirmation dialog appears. Type the confirmation word exactly and press `Enter`.

## CLI

All TUI actions have CLI equivalents for scripting and CI/CD integration.

### Global Options

```bash
sage-restic-manager [OPTIONS] <COMMAND>
```

| Option | Description |
|--------|-------------|
| `--non-interactive` | Suppress TUI; use CLI output only |

### Commands

#### backup

Run a backup of all selected sources.

```bash
sage-restic-manager backup --non-interactive
```

Exits with a non-zero status if no sources are selected or if Restic is unavailable.

#### check

Verify the repository integrity.

```bash
sage-restic-manager check --non-interactive
```

Prints `Repository check passed.` or `Repository check FAILED:` with stderr.

#### snapshots

List snapshots in the repository.

```bash
sage-restic-manager snapshots --non-interactive
```

#### forget

Apply the configured retention policy and optionally prune.

```bash
sage-restic-manager forget --non-interactive
```

Use `--dry-run` to preview what would be removed without deleting data.

#### discover

Scan for Docker volumes and print results.

```bash
sage-restic-manager discover --non-interactive
```

#### install-schedule

Generate and install systemd units.

```bash
sage-restic-manager install-schedule --non-interactive
```

#### config

Print current configuration and file paths.

```bash
sage-restic-manager config --non-interactive
```

Output includes:

- Config directory path
- Repository URL
- Budget thresholds
- Restic binary path
- Update channel
- Selected source count

#### self-update

Check GitHub releases and update the binary if a newer version is available.

```bash
sage-restic-manager self-update --non-interactive
```

The updater verifies SHA256 checksums and minisign signatures before replacing the binary. A `.bak` copy of the current binary is kept during the process. If replacement fails, the updater attempts to restore the backup.

#### logs

Show recent log output via `journalctl` (for systemd-scheduled runs) or the local log file.

```bash
sage-restic-manager logs --non-interactive --lines 100
```

### Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | General error |
| `130` | Interrupted (SIGINT) |
