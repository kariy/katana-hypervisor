use crate::{HypervisorError, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub struct StorageManager {
    base_dir: PathBuf,
}

impl StorageManager {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// Create storage directory for an instance
    pub fn create_instance_storage(&self, instance_id: &str, _quota_bytes: u64) -> Result<PathBuf> {
        let instance_dir = self.base_dir.join(instance_id);

        // Create instance directory
        fs::create_dir_all(&instance_dir)?;

        // Create data subdirectory (for katana's database)
        let data_dir = instance_dir.join("data");
        fs::create_dir_all(&data_dir)?;

        // TODO: Set filesystem quota on Linux
        // For now, we'll just track usage and warn

        Ok(instance_dir)
    }

    /// Get the instance directory path
    pub fn get_instance_dir(&self, instance_id: &str) -> PathBuf {
        self.base_dir.join(instance_id)
    }

    /// Get disk usage for an instance
    pub fn get_disk_usage(&self, instance_id: &str) -> Result<u64> {
        let instance_dir = self.base_dir.join(instance_id);

        if !instance_dir.exists() {
            return Ok(0);
        }

        calculate_dir_size(&instance_dir)
    }

    /// Check if storage quota is exceeded
    pub fn check_quota(&self, instance_id: &str, quota_bytes: u64) -> Result<()> {
        let usage = self.get_disk_usage(instance_id)?;

        if usage > quota_bytes {
            return Err(HypervisorError::StorageQuotaExceeded {
                used: usage,
                limit: quota_bytes,
            });
        }

        Ok(())
    }

    /// Delete instance storage
    pub fn delete_instance_storage(&self, instance_id: &str) -> Result<()> {
        let instance_dir = self.base_dir.join(instance_id);

        if instance_dir.exists() {
            fs::remove_dir_all(&instance_dir)?;
        }

        Ok(())
    }

    /// Get paths for instance files
    pub fn get_paths(&self, instance_id: &str) -> InstancePaths {
        let instance_dir = self.base_dir.join(instance_id);

        InstancePaths {
            instance_dir: instance_dir.clone(),
            data_dir: instance_dir.join("data"),
            serial_log: instance_dir.join("serial.log"),
            qmp_socket: instance_dir.join("qmp.sock"),
            pid_file: instance_dir.join("qemu.pid"),
        }
    }
}

pub struct InstancePaths {
    pub instance_dir: PathBuf,
    pub data_dir: PathBuf,
    pub serial_log: PathBuf,
    pub qmp_socket: PathBuf,
    pub pid_file: PathBuf,
}

/// Calculate total size of a directory recursively
fn calculate_dir_size(path: &Path) -> Result<u64> {
    let mut total_size = 0u64;

    if path.is_file() {
        return Ok(fs::metadata(path)?.len());
    }

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();

            if entry_path.is_file() {
                total_size += fs::metadata(&entry_path)?.len();
            } else if entry_path.is_dir() {
                total_size += calculate_dir_size(&entry_path)?;
            }
        }
    }

    Ok(total_size)
}

#[cfg(test)]
#[path = "storage_tests.rs"]
mod storage_tests;
