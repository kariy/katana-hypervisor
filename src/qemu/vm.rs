use crate::{qemu::QemuConfig, HypervisorError, Result};
use std::process::{Command, Stdio};
use std::fs;
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;

pub struct VmManager;

impl VmManager {
    pub fn new() -> Self {
        Self
    }

    /// Launch a QEMU VM with the given configuration
    pub fn launch_vm(&self, config: &QemuConfig) -> Result<i32> {
        // Build QEMU command line
        let args = config.to_qemu_args();

        tracing::info!("Launching QEMU VM with command: {:?}", args);

        // Execute QEMU
        let output = Command::new(&args[0])
            .args(&args[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
            .wait_with_output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(HypervisorError::QemuFailed(format!(
                "QEMU launch failed: {}",
                stderr
            )));
        }

        // Read PID from PID file
        // Wait a bit for QEMU to write the PID file
        std::thread::sleep(std::time::Duration::from_millis(500));

        let pid = self.read_pid_file(&config.pid_file)?;

        tracing::info!("QEMU VM launched with PID: {}", pid);

        Ok(pid)
    }

    /// Stop a VM gracefully via signal
    pub fn stop_vm(&self, pid: i32, timeout_secs: u64) -> Result<()> {
        tracing::info!("Stopping VM with PID: {}", pid);

        // Send SIGTERM for graceful shutdown
        kill(Pid::from_raw(pid), Signal::SIGTERM)
            .map_err(|e| HypervisorError::QemuFailed(format!("Failed to send SIGTERM: {}", e)))?;

        // Wait for process to exit
        let start = std::time::Instant::now();
        while start.elapsed().as_secs() < timeout_secs {
            if !self.is_process_running(pid) {
                tracing::info!("VM stopped gracefully");
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        // If still running, force kill
        tracing::warn!("VM did not stop gracefully, sending SIGKILL");
        self.kill_vm(pid)?;

        Ok(())
    }

    /// Force kill a VM
    pub fn kill_vm(&self, pid: i32) -> Result<()> {
        tracing::info!("Force killing VM with PID: {}", pid);

        kill(Pid::from_raw(pid), Signal::SIGKILL)
            .map_err(|e| HypervisorError::QemuFailed(format!("Failed to send SIGKILL: {}", e)))?;

        // Wait a bit to ensure process is dead
        std::thread::sleep(std::time::Duration::from_millis(200));

        Ok(())
    }

    /// Check if a process is running
    pub fn is_process_running(&self, pid: i32) -> bool {
        // Try to send signal 0 (does not actually send a signal, just checks if process exists)
        kill(Pid::from_raw(pid), None).is_ok()
    }

    /// Read PID from PID file
    fn read_pid_file(&self, pid_file: &std::path::Path) -> Result<i32> {
        if !pid_file.exists() {
            return Err(HypervisorError::QemuFailed(
                "PID file not found".to_string(),
            ));
        }

        let pid_str = fs::read_to_string(pid_file)?;
        let pid: i32 = pid_str
            .trim()
            .parse()
            .map_err(|e| HypervisorError::QemuFailed(format!("Invalid PID in file: {}", e)))?;

        Ok(pid)
    }
}
