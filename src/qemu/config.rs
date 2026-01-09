use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct QemuConfig {
    // Resource limits
    pub memory_mb: u64,
    pub vcpus: u32,
    pub cpu_type: String,

    // Boot components
    pub kernel_path: PathBuf,
    pub initrd_path: PathBuf,
    pub bios_path: Option<PathBuf>,

    // Kernel command line
    pub kernel_cmdline: String,

    // Network
    pub rpc_port: u16,

    // Storage
    pub disk_image: Option<PathBuf>,

    // Paths
    pub qmp_socket: PathBuf,
    pub serial_log: PathBuf,
    pub pid_file: PathBuf,

    // TEE configuration
    pub sev_snp: Option<SevSnpConfig>,

    // Enable KVM acceleration
    pub enable_kvm: bool,
}

#[derive(Debug, Clone)]
pub struct SevSnpConfig {
    pub cbitpos: u8,
    pub reduced_phys_bits: u8,
    pub vcpu_type: String,
}

impl QemuConfig {
    /// Build QEMU command line arguments
    pub fn to_qemu_args(&self) -> Vec<String> {
        let mut args = vec!["qemu-system-x86_64".to_string()];

        // Enable KVM if requested
        if self.enable_kvm {
            args.push("-enable-kvm".to_string());
        }

        // CPU configuration
        if let Some(ref sev_snp) = self.sev_snp {
            // SEV-SNP mode
            args.push("-cpu".to_string());
            args.push(sev_snp.vcpu_type.clone());

            // Machine type with confidential guest support
            args.push("-machine".to_string());
            args.push("q35,confidential-guest-support=sev0".to_string());

            // SEV-SNP guest object
            args.push("-object".to_string());
            args.push(format!(
                "sev-snp-guest,id=sev0,cbitpos={},reduced-phys-bits={}",
                sev_snp.cbitpos, sev_snp.reduced_phys_bits
            ));

            // BIOS (OVMF) is required for SEV
            if let Some(ref bios_path) = self.bios_path {
                args.push("-bios".to_string());
                args.push(bios_path.to_string_lossy().to_string());
            }
        } else {
            // Non-TEE mode
            args.push("-cpu".to_string());
            args.push(self.cpu_type.clone());

            args.push("-machine".to_string());
            args.push("q35".to_string());
        }

        // Memory and vCPUs
        args.push("-smp".to_string());
        args.push(self.vcpus.to_string());

        args.push("-m".to_string());
        args.push(format!("{}M", self.memory_mb));

        // Kernel boot
        args.push("-kernel".to_string());
        args.push(self.kernel_path.to_string_lossy().to_string());

        args.push("-initrd".to_string());
        args.push(self.initrd_path.to_string_lossy().to_string());

        args.push("-append".to_string());
        args.push(self.kernel_cmdline.clone());

        // Network - user networking with port forwarding
        args.push("-netdev".to_string());
        args.push(format!("user,id=net0,hostfwd=tcp::{}-:5050", self.rpc_port));

        args.push("-device".to_string());
        args.push("virtio-net-pci,netdev=net0".to_string());

        // Storage - virtio-blk disk image
        if let Some(ref disk_path) = self.disk_image {
            args.push("-drive".to_string());
            args.push(format!(
                "file={},if=virtio,format=qcow2",
                disk_path.to_string_lossy()
            ));
        }

        // No graphics (use -display none instead of -nographic for compatibility with -daemonize)
        args.push("-display".to_string());
        args.push("none".to_string());

        // Serial console to file
        args.push("-serial".to_string());
        args.push(format!("file:{}", self.serial_log.to_string_lossy()));

        // QMP socket
        args.push("-qmp".to_string());
        args.push(format!(
            "unix:{},server,nowait",
            self.qmp_socket.to_string_lossy()
        ));

        // Daemonize
        args.push("-daemonize".to_string());

        // PID file
        args.push("-pidfile".to_string());
        args.push(self.pid_file.to_string_lossy().to_string());

        args
    }

    /// Build kernel command line with katana arguments
    pub fn build_kernel_cmdline(katana_args: &[String]) -> String {
        let katana_args_str = katana_args.join(" ");

        format!("console=ttyS0 loglevel=4 katana.args={}", katana_args_str)
    }
}

#[cfg(test)]
mod tests {
    use super::{QemuConfig, SevSnpConfig};
    use std::path::PathBuf;

    fn create_test_config() -> QemuConfig {
        QemuConfig {
            memory_mb: 4096,
            vcpus: 4,
            cpu_type: "host".to_string(),
            kernel_path: PathBuf::from("/test/vmlinuz"),
            initrd_path: PathBuf::from("/test/initrd.img"),
            bios_path: None,
            kernel_cmdline: "console=ttyS0".to_string(),
            rpc_port: 5050,
            qmp_socket: PathBuf::from("/tmp/qmp.sock"),
            serial_log: PathBuf::from("/tmp/serial.log"),
            pid_file: PathBuf::from("/tmp/qemu.pid"),
            sev_snp: None,
            enable_kvm: true,
        }
    }

    #[test]
    fn test_non_tee_qemu_args() {
        let config = create_test_config();
        let args = config.to_qemu_args();

        // Verify essential arguments
        assert!(args.contains(&"qemu-system-x86_64".to_string()));
        assert!(args.contains(&"-enable-kvm".to_string()));
        assert!(args.contains(&"-cpu".to_string()));
        assert!(args.contains(&"host".to_string()));
        assert!(args.contains(&"-smp".to_string()));
        assert!(args.contains(&"4".to_string()));
        assert!(args.contains(&"-m".to_string()));
        assert!(args.contains(&"4096M".to_string()));
        assert!(args.contains(&"-machine".to_string()));
        assert!(args.contains(&"q35".to_string()));
    }

    #[test]
    fn test_kernel_boot_args() {
        let config = create_test_config();
        let args = config.to_qemu_args();

        assert!(args.contains(&"-kernel".to_string()));
        assert!(args.contains(&"/test/vmlinuz".to_string()));
        assert!(args.contains(&"-initrd".to_string()));
        assert!(args.contains(&"/test/initrd.img".to_string()));
        assert!(args.contains(&"-append".to_string()));
        assert!(args.contains(&"console=ttyS0".to_string()));
    }

    #[test]
    fn test_networking_args() {
        let config = create_test_config();
        let args = config.to_qemu_args();

        assert!(args.contains(&"-netdev".to_string()));
        assert!(args.contains(&"user,id=net0,hostfwd=tcp::5050-:5050".to_string()));
        assert!(args.contains(&"-device".to_string()));
        assert!(args.contains(&"virtio-net-pci,netdev=net0".to_string()));
    }

    #[test]
    fn test_serial_and_qmp_args() {
        let config = create_test_config();
        let args = config.to_qemu_args();

        assert!(args.contains(&"-serial".to_string()));
        assert!(args.contains(&"file:/tmp/serial.log".to_string()));
        assert!(args.contains(&"-qmp".to_string()));
        assert!(args.contains(&"unix:/tmp/qmp.sock,server,nowait".to_string()));
        assert!(args.contains(&"-display".to_string()));
        assert!(args.contains(&"none".to_string()));
    }

    #[test]
    fn test_daemonize_and_pid() {
        let config = create_test_config();
        let args = config.to_qemu_args();

        assert!(args.contains(&"-daemonize".to_string()));
        assert!(args.contains(&"-pidfile".to_string()));
        assert!(args.contains(&"/tmp/qemu.pid".to_string()));
    }

    #[test]
    fn test_sev_snp_config() {
        let mut config = create_test_config();
        config.cpu_type = "EPYC-v4".to_string();
        config.sev_snp = Some(SevSnpConfig {
            cbitpos: 51,
            reduced_phys_bits: 1,
            vcpu_type: "EPYC-v4".to_string(),
        });
        config.bios_path = Some(PathBuf::from("/test/ovmf.fd"));

        let args = config.to_qemu_args();

        // Verify SEV-SNP specific args
        assert!(args.contains(&"-cpu".to_string()));
        assert!(args.contains(&"EPYC-v4".to_string()));
        assert!(args.contains(&"-machine".to_string()));
        assert!(args.contains(&"q35,confidential-guest-support=sev0".to_string()));
        assert!(args.contains(&"-object".to_string()));
        assert!(args.contains(&"sev-snp-guest,id=sev0,cbitpos=51,reduced-phys-bits=1".to_string()));
        assert!(args.contains(&"-bios".to_string()));
        assert!(args.contains(&"/test/ovmf.fd".to_string()));
    }

    #[test]
    fn test_build_kernel_cmdline() {
        let katana_args = vec![
            "--http.addr=0.0.0.0".to_string(),
            "--http.port=5050".to_string(),
            "--dev".to_string(),
        ];

        let cmdline = QemuConfig::build_kernel_cmdline(&katana_args);

        assert!(cmdline.contains("console=ttyS0"));
        assert!(cmdline.contains("loglevel=4"));
        assert!(cmdline.contains("katana.args="));
        assert!(cmdline.contains("--http.addr=0.0.0.0"));
        assert!(cmdline.contains("--http.port=5050"));
        assert!(cmdline.contains("--dev"));
    }

    #[test]
    fn test_custom_memory_sizes() {
        let mut config = create_test_config();

        // Test different memory sizes
        config.memory_mb = 512;
        let args = config.to_qemu_args();
        assert!(args.contains(&"512M".to_string()));

        config.memory_mb = 8192;
        let args = config.to_qemu_args();
        assert!(args.contains(&"8192M".to_string()));
    }

    #[test]
    fn test_custom_vcpu_counts() {
        let mut config = create_test_config();

        config.vcpus = 1;
        let args = config.to_qemu_args();
        assert!(args.contains(&"1".to_string()));

        config.vcpus = 8;
        let args = config.to_qemu_args();
        assert!(args.contains(&"8".to_string()));
    }

    #[test]
    fn test_custom_rpc_port() {
        let mut config = create_test_config();
        config.rpc_port = 8080;

        let args = config.to_qemu_args();
        assert!(args.contains(&"user,id=net0,hostfwd=tcp::8080-:5050".to_string()));
    }

    #[test]
    fn test_no_kvm_mode() {
        let mut config = create_test_config();
        config.enable_kvm = false;

        let args = config.to_qemu_args();
        assert!(!args.contains(&"-enable-kvm".to_string()));
    }

    #[test]
    fn test_tee_mode_kernel_cmdline() {
        let katana_args = vec![
            "--http.addr=0.0.0.0".to_string(),
            "--http.port=5050".to_string(),
            "--tee.provider".to_string(),
            "sev-snp".to_string(),
        ];

        let cmdline = QemuConfig::build_kernel_cmdline(&katana_args);

        assert!(cmdline.contains("console=ttyS0"));
        assert!(cmdline.contains("loglevel=4"));
        assert!(cmdline.contains("katana.args="));
        assert!(cmdline.contains("--http.addr=0.0.0.0"));
        assert!(cmdline.contains("--http.port=5050"));
        assert!(cmdline.contains("--tee.provider"));
        assert!(cmdline.contains("sev-snp"));
    }
}
