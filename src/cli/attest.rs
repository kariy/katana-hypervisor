use anyhow::Result;

use crate::state::StateDatabase;
use crate::tee::attestation::AttestationVerifier;

pub fn execute(name: &str, db: &StateDatabase, output_json: bool) -> Result<()> {
    println!("===========================================");
    println!(" SEV-SNP Attestation Verification");
    println!("===========================================");
    println!();

    // Load instance from database
    let instance = db.get_instance(name)?;

    // Verify instance is running
    if !matches!(instance.status, crate::instance::InstanceStatus::Running) {
        anyhow::bail!(
            "Instance '{}' is not running (status: {:?})",
            name,
            instance.status
        );
    }

    // Verify instance has TEE mode enabled
    if !instance.config.tee_mode {
        anyhow::bail!(
            "Instance '{}' is not running in TEE mode.\nCreate with --tee flag to enable SEV-SNP.",
            name
        );
    }

    // Verify instance has expected measurement
    let expected_measurement = instance
        .config
        .expected_measurement
        .as_ref()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No expected measurement found for instance '{}'.\nThis should have been calculated during instance creation.",
                name
            )
        })?;

    // Build RPC URL
    let rpc_url = format!("http://localhost:{}", instance.config.rpc_port);

    println!("Instance:    {}", name);
    println!("RPC URL:     {}", rpc_url);
    println!("TEE Mode:    SEV-SNP");
    println!("Expected:    {}", expected_measurement);
    println!();

    // Create verifier and perform attestation
    println!("Requesting attestation quote from Katana...");

    let verifier = AttestationVerifier::new();

    // Use tokio runtime for async operation
    let runtime = tokio::runtime::Runtime::new()?;
    let result = runtime.block_on(verifier.verify_attestation(&rpc_url, expected_measurement))?;

    println!();

    if output_json {
        // Output as JSON
        let json = serde_json::to_string_pretty(&result)?;
        println!("{}", json);
    } else {
        // Human-readable output
        println!("Blockchain State:");
        println!("  Block Number: {}", result.block_number);
        println!("  Block Hash:   {}", result.block_hash);
        println!("  State Root:   {}", result.state_root);
        println!();

        println!("Measurement Verification:");
        println!("  Expected: {}", result.expected_measurement);
        println!("  Actual:   {}", result.actual_measurement);
        println!();

        if result.verified {
            println!("===========================================");
            println!(" ✓ ATTESTATION VERIFIED");
            println!("===========================================");
            println!();
            println!("The running Katana instance was launched with");
            println!("the expected boot components. This proves:");
            println!();
            println!("  1. The Katana binary matches the reproducible build");
            println!("  2. The kernel and initrd have not been tampered with");
            println!("  3. The launch measurement is cryptographically bound");
            println!();
        } else {
            println!("===========================================");
            println!(" ✗ ATTESTATION FAILED");
            println!("===========================================");
            println!();
            println!("The measurements do NOT match!");
            println!();
            println!("This indicates the running instance was NOT");
            println!("launched with the expected boot components.");
            println!();
            println!("Possible causes:");
            println!("  1. Different kernel or initrd was used");
            println!("  2. The Katana binary was modified");
            println!("  3. Different OVMF firmware or kernel cmdline");
            println!("  4. The expected measurement is outdated");
            println!();

            return Err(anyhow::anyhow!("Attestation verification failed"));
        }
    }

    Ok(())
}
