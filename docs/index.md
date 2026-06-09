# Sage Restic Manager

A production-grade terminal backup management system for [Restic](https://restic.net/), built for self-hosted servers and headless environments. It provides a keyboard-driven TUI for daily operations and a complete CLI for automation, with first-class support for Docker volume backups.

## What It Is

Sage Restic Manager wraps Restic in an ergonomic interface that handles the operational complexity of running scheduled backups on servers. Rather than managing Restic commands, environment variables, and cron jobs manually, you configure everything through structured TOML files and a focused terminal interface.

The application is designed for operators who run Docker workloads on Ubuntu or similar systemd-based systems and need reliable, observable backups without a GUI.

## Key Features

- **Text User Interface (TUI)** - Navigate backup sources, repository status, snapshots, scheduling, and logs with keyboard shortcuts
- **Docker Volume Discovery** - Automatically scans `/var/lib/docker/volumes` and presents named volumes as backup candidates
- **Systemd Timer Integration** - Generates and installs systemd service and timer units for reliable scheduling
- **Secure Credential Storage** - Prioritizes the OS secret service (keyring) for credentials, falling back to permission-restricted files
- **Storage Budget Management** - Tracks repository growth and warns when approaching configurable size limits
- **Self-Updating Binary** - Verifies SHA256 checksums and minisign signatures before replacing itself
- **Progress Reporting** - Real-time backup progress in the TUI with file counts, byte counts, and duration
- **Multi-Backend Support** - Local filesystem, S3-compatible, and Backblaze B2 repositories

## Architecture

The application is structured into several domains:

- **Configuration** - TOML-based configs stored in `~/.config/sage-restic-manager/`
- **Restic Client** - Async command execution with JSON output parsing
- **Discovery** - Docker volume scanning and size estimation
- **Scheduler** - Systemd unit generation and lifecycle management
- **TUI** - Ratatui-based interface with async background tasks

## Requirements

- Linux with systemd (tested on Ubuntu 24.04)
- Restic >= 0.16 installed and available on PATH
- A running OS secret service (GNOME Keyring, KDE Wallet, or KeepassXC) for secure credential storage
- For Docker volume backup: root access or membership in the `docker` group

## Quick Links

- [Getting Started](getting-started.md) - Install and run your first backup
- [Docker Volume Backup](docker-volumes.md) - Back up named volumes from running containers
- [Configuration](configuration.md) - Reference for all TOML config files
