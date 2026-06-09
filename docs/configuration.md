# Configuration

All configuration lives in `~/.config/sage-restic-manager/`. Each file is TOML. The application creates defaults on first run. Every config file is written atomically with `0o600` permissions on Unix.

## Directory Layout

```
~/.config/sage-restic-manager/
├── config.toml
├── sources.toml
├── schedules.toml
└── credentials.toml      # Fallback only; prefer keyring
```

The directory itself is created with `0o700` permissions.

## config.toml

The main application configuration.

```toml
[repository]
url = "s3:https://s3.amazonaws.com/my-backup-bucket"

[repository.env]
# S3 example
AWS_ACCESS_KEY_ID = "AKIAXXXXXXXXXXXXXXXX"
AWS_SECRET_ACCESS_KEY = "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
AWS_ENDPOINT_URL_S3 = "https://s3.eu-central-1.example.com"

# B2 example (alternative to S3)
# B2_ACCOUNT_ID = "xxxxxxxxxxxxxxxxxxxxxxxxxxxx"
# B2_ACCOUNT_KEY = "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"

[retention]
keep_daily = 7
keep_weekly = 4
keep_monthly = 6
keep_yearly = 2

[budget]
total_bytes = 8589934592        # 8 GiB
warning_bytes = 6442450944      # 6 GiB
critical_bytes = 7516192768     # 7 GiB

restic_binary = "restic"
update_channel = "stable"
```

### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `repository.url` | string | required | Restic repository location. Supports local paths, `s3:`, `s3:http://`, and `b2:` schemes. |
| `repository.env` | table | `{}` | Backend-specific environment variables written to the Restic child process. |
| `retention.keep_daily` | integer | `7` | Snapshots to keep per day during `forget`. |
| `retention.keep_weekly` | integer | `4` | Snapshots to keep per week. |
| `retention.keep_monthly` | integer | `6` | Snapshots to keep per month. |
| `retention.keep_yearly` | integer | `2` | Snapshots to keep per year. |
| `budget.total_bytes` | integer | `0` | Hard storage budget in bytes. `0` disables budgeting. |
| `budget.warning_bytes` | integer | `0` | Threshold for warning state. |
| `budget.critical_bytes` | integer | `0` | Threshold for critical state. |
| `restic_binary` | string | `"restic"` | Path or name of the Restic executable. Must be absolute if used with systemd scheduling. |
| `update_channel` | string | `"stable"` | Release channel for self-updates. `"stable"` or `"nightly"`. |

### Backend Environment Variables

Values under `[repository.env]` are passed to Restic. The application handles two special cases:

- **S3**: When `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` are present, they are written to a temporary `AWS_SHARED_CREDENTIALS_FILE` with `0o600` permissions instead of being passed as environment variables. The endpoint URL is passed normally.
- **B2**: `B2_ACCOUNT_ID` and `B2_ACCOUNT_KEY` are passed as environment variables to Restic. Backblaze B2 requires this mechanism; no shared-credentials-file equivalent exists.

## sources.toml

Defines backup sources and their states.

```toml
docker_volumes_path = "/var/lib/docker/volumes"

[[sources]]
path = "/var/lib/docker/volumes/postgres-data/_data"
label = "postgres-data"
kind = "docker_volume"
state = "selected"
tags = ["database", "production"]
exclude_patterns = ["*.tmp", "*.pid"]
last_backup = "2026-06-01T04:00:00Z"
last_snapshot_id = "abc1234"
last_backup_status = "success"
size_bytes = 2147483648
first_discovered = "2026-05-15T10:30:00Z"
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `docker_volumes_path` | string | Root path for Docker volume discovery. Default is `/var/lib/docker/volumes`. |
| `sources` | array | List of backup sources. |
| `sources[].path` | string | Absolute path to back up. |
| `sources[].label` | string | Human-readable label. |
| `sources[].kind` | string | `"flat_path"`, `"docker_volume"`, or `"container_root"`. |
| `sources[].state` | string | `"unapproved"`, `"selected"`, `"unselected"`, or `"ignored"`. |
| `sources[].tags` | array of strings | Tags appended to the Restic backup command. |
| `sources[].exclude_patterns` | array of strings | Glob patterns passed to `--exclude`. |
| `sources[].last_backup` | string (RFC 3339) | Timestamp of last successful or attempted backup. |
| `sources[].last_snapshot_id` | string | Restic snapshot ID from last backup. |
| `sources[].last_backup_status` | string | `"running"`, `"success"`, `"partial"`, or `"failed"`. |
| `sources[].size_bytes` | integer | Estimated size at time of discovery. |
| `sources[].first_discovered` | string (RFC 3339) | When the source was first found by the scanner. |

## schedules.toml

Defines backup schedules. The active schedule is used to generate systemd timer and service units.

```toml
[[schedules]]
name = "default"
enabled = true
frequency = "twice_weekly"
on_calendar = null
run_after_boot_sec = 300
run_on_battery = false
```

### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `schedules[].name` | string | required | Schedule identifier. |
| `schedules[].enabled` | boolean | `false` | Whether this schedule is active. Only one schedule should be enabled at a time. |
| `schedules[].frequency` | string | `"twice_weekly"` | `"daily"`, `"twice_weekly"`, `"weekly"`, or `"custom"`. |
| `schedules[].on_calendar` | string or null | `null` | Raw systemd `OnCalendar` expression. Required when `frequency` is `"custom"`. |
| `schedules[].run_after_boot_sec` | integer or null | `null` | Seconds after boot to trigger a backup. Maps to `OnBootSec`. |
| `schedules[].run_on_battery` | boolean or null | `null` | If `false`, adds `ConditionACPower=true` to the service unit. |

### Frequency Mapping

| Frequency | OnCalendar Value |
|-----------|-----------------|
| `daily` | `daily` |
| `twice_weekly` | `Mon,Thu 02:00:00` |
| `weekly` | `weekly` |
| `custom` | Value of `on_calendar` |

### OnCalendar Validation

Custom calendar expressions are validated on load. Only alphanumeric characters, spaces, and the symbols `*:-,./` are permitted. Newlines, carriage returns, and brackets are rejected.

## credentials.toml

Fallback storage when the OS keyring is unavailable. **Prefer the keyring.**

```toml
access_key_id = "AKIAXXXXXXXXXXXXXXXX"
secret_access_key = "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
repository_password = "my-very-strong-password"
```

This file is created with `0o600` permissions. If the keyring becomes available later, saving credentials will migrate them to the keyring and delete this file automatically.

### Keyring Storage

Credentials are serialized to JSON and stored under the service `sage-restic-manager` with the user `credentials`. This is compatible with GNOME Keyring, KDE Wallet, KeePassXC, and other freedesktop-secret implementations.

## Log Directory

Runtime logs are written to `~/.local/share/sage-restic-manager/logs/` by default. Each backup run produces timestamped log entries accessible via the **Logs** screen in the TUI or through `journalctl` when running under systemd.
