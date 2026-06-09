# Docker Volume Backup

Docker containers persist data in named volumes or bind mounts. Sage Restic Manager is designed to back up named volumes efficiently without stopping containers, making it ideal for databases, caches, and application state.

## Understanding the Target

Docker named volumes reside under `/var/lib/docker/volumes/<name>/_data` on the host filesystem. Because Docker manages these directories, the application scans the parent path and presents each volume as a selectable backup source.

### Prerequisites

- Docker must be installed and the daemon running
- Your user must have read access to `/var/lib/docker/volumes` or be in the `docker` group
- Restic must be installed on the host (not inside a container)

## Discovery Workflow

### 1. Set the Docker Volumes Path

Navigate to the **Sources** screen and press `Shift+d` (or edit `sources.toml`):

```toml
docker_volumes_path = "/var/lib/docker/volumes"
```

If Docker stores volumes elsewhere (for example, on a custom data root), set the correct path.

### 2. Run Discovery

In the **Sources** screen, press `d`. The scanner reads the volume directory and creates source entries for each named volume it finds.

Example output in the TUI:

```
postgres-data       2.1 GB   ?
redis-cache         512 MB   ?
app-uploads         4.3 GB   ?
```

The `?` state means the source is unapproved. It will not be backed up until you approve it.

### 3. Approve Sources

Highlight each source and press `a` to approve. Approved sources show `-` (unselected).

### 4. Select Sources for Backup

Highlight approved sources and press `Enter` to toggle selection. Selected sources show `+`.

For a production database, you might also add tags before the first backup:

1. Highlight the source
2. Press `Shift+t` to open the tags input
3. Enter comma-separated tags: `database,production,postgres`

Tags are passed to Restic and appear in the snapshot metadata.

### 5. Save

Press `s` to write `sources.toml`. Without saving, your approvals and selections are lost on restart.

## Backup Behavior

When you run a backup (press `b` in the Repository screen), the application:

1. Gathers all selected source paths
2. Collects tags from each source (deduplicated)
3. Collects exclude patterns from each source
4. Runs `restic backup` with `--tag` for each tag and `--exclude` for each pattern

Restic handles the snapshot creation atomically. Even if multiple volumes are selected, they are included in a single snapshot, which is faster and produces cleaner history than separate backups.

### Exclude Patterns

For volatile data you do not want to snapshot, add exclude patterns:

1. Highlight a source
2. Press `Shift+e` to edit exclude patterns
3. Enter comma-separated globs: `*.tmp,*.pid,*.sock`

These are passed directly to Restic's `--exclude` flag.

## Restoration

To restore a volume from a snapshot:

1. Go to the **Snapshots** screen (`4`)
2. Select the snapshot containing the volume data
3. Press `R` to jump to the **Restore** screen (`5`)
4. Press `p` and enter the source path within the snapshot (for example, `/var/lib/docker/volumes/postgres-data/_data`)
5. Press `t` and enter the target path. For a running container, this is the host path to the volume root
6. Press `Enter` to execute

Restic extracts the data to the target path. If the container is running, consider stopping it first to avoid corruption.

### Example: Restore postgres-data

```bash
# Stop the container
docker stop postgres

# The restore target should be the host volume path
sage-restic-manager
# In TUI: select snapshot, set path to /var/lib/docker/volumes/postgres-data/_data
# set target to /var/lib/docker/volumes/postgres-data/_data

# Restart the container
docker start postgres
```

For partial restores (a single file or subdirectory), set the source path to the specific file within the snapshot and the target to a temporary location, then copy the file into the container:

```bash
docker cp /tmp/restore/pg_hba.conf postgres:/etc/postgresql/
```

## Consistency Considerations

### Running Containers

Restic backup is crash-consistent. For databases, this means the on-disk state reflects what was written at the moment of snapshot. To improve consistency:

- Use database-specific tools (pg_dump, mysqldump) for logical backups as a secondary strategy
- Schedule backups during low-traffic periods
- For write-heavy databases, consider stopping the container briefly or using filesystem snapshots at the host level

### Volume Bindings

Bind mounts (`docker run -v /host/path:/container/path`) are not named volumes. The application discovers them only if they fall under the scanned directory. For host paths outside `/var/lib/docker/volumes`, add them as flat paths with `+` in the Sources screen.

## Multi-Container Stacks

For Docker Compose stacks with multiple volumes, run discovery once and approve all relevant volumes. Tag each source with the stack name for easier filtering:

```
web-nginx       45 MB   +   stack:web
web-redis       12 MB   +   stack:web
web-postgres    890 MB  +   stack:web, database
```

This produces a single snapshot tagged with both `stack:web` and `database`, making it easy to reason about the state of the entire stack at a point in time.

## Custom Volume Paths

Some Docker installations use a non-standard data root. Find yours with:

```bash
docker info --format '{{ .DockerRootDir }}'
```

Set `docker_volumes_path` to the `volumes` subdirectory of that root. Then run discovery again.
