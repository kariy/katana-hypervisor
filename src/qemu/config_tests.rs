#[cfg(test)]
mod tests {
    use super::super::{QemuConfig, SevSnpConfig};
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
        assert!(args.contains(&"-nographic".to_string()));
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
}
