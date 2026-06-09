use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, warn};

use crate::config::sources::{BackupSource, ContainerOrigin};
use crate::error::Result;

pub enum DockerDiscoveryResult {
    Found {
        container: BackupSource,
        children: Vec<BackupSource>,
    },
    PathNotFound {
        searched: PathBuf,
    },
    PermissionDenied {
        path: PathBuf,
    },
}

#[allow(dead_code)]
pub enum ContainerScanResult {
    Ok {
        container_path: PathBuf,
        children: Vec<BackupSource>,
    },
    PathNotFound,
    PermissionDenied,
}

pub struct VolumeDiscovery;

impl VolumeDiscovery {
    pub async fn discover_docker_volumes(base: &PathBuf) -> DockerDiscoveryResult {
        if !base.exists() {
            debug!("Docker volumes path does not exist: {}", base.display());
            return DockerDiscoveryResult::PathNotFound { searched: base.clone() };
        }

        let mut entries = match fs::read_dir(base).await {
            Ok(e) => e,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    return DockerDiscoveryResult::PermissionDenied { path: base.clone() };
                }
                warn!("Cannot read docker volumes directory {}: {}", base.display(), e);
                return DockerDiscoveryResult::PathNotFound { searched: base.clone() };
            }
        };

        let label = base.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("volumes")
            .to_string();

        let mut container = BackupSource::new_container(
            base.clone(),
            label,
            ContainerOrigin::DockerVolumes,
        );
        container.state = crate::config::SourceState::Selected;

        let mut children = Vec::new();

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            if name.starts_with('.') {
                continue;
            }
            let data_path = path.join("_data");
            let actual_path = if data_path.exists() { data_path } else { path.clone() };
            let size = directory_size(&actual_path).await;
            let mut child = BackupSource::new_flat_child(
                actual_path,
                name,
                base.clone(),
            );
            child.size_bytes = size;
            children.push(child);
        }

        children.sort_by(|a, b| a.label.cmp(&b.label));

        DockerDiscoveryResult::Found { container, children }
    }

    pub async fn scan_container_children(container_path: &PathBuf) -> ContainerScanResult {
        if !container_path.exists() {
            return ContainerScanResult::PathNotFound;
        }

        let mut entries = match fs::read_dir(container_path).await {
            Ok(e) => e,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    return ContainerScanResult::PermissionDenied;
                }
                return ContainerScanResult::PathNotFound;
            }
        };

        let mut children = Vec::new();

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            if name.starts_with('.') {
                continue;
            }
            let size = directory_size(&path).await;
            let mut child = BackupSource::new_flat_child(
                path,
                name,
                container_path.clone(),
            );
            child.size_bytes = size;
            children.push(child);
        }

        children.sort_by(|a, b| a.label.cmp(&b.label));

        ContainerScanResult::Ok {
            container_path: container_path.clone(),
            children,
        }
    }

}

pub async fn directory_size(path: &Path) -> Option<u64> {
    if !path.exists() {
        return None;
    }
    match dir_size_recursive(path).await {
        Ok(size) => Some(size),
        Err(_) => None,
    }
}

async fn dir_size_recursive(path: &Path) -> Result<u64> {
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(current) = stack.pop() {
        let mut entries = match fs::read_dir(&current).await {
            Ok(e) => e,
            Err(_) => continue,
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let metadata = match entry.metadata().await {
                Ok(m) => m,
                Err(_) => continue,
            };
            if metadata.is_dir() {
                stack.push(entry.path());
            } else if metadata.is_file() {
                total += metadata.len();
            }
        }
    }
    Ok(total)
}
