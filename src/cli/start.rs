use crate::{
    instance::InstanceStatus,
    qemu::{QemuConfig, VmManager},
    state::StateDatabase,
};
use anyhow::Result;

pub fn execute(name: &str, db: &StateDatabase, vm_manager: &VmManager) -> Result<()> {
    tracing::info!("Starting instance: {}", name);

    // Load instance from database
    let mut state = db.get_instance(name)?;

    // Check state
    match state.status {
        InstanceStatus::Running => {
            println!("Instance '{}' is already running", name);
            return Ok(());
        }
        InstanceStatus::Starting => {
            anyhow::bail!("Instance '{}' is already starting", name);
        }
        _ => {}
    }

    // Check if boot components exist BEFORE updating status
    if !state.config.kernel_path.exists() {
        anyhow::bail!(
            "Kernel not found at {}. Please build boot components first:\n  cd /home/ubuntu/katana && make build-tee",
            state.config.kernel_path.display()
        );
    }

    if !state.config.initrd_path.exists() {
        anyhow::bail!(
            "Initrd not found at {}. Please build boot components first:\n  cd /home/ubuntu/katana && make build-tee",
            state.config.initrd_path.display()
        );
    }

    // Build katana arguments
    let katana_args = state.config.build_katana_args();
    let kernel_cmdline = QemuConfig::build_kernel_cmdline(&katana_args);

    // Build SEV-SNP config if TEE mode is enabled
    let sev_snp_config = if state.config.tee_mode {
        Some(crate::qemu::config::SevSnpConfig {
            cbitpos: 51,           // C-bit position for AMD EPYC
            reduced_phys_bits: 1,  // Reserved physical address bits
            vcpu_type: state.config.vcpu_type.clone(),
        })
    } else {
        None
    };

    // Build QEMU configuration
    let qemu_config = QemuConfig {
        memory_mb: state.config.memory_mb,
        vcpus: state.config.vcpus,
        cpu_type: state.config.vcpu_type.clone(),
        kernel_path: state.config.kernel_path.clone(),
        initrd_path: state.config.initrd_path.clone(),
        bios_path: state.config.ovmf_path.clone(),
        kernel_cmdline,
        rpc_port: state.config.rpc_port,
        qmp_socket: state.qmp_socket.clone().unwrap(),
        serial_log: state.serial_log.clone().unwrap(),
        pid_file: std::path::PathBuf::from(format!(
            "/tmp/katana-hypervisor-{}.pid",
            state.id
        )),
        sev_snp: sev_snp_config,
        enable_kvm: true,
    };

    println!("Starting QEMU VM...");
    println!("  Kernel: {}", qemu_config.kernel_path.display());
    println!("  Initrd: {}", qemu_config.initrd_path.display());
    println!("  vCPUs: {}", qemu_config.vcpus);
    println!("  Memory: {} MB", qemu_config.memory_mb);
    println!("  RPC Port: {}", qemu_config.rpc_port);
    if state.config.tee_mode {
        println!("  TEE Mode: AMD SEV-SNP ({})", state.config.vcpu_type);
    }

    // Launch VM
    let pid = vm_manager.launch_vm(&qemu_config)?;

    // Update state
    state.vm_pid = Some(pid);
    state.update_status(InstanceStatus::Running);
    db.save_instance(&state)?;

    println!("\n✓ Instance '{}' started successfully", name);
    println!("  PID: {}", pid);
    println!("  RPC Endpoint: http://localhost:{}", state.config.rpc_port);
    println!("  Serial Log: {}", state.serial_log.unwrap().display());
    println!("\nWait a few seconds for katana to initialize, then test with:");
    println!("  curl -X POST http://localhost:{} \\", state.config.rpc_port);
    println!("    -H 'Content-Type: application/json' \\");
    println!("    -d '{{\"jsonrpc\":\"2.0\",\"method\":\"starknet_chainId\",\"params\":[],\"id\":1}}'");

    Ok(())
}
