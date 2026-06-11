# Scheduling

Sage Restic Manager integrates with systemd timers for reliable, cron-free scheduling. It generates user-level systemd units and manages their lifecycle through the TUI or CLI.

## Why Systemd Timers

Compared to cron jobs, systemd timers offer:

- Structured logging via `journalctl`
- Dependency ordering (for example, wait for network)
- Automatic missed-run handling with `Persistent=true`
- Status inspection via `systemctl status`
- Per-job resource limits and sandboxing

## Quick Setup

### From the TUI

1. Navigate to the **Scheduler** screen (`6`)
2. Press `f` to choose a frequency:
   - **Daily** - Every day
   - **Twice Weekly** - Monday and Thursday at 02:00
   - **Weekly** - Once per week
   - **Custom** - Enter a raw systemd `OnCalendar` expression
3. Press `i` to install the systemd unit files
4. Press `e` to enable and start the timer

The dashboard shows the next trigger time and whether the timer is active.

### From the CLI

```bash
sage-restic-manager install-schedule --non-interactive
```

This writes the service and timer files to `~/.config/systemd/user/`. To enable manually:

```bash
systemctl --user daemon-reload
systemctl --user enable sage-restic-manager.timer
systemctl --user start sage-restic-manager.timer
```

## Generated Units

### Timer Unit

`~/.config/systemd/user/sage-restic-manager.timer`

```ini
[Unit]
Description=sage-restic-manager scheduled backup
Requires=sage-restic-manager.service

[Timer]
OnCalendar=Mon,Thu 02:00:00
Persistent=true
RandomizedDelaySec=1800

[Install]
WantedBy=timers.target
```

- `Persistent=true` ensures that if the system is off during the scheduled time, the backup runs as soon as the system comes back online
- `RandomizedDelaySec=1800` spreads load by adding up to 30 minutes of jitter

### Service Unit

`~/.config/systemd/user/sage-restic-manager.service`

```ini
[Unit]
Description=sage-restic-manager backup job
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
ExecStart=/usr/local/bin/sage-restic-manager backup --non-interactive
StandardOutput=journal
StandardError=journal
Environment="PATH=/usr/local/bin:/usr/bin:/bin:/snap/bin"
ConditionACPower=true
```

The service runs with your current user. It requires `network-online.target` to ensure the repository backend (S3, B2, or local network share) is reachable before starting.

### Custom OnCalendar Expressions

For advanced scheduling, press `c` in the Scheduler screen and enter a systemd calendar expression.

Examples:

| Expression | Meaning |
|------------|---------|
| `*-*-* 03:00:00` | Every day at 03:00 |
| `Mon..Fri *-*-* 04:00:00` | Every weekday at 04:00 |
| `*-*-1..7 02:00:00` | First week of every month at 02:00 |
| `*-*-15 03:00:00` | 15th of every month at 03:00 |

Invalid characters (brackets, newlines, semicolons) are rejected when saving the schedule.

## Lifecycle Management

### Check Status

```bash
systemctl --user status sage-restic-manager.timer
systemctl --user status sage-restic-manager.service
```

### View Logs

```bash
journalctl --user -u sage-restic-manager.service -n 100
```

Or use the built-in `logs` command:

```bash
sage-restic-manager logs --non-interactive --lines 100
```

### Disable

In the TUI, press `d` on the Scheduler screen. Or from the shell:

```bash
systemctl --user disable sage-restic-manager.timer
systemctl --user stop sage-restic-manager.timer
```

### Remove Units

Delete the files from `~/.config/systemd/user/` and reload systemd:

```bash
rm ~/.config/systemd/user/sage-restic-manager.*
systemctl --user daemon-reload
```

## Battery and Boot Scheduling

For laptops, set `run_on_battery = false` in `schedules.toml` to skip backups when running on battery power. This adds:

```ini
ConditionACPower=true
```

To run a backup a fixed time after boot (for example, 5 minutes), set:

```toml
run_after_boot_sec = 300
```

This adds `OnBootSec=300s` to the timer unit.

## Permissions

User-level systemd units run under your own account. They can access:

- Your home directory
- The Docker socket (if you are in the `docker` group)
- The OS keyring (via the user session bus)

If you need to back up paths readable only by root (for example, system Docker volumes), consider running the timer as a system-level service instead, configured outside the TUI.
