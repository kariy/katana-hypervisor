use anyhow::Result;
use std::path::PathBuf;

use crate::qemu::config::QemuConfig;
use crate::tee::measurement::MeasurementCalculator;

pub fn execute(
    kernel: Option<PathBuf>,
    initrd: Option<PathBuf>,
    ovmf: Option<PathBuf>,
    cmdline: Option<String>,
    vcpus: u32,
    vcpu_type: String,
    katana_repo: PathBuf,
    output_json: bool,
) -> Result<()> {
    println!("===========================================");
    println!(" SEV-SNP Measurement Calculator");
    println!("===========================================");
    println!();

    // Create measurement calculator
    let calculator = MeasurementCalculator::new(&katana_repo);

    // Determine OVMF path
    let ovmf_path = if let Some(ovmf) = ovmf {
        ovmf
    } else {
        // Use default from katana build
        let default_ovmf = katana_repo.join("tee/build/ovmf.fd");
        if !default_ovmf.exists() {
            anyhow::bail!(
                "Default OVMF not found at: {}\nSpecify --ovmf or build VM components first",
                default_ovmf.display()
            );
        }
        default_ovmf
    };

    // Determine kernel/initrd paths
    let (kernel_path, initrd_path) = match (kernel, initrd) {
        (Some(k), Some(i)) => {
            // Both provided
            (Some(k), Some(i))
        }
        (None, None) => {
            // Neither provided - try defaults
            let default_kernel = katana_repo.join("tee/build/vmlinuz");
            let default_initrd = katana_repo.join("tee/build/initrd.img");

            if default_kernel.exists() && default_initrd.exists() {
                (Some(default_kernel), Some(default_initrd))
            } else {
                // UEFI boot mode (no kernel/initrd)
                println!("No kernel/initrd specified or found - using UEFI boot mode");
                (None, None)
            }
        }
        _ => {
            anyhow::bail!("Both --kernel and --initrd must be specified together");
        }
    };

    // Build kernel command line if using direct boot
    let cmdline_str = if kernel_path.is_some() {
        cmdline.or_else(|| {
            // Generate default command line with katana args
            let katana_args = vec!["--http.addr=0.0.0.0".to_string(), "--http.port=5050".to_string()];
            Some(QemuConfig::build_kernel_cmdline(&katana_args))
        })
    } else {
        None
    };

    println!("Configuration:");
    println!("  OVMF:      {}", ovmf_path.display());
    if let Some(ref k) = kernel_path {
        println!("  Kernel:    {}", k.display());
    }
    if let Some(ref i) = initrd_path {
        println!("  Initrd:    {}", i.display());
    }
    if let Some(ref c) = cmdline_str {
        println!("  Cmdline:   {}", c);
    }
    println!("  VCPUs:     {}", vcpus);
    println!("  VCPU Type: {}", vcpu_type);
    println!();

    // Calculate measurement
    println!("Calculating measurement...");

    if output_json {
        // Calculate with metadata
        let output = calculator.calculate_with_metadata(
            &ovmf_path,
            kernel_path.as_deref(),
            initrd_path.as_deref(),
            cmdline_str.as_deref(),
            vcpus,
            &vcpu_type,
        )?;

        println!();
        println!("===========================================");
        println!(" Measurement (JSON):");
        println!("===========================================");
        println!("{}", output.json_metadata);
    } else {
        // Calculate plain measurement
        let measurement = calculator.calculate(
            &ovmf_path,
            kernel_path.as_deref(),
            initrd_path.as_deref(),
            cmdline_str.as_deref(),
            vcpus,
            &vcpu_type,
        )?;

        println!();
        println!("===========================================");
        println!(" Measurement:");
        println!("===========================================");
        println!("{}", measurement);
        println!();
        println!("This measurement should match the value");
        println!("extracted from attestation reports when");
        println!("Katana is running in SEV-SNP mode.");
    }

    Ok(())
}
