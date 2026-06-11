# Troubleshooting

## Installation Issues

### Restic Not Found

**Symptom:** TUI shows "Restic unavailable" or CLI exits with `ResticNotFound`.

**Solution:**

```bash
which restic
restic version
```

If Restic is installed at a non-standard path, set `restic_binary` in `config.toml` to the absolute path:

```toml
restic_binary = "/opt/restic/restic"
```

### Binary Permission Denied

**Symptom:** `Permission denied` when running the binary after copying to `/usr/local/bin/`.

**Solution:**

```bash
chmod +x /usr/local/bin/sage-restic-manager
```

## Credential Issues

### Keyring Unavailable Warning

**Symptom:** Logs show "Keyring credential load failed; falling back to plaintext file."

**Solution:** Install and start a secret service provider:

```bash
# Ubuntu/Debian
sudo apt install gnome-keyring libsecret-1-0

# Start the daemon in your session
eval $(gnome-keyring-daemon --start --components=secrets)
export SSH_AUTH_SOCK
```

For headless servers, use KeePassXC with secret service integration over DBus.

### Credentials Not Persisting

**Symptom:** You enter credentials in the TUI but they are lost on restart.

**Solution:** Check that `~/.config/sage-restic-manager/` is writable and that saving does not produce an error in the TUI status bar. If the keyring fails to save, verify it is unlocked:

```bash
secret-tool lookup service sage-restic-manager user credentials
```

## Docker Discovery Issues

### Path Not Found

**Symptom:** Discovery shows "Docker volumes path not found."

**Solution:** Verify the path exists and is readable:

```bash
ls -la /var/lib/docker/volumes
```

If Docker uses a custom data root:

```bash
docker info --format '{{ .DockerRootDir }}'
```

Then update `docker_volumes_path` in `sources.toml` or via the TUI.

### Permission Denied

**Symptom:** "Permission denied reading Docker volumes path."

**Solution:** Add your user to the `docker` group. This is the preferred fix because systemd timers run as your user and cannot use `sudo`:

```bash
sudo usermod -aG docker $USER
# Log out and back in for the group change to take effect
```

**Why not sudo?** You can run `sudo sage-restic-manager` for one-off discovery, but scheduled backups via systemd timers run under your own account and will fail with permission denied if your user cannot read the Docker volumes path. The `docker` group grants read access without requiring root.

**Verify access:**

```bash
ls -la /var/lib/docker/volumes
```

If you still see "Permission denied" after joining the group and logging back in, your distribution may use a different Docker data root. Check it with:

```bash
docker info --format '{{ .DockerRootDir }}'
```

### No Volumes Discovered

**Symptom:** Discovery completes but shows zero volumes.

**Solution:** Check that named volumes exist:

```bash
docker volume ls
```

Bind mounts and anonymous volumes do not appear under `/var/lib/docker/volumes`. Add bind mounts manually as flat paths with `+` in the Sources screen.

## Backup Issues

### Backup Fails Immediately

**Symptom:** Backup exits with status code -1 and no snapshot is created.

**Solution:**

1. Verify the repository is initialized: Repository screen (`3`) -> `i` to init
2. Verify credentials are correct: Check `credentials.toml` or keyring contents
3. Test Restic directly:

```bash
restic -r <repo-url> snapshots
```

### Backup Running but No Progress

**Symptom:** TUI shows "Backup running" but the progress string does not update.

**Solution:** Some backends (B2, cold S3 tiers) have high latency for the initial index load. Wait several minutes. Check the **Logs** screen (`7`) for Restic stderr output.

### Repository Lock

**Symptom:** Restic reports "repository is already locked."

**Solution:** A previous backup or check may have crashed. Unlock the repository:

```bash
restic -r <repo-url> unlock
```

Then retry the backup.

## Scheduling Issues

### Timer Not Triggering

**Symptom:** The timer is enabled but backups never run.

**Solution:**

1. Check timer status:

```bash
systemctl --user status sage-restic-manager.timer
```

2. Verify the calendar expression is valid:

```bash
systemd-analyze calendar "Mon,Thu 02:00:00"
```

3. Check if the timer is actually started, not just enabled:

```bash
systemctl --user start sage-restic-manager.timer
```

4. Review the service logs:

```bash
journalctl --user -u sage-restic-manager.service -n 50
```

### Install Fails

**Symptom:** "Install failed" when pressing `i` in the Scheduler screen.

**Solution:** The systemd user directory may not exist:

```bash
mkdir -p ~/.config/systemd/user
systemctl --user daemon-reload
```

## Storage Budget Alerts

### Incorrect Budget Calculation

**Symptom:** Dashboard shows critical warning but the repository is small.

**Solution:** Budget values are in bytes, not GiB. In the TUI you enter GiB, which are converted automatically. If editing `config.toml` directly, ensure you specify bytes:

```toml
[budget]
total_bytes = 1073741824      # 1 GiB
warning_bytes = 805306368     # 0.75 GiB
critical_bytes = 966367641    # 0.9 GiB
```

## Update Issues

### Self-Update Fails

**Symptom:** "Update failed: Release missing SHA256 checksum asset."

**Solution:** The release publisher did not attach a checksum file. Do not install this release. Wait for a properly signed release or build from source.

### Signature Verification Failed

**Symptom:** "Minisign signature verification failed."

**Solution:** The release may have been tampered with or the embedded public key does not match the signing key. Verify the public key in the source and contact the release publisher.

## TUI Issues

### Terminal Not Restoring

**Symptom:** After quitting, the terminal remains in raw mode or the alternate screen persists.

**Solution:** Press `Ctrl+C` or run `reset`. The TUI attempts to restore the terminal on exit, but abrupt termination (SIGKILL, power loss) may leave it in an inconsistent state.

### Corrupted Display

**Symptom:** Characters overlap or lines do not refresh.

**Solution:** Resize the terminal window. The TUI redraws on resize events. Ensure your terminal supports Unicode and 256 colors.

## Getting Help

Before opening an issue, gather the following:

1. Output of `sage-restic-manager config --non-interactive`
2. Last 50 lines of `sage-restic-manager logs --non-interactive --lines 50`
3. Restic version: `restic version`
4. Operating system and version
5. Whether the issue occurs in TUI, CLI, or systemd-scheduled runs
