# Sage Restic Manager

A production-grade terminal backup management system for [Restic](https://restic.net/), built for self-hosted servers and headless environments.

## Overview

Sage Restic Manager wraps Restic in an ergonomic interface that handles the operational complexity of running scheduled backups on servers. It provides a keyboard-driven TUI for daily operations and a complete CLI for automation, with first-class support for Docker volume backups.

## Features

- **Text User Interface (TUI)** - Navigate backup sources, repository status, snapshots, scheduling, and logs with keyboard shortcuts
- **Docker Volume Discovery** - Automatically scans `/var/lib/docker/volumes` and presents named volumes as backup candidates
- **Systemd Timer Integration** - Generates and installs systemd service and timer units for reliable scheduling
- **Secure Credential Storage** - Prioritizes the OS secret service (keyring) for credentials, falling back to permission-restricted files
- **Storage Budget Management** - Tracks repository growth and warns when approaching configurable size limits
- **Self-Updating Binary** - Verifies SHA256 checksums and minisign signatures before replacing itself
- **Multi-Backend Support** - Local filesystem, S3-compatible, and Backblaze B2 repositories

## Quick Start

### Prerequisites

- Linux with systemd (tested on Ubuntu 24.04)
- Restic >= 0.16 installed and available on PATH
- A running OS secret service (GNOME Keyring, KDE Wallet, or KeepassXC) for secure credential storage

### Install with curl (recommended)

The one-line installer detects your platform, downloads the correct binary, verifies the SHA256 checksum, and installs to `/usr/local/bin` (or `~/.local/bin` if root is unavailable):

```bash
curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/sage-restic-manager/main/install.sh | bash
```

Install a specific version:

```bash
curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/sage-restic-manager/main/install.sh | bash -s -- v0.1.1
```

### Build from Source

```bash
git clone https://github.com/theadeyemiolayinka/sage-restic-manager.git
cd sage-restic-manager
cargo build --release
sudo cp target/release/sage-restic-manager /usr/local/bin/
```

### Run

```bash
sage-restic-manager
```

Without arguments, the application opens the TUI. Use `Tab` or number keys `1` through `8` to switch screens.

## Documentation

Full documentation is available at [the GitHub Pages site](https://theadeyemiolayinka.github.io/sage-restic-manager/).

### Deploying Documentation

Documentation is built with [MkDocs Material](https://squidfunk.github.io/mkdocs-material/) and deployed automatically via GitHub Actions.

**Required setup:**

1. Push all files to the `main` branch
2. Go to **Repository Settings -> Pages**
3. Set **Source** to **GitHub Actions**
4. Push any change to `main` or trigger the workflow manually. The site will deploy to `https://<username>.github.io/sage-restic-manager/`

The `.github/workflows/docs.yml` workflow handles building and deployment. No local tools are needed.

## License

MIT
