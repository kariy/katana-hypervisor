use crate::{qemu::QemuConfig, HypervisorError, Result};
use std::fs;
use std::process::{Command, Stdio};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;

/// Represents a single QEMU VM instance with its configuration and state.
///
/// This struct encapsulates all information about a VM and provides instance methods
/// for lifecycle operations. Unlike `VmManager`, which is a stateless manager for VMs,
/// `Vm` maintains the state of a specific VM instance.
///
/// # Lifecycle and Cleanup
///
/// **IMPORTANT**: The `Vm` struct does NOT automatically stop the VM when dropped. QEMU
/// processes run independently in daemon mode and will continue running even after the
/// `Vm` instance is dropped or goes out of scope.
///
/// ## Recommended Cleanup Pattern
///
/// **Always explicitly stop the VM** before dropping:
///
/// ```no_run
/// # use katana_core::qemu::{QemuConfig, Vm};
/// # use std::path::PathBuf;
/// # fn example(config: QemuConfig) -> katana_core::Result<()> {
/// let mut vm = Vm::new(config);
/// vm.launch()?;
///
/// // ... perform operations ...
///
/// // Explicitly stop before dropping (RECOMMENDED)
/// vm.stop(10)?;  // Graceful shutdown with 10 second timeout
/// # Ok(())
/// # }
/// ```
///
/// ## What Happens If You Forget to Stop
///
/// If a `Vm` instance is dropped while the VM is still running:
/// - The QEMU process continues running in the background
/// - The PID is lost (unless you tracked it separately)
/// - You'll need to manually find and kill the process (e.g., via `ps` or PID file)
/// - System resources remain allocated
///
/// ```no_run
/// # use katana_core::qemu::{QemuConfig, Vm};
/// # fn example(config: QemuConfig) -> katana_core::Result<()> {
/// let mut vm = Vm::new(config);
/// vm.launch()?;
///
/// // BAD: Dropping without stopping
/// drop(vm);  // VM still running in background as orphaned process!
/// # Ok(())
/// # }
/// ```
///
/// ## Design Rationale
///
/// The `Drop` implementation intentionally does not stop the VM because:
/// 1. **Explicit is better than implicit** - VM shutdown is a significant operation
/// 2. **Daemon mode** - QEMU runs independently and may outlive the parent process
/// 3. **Error handling** - Stopping a VM can fail and Drop cannot return Result
/// 4. **Intent preservation** - You may want the VM to continue running after drop
///
/// ## Recovery from Lost Instances
///
/// If you lose the `Vm` instance but need to stop the VM, you can:
///
/// ```no_run
/// # use katana_core::qemu::{QemuConfig, Vm};
/// # use std::path::PathBuf;
/// # fn example(config: QemuConfig) -> katana_core::Result<()> {
/// // Reattach using PID from file or other source
/// let pid_str = std::fs::read_to_string("/tmp/qemu.pid")?;
/// let pid: i32 = pid_str.trim().parse()
///     .map_err(|e| katana_core::HypervisorError::QemuFailed(format!("Invalid PID: {}", e)))?;
/// let mut vm = Vm::from_running(config, pid);
/// vm.stop(10)?;
/// # Ok(())
/// # }
/// ```
///
/// # Examples
///
/// ## Basic Usage
///
/// ```no_run
/// use katana_core::qemu::{QemuConfig, Vm};
/// use std::path::PathBuf;
///
/// # fn example() -> katana_core::Result<()> {
/// let config = QemuConfig {
///     memory_mb: 2048,
///     vcpus: 2,
///     cpu_type: "host".to_string(),
///     kernel_path: PathBuf::from("/path/to/kernel"),
///     initrd_path: PathBuf::from("/path/to/initrd"),
///     bios_path: None,
///     kernel_cmdline: "console=ttyS0".to_string(),
///     rpc_port: 5050,
///     disk_image: None,
///     qmp_socket: PathBuf::from("/tmp/qmp.sock"),
///     serial_log: PathBuf::from("/tmp/serial.log"),
///     pid_file: PathBuf::from("/tmp/qemu.pid"),
///     sev_snp: None,
///     enable_kvm: true,
/// };
///
/// let mut vm = Vm::new(config);
/// vm.launch()?;
///
/// // Perform operations
/// vm.pause()?;
/// vm.resume()?;
///
/// // Always clean up explicitly
/// vm.stop(10)?;
/// # Ok(())
/// # }
/// ```
///
/// ## Using RAII Pattern with Result
///
/// ```no_run
/// # use katana_core::qemu::{QemuConfig, Vm};
/// # fn example(config: QemuConfig) -> katana_core::Result<()> {
/// fn run_vm(config: QemuConfig) -> katana_core::Result<()> {
///     let mut vm = Vm::new(config);
///     vm.launch()?;
///
///     // Do work...
///
///     // Cleanup happens here before function returns
///     vm.stop(10)?;
///     Ok(())
/// }
///
/// // If run_vm returns Err, VM may still be running
/// if let Err(e) = run_vm(config) {
///     eprintln!("VM failed: {}. You may need to clean up manually.", e);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct Vm {
    /// Configuration for this VM instance
    config: QemuConfig,

    /// Process ID of the running QEMU process, None if not launched
    pid: Option<i32>,
}

impl Vm {
    /// Create a new VM instance with the given configuration.
    ///
    /// The VM is not launched automatically. Call `launch()` to start it.
    ///
    /// # Parameters
    /// - `config`: QEMU configuration for this VM
    pub fn new(config: QemuConfig) -> Self {
        Self {
            config,
            pid: None,
        }
    }

    /// Create a VM instance from an already running QEMU process.
    ///
    /// This is useful when you need to attach to a VM that was launched externally
    /// or when recovering from a restart.
    ///
    /// # Parameters
    /// - `config`: QEMU configuration for this VM
    /// - `pid`: Process ID of the running QEMU process
    ///
    /// # Note
    /// This does not verify that the process is actually running or that it matches
    /// the configuration. Use `is_running()` to verify.
    pub fn from_running(config: QemuConfig, pid: i32) -> Self {
        Self {
            config,
            pid: Some(pid),
        }
    }

    /// Launch this VM with its configured settings.
    ///
    /// This spawns a QEMU process in daemon mode and waits for the PID file to be written.
    /// After successful launch, the VM's PID is stored and can be retrieved via `pid()`.
    ///
    /// # Errors
    /// - If QEMU launch fails
    /// - If PID file cannot be read
    /// - If the VM is already running
    pub fn launch(&mut self) -> Result<()> {
        if self.pid.is_some() {
            return Err(HypervisorError::QemuFailed(
                "VM is already running".to_string(),
            ));
        }

        // Build QEMU command line
        let args = self.config.to_qemu_args();

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

        let pid = self.read_pid_file()?;
        self.pid = Some(pid);

        tracing::info!("QEMU VM launched with PID: {}", pid);

        Ok(())
    }

    /// Stop this VM gracefully via SIGTERM.
    ///
    /// Sends SIGTERM to the QEMU process and waits up to `timeout_secs` for it to exit.
    /// If the VM doesn't stop within the timeout, it will be force-killed with SIGKILL.
    ///
    /// # Parameters
    /// - `timeout_secs`: Maximum seconds to wait for graceful shutdown
    ///
    /// # Errors
    /// - If the VM is not running
    /// - If signal sending fails
    pub fn stop(&mut self, timeout_secs: u64) -> Result<()> {
        let pid = self.require_pid()?;

        tracing::info!("Stopping VM with PID: {}", pid);

        // Send SIGTERM for graceful shutdown
        kill(Pid::from_raw(pid), Signal::SIGTERM)
            .map_err(|e| HypervisorError::QemuFailed(format!("Failed to send SIGTERM: {}", e)))?;

        // Wait for process to exit
        let start = std::time::Instant::now();
        while start.elapsed().as_secs() < timeout_secs {
            if !self.is_running() {
                tracing::info!("VM stopped gracefully");
                self.pid = None;
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        // If still running, force kill
        tracing::warn!("VM did not stop gracefully, sending SIGKILL");
        self.kill()?;

        Ok(())
    }

    /// Force kill this VM with SIGKILL.
    ///
    /// This immediately terminates the QEMU process without graceful shutdown.
    /// Use `stop()` for graceful shutdown instead.
    ///
    /// # Warning
    /// This may cause data loss or corruption in the guest OS.
    ///
    /// # Errors
    /// - If the VM is not running
    /// - If signal sending fails
    pub fn kill(&mut self) -> Result<()> {
        let pid = self.require_pid()?;

        tracing::info!("Force killing VM with PID: {}", pid);

        kill(Pid::from_raw(pid), Signal::SIGKILL)
            .map_err(|e| HypervisorError::QemuFailed(format!("Failed to send SIGKILL: {}", e)))?;

        // Wait a bit to ensure process is dead
        std::thread::sleep(std::time::Duration::from_millis(200));

        self.pid = None;

        Ok(())
    }

    /// Check if this VM is currently running.
    ///
    /// Uses signal 0 to check if the process exists without actually sending a signal.
    ///
    /// # Returns
    /// - `true` if the VM process is running
    /// - `false` if the VM was never launched or has stopped
    pub fn is_running(&self) -> bool {
        match self.pid {
            Some(pid) => kill(Pid::from_raw(pid), None).is_ok(),
            None => false,
        }
    }

    /// Pause VM execution (freeze vCPUs).
    ///
    /// Connects to the VM's QMP socket and sends a stop command to freeze vCPU execution.
    /// Memory and device state are preserved. The guest OS is unaware of the pause.
    ///
    /// For detailed information about resource effects, behavior, and use cases,
    /// see [`QmpClient::stop()`](crate::qemu::QmpClient::stop).
    ///
    /// # Errors
    /// - If the VM is not running
    /// - If QMP socket connection fails
    /// - If QMP command fails
    pub fn pause(&self) -> Result<()> {
        self.require_pid()?;

        tracing::info!("Pausing VM via QMP");

        let mut qmp_client = crate::qemu::QmpClient::new();
        qmp_client.connect(&self.config.qmp_socket)?;
        qmp_client.stop()?;

        tracing::info!("VM paused successfully");
        Ok(())
    }

    /// Resume VM execution (unfreeze vCPUs).
    ///
    /// Connects to the VM's QMP socket and sends a continue command to resume vCPU execution.
    /// This restores execution after a pause.
    ///
    /// For detailed information about resource effects, behavior, and use cases,
    /// see [`QmpClient::cont()`](crate::qemu::QmpClient::cont).
    ///
    /// # Errors
    /// - If the VM is not running
    /// - If QMP socket connection fails
    /// - If QMP command fails
    pub fn resume(&self) -> Result<()> {
        self.require_pid()?;

        tracing::info!("Resuming VM via QMP");

        let mut qmp_client = crate::qemu::QmpClient::new();
        qmp_client.connect(&self.config.qmp_socket)?;
        qmp_client.cont()?;

        tracing::info!("VM resumed successfully");
        Ok(())
    }

    /// Suspend VM to RAM (ACPI S3 sleep).
    ///
    /// Connects to the VM's QMP socket and triggers an ACPI S3 suspend. This is a
    /// guest-cooperative operation where the guest OS participates in the suspend sequence.
    ///
    /// For detailed information about ACPI requirements, resource effects, and use cases,
    /// see [`QmpClient::system_suspend()`](crate::qemu::QmpClient::system_suspend).
    ///
    /// # Errors
    /// - If the VM is not running
    /// - If QMP socket connection fails
    /// - If QMP command fails (especially if guest lacks ACPI support)
    pub fn suspend(&self) -> Result<()> {
        self.require_pid()?;

        tracing::info!("Suspending VM via QMP");

        let mut qmp_client = crate::qemu::QmpClient::new();
        qmp_client.connect(&self.config.qmp_socket)?;
        qmp_client.system_suspend()?;

        tracing::info!("VM suspend command sent");
        Ok(())
    }

    /// Wake VM from suspend (ACPI wakeup).
    ///
    /// Connects to the VM's QMP socket and triggers an ACPI wakeup event. This brings
    /// a suspended VM back to running state through the guest's ACPI resume handlers.
    ///
    /// For detailed information about ACPI requirements, resource effects, and use cases,
    /// see [`QmpClient::system_wakeup()`](crate::qemu::QmpClient::system_wakeup).
    ///
    /// # Errors
    /// - If the VM is not running
    /// - If QMP socket connection fails
    /// - If QMP command fails (especially if VM is not in suspended state)
    pub fn wake(&self) -> Result<()> {
        self.require_pid()?;

        tracing::info!("Waking VM via QMP");

        let mut qmp_client = crate::qemu::QmpClient::new();
        qmp_client.connect(&self.config.qmp_socket)?;
        qmp_client.system_wakeup()?;

        tracing::info!("VM wakeup command sent");
        Ok(())
    }

    /// Reset VM (hard reboot).
    ///
    /// Connects to the VM's QMP socket and triggers a hard reset without graceful shutdown.
    /// This is equivalent to pressing a physical reset button.
    ///
    /// For detailed information about risks, resource effects, and use cases,
    /// see [`QmpClient::system_reset()`](crate::qemu::QmpClient::system_reset).
    ///
    /// # Warning
    /// This is a hard reset without graceful shutdown. May cause data loss or corruption.
    ///
    /// # Errors
    /// - If the VM is not running
    /// - If QMP socket connection fails
    /// - If QMP command fails
    pub fn reset(&self) -> Result<()> {
        self.require_pid()?;

        tracing::info!("Resetting VM via QMP");

        let mut qmp_client = crate::qemu::QmpClient::new();
        qmp_client.connect(&self.config.qmp_socket)?;
        qmp_client.system_reset()?;

        tracing::info!("VM reset command sent");
        Ok(())
    }

    /// Get the process ID of this VM.
    ///
    /// # Returns
    /// - `Some(pid)` if the VM has been launched
    /// - `None` if the VM has not been launched or has been stopped
    pub fn pid(&self) -> Option<i32> {
        self.pid
    }

    /// Get a reference to this VM's configuration.
    pub fn config(&self) -> &QemuConfig {
        &self.config
    }

    /// Get the QMP socket path for this VM.
    pub fn qmp_socket(&self) -> &std::path::Path {
        &self.config.qmp_socket
    }

    /// Get the PID file path for this VM.
    pub fn pid_file(&self) -> &std::path::Path {
        &self.config.pid_file
    }

    /// Get the serial log path for this VM.
    pub fn serial_log(&self) -> &std::path::Path {
        &self.config.serial_log
    }

    /// Helper to require a PID, returning an error if not launched.
    fn require_pid(&self) -> Result<i32> {
        self.pid.ok_or_else(|| {
            HypervisorError::QemuFailed("VM is not running".to_string())
        })
    }

    /// Read PID from the VM's PID file.
    fn read_pid_file(&self) -> Result<i32> {
        let pid_file = &self.config.pid_file;

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

impl Drop for Vm {
    /// Cleanup when the Vm instance is dropped.
    ///
    /// Note: This does NOT automatically stop the VM, as the QEMU process runs
    /// independently in daemon mode. Use `stop()` or `kill()` explicitly to
    /// terminate the VM before dropping.
    fn drop(&mut self) {
        // We intentionally don't stop the VM here, as it may be running independently
        // and the user may want to keep it running. Explicit cleanup is preferred.
        if self.pid.is_some() {
            tracing::debug!(
                "Vm instance dropped while VM (PID: {:?}) is still running. \
                VM will continue running in background.",
                self.pid
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_config() -> QemuConfig {
        QemuConfig {
            memory_mb: 2048,
            vcpus: 2,
            cpu_type: "host".to_string(),
            kernel_path: PathBuf::from("/test/vmlinuz"),
            initrd_path: PathBuf::from("/test/initrd.img"),
            bios_path: None,
            kernel_cmdline: "console=ttyS0".to_string(),
            rpc_port: 5050,
            disk_image: None,
            qmp_socket: PathBuf::from("/tmp/qmp.sock"),
            serial_log: PathBuf::from("/tmp/serial.log"),
            pid_file: PathBuf::from("/tmp/qemu.pid"),
            sev_snp: None,
            enable_kvm: true,
        }
    }

    #[test]
    fn test_new_vm() {
        let config = create_test_config();
        let vm = Vm::new(config);

        assert!(vm.pid().is_none());
        assert!(!vm.is_running());
    }

    #[test]
    fn test_from_running() {
        let config = create_test_config();
        let vm = Vm::from_running(config, 12345);

        assert_eq!(vm.pid(), Some(12345));
    }

    #[test]
    fn test_accessors() {
        let config = create_test_config();
        let vm = Vm::new(config);

        assert_eq!(vm.config().memory_mb, 2048);
        assert_eq!(vm.config().vcpus, 2);
        assert_eq!(vm.qmp_socket(), std::path::Path::new("/tmp/qmp.sock"));
        assert_eq!(vm.pid_file(), std::path::Path::new("/tmp/qemu.pid"));
        assert_eq!(vm.serial_log(), std::path::Path::new("/tmp/serial.log"));
    }

    #[test]
    fn test_require_pid_fails_when_not_running() {
        let config = create_test_config();
        let vm = Vm::new(config);

        let result = vm.require_pid();
        assert!(result.is_err());
    }
}
