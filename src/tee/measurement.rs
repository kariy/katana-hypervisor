use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct MeasurementCalculator {
    /// Path to katana's calculate-measurement.sh script
    script_path: PathBuf,
}

impl MeasurementCalculator {
    pub fn new(katana_repo_path: &Path) -> Self {
        let script_path = katana_repo_path
            .join("tee")
            .join("scripts")
            .join("calculate-measurement.sh");

        Self { script_path }
    }

    /// Calculate SEV-SNP launch measurement for given boot components
    pub fn calculate(
        &self,
        ovmf_path: &Path,
        kernel_path: Option<&Path>,
        initrd_path: Option<&Path>,
        kernel_cmdline: Option<&str>,
        vcpus: u32,
        vcpu_type: &str,
    ) -> Result<String> {
        // Verify script exists
        if !self.script_path.exists() {
            anyhow::bail!(
                "Measurement script not found at: {}",
                self.script_path.display()
            );
        }

        // Verify OVMF exists
        if !ovmf_path.exists() {
            anyhow::bail!("OVMF file not found at: {}", ovmf_path.display());
        }

        // Build command
        let mut cmd = Command::new("bash");
        cmd.arg(&self.script_path);
        cmd.arg(ovmf_path);

        // Add optional kernel boot parameters
        if let Some(kernel) = kernel_path {
            if !kernel.exists() {
                anyhow::bail!("Kernel file not found at: {}", kernel.display());
            }
            cmd.arg(kernel);

            if let Some(initrd) = initrd_path {
                if !initrd.exists() {
                    anyhow::bail!("Initrd file not found at: {}", initrd.display());
                }
                cmd.arg(initrd);
            } else {
                cmd.arg("");
            }

            cmd.arg(kernel_cmdline.unwrap_or(""));
        } else {
            // UEFI boot mode - no kernel/initrd
            cmd.arg("");
            cmd.arg("");
            cmd.arg("");
        }

        cmd.arg(vcpus.to_string());
        cmd.arg(vcpu_type);

        // Create temp directory for output
        let temp_dir = tempfile::TempDir::new()?;
        cmd.arg(temp_dir.path());

        // Execute script
        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Measurement calculation failed: {}", stderr);
        }

        // Read measurement from output file
        let measurement_file = temp_dir.path().join("expected-measurement.txt");
        let measurement = std::fs::read_to_string(&measurement_file)?;

        // Trim whitespace and return
        Ok(measurement.trim().to_string())
    }

    /// Calculate measurement and return both hex and JSON output
    pub fn calculate_with_metadata(
        &self,
        ovmf_path: &Path,
        kernel_path: Option<&Path>,
        initrd_path: Option<&Path>,
        kernel_cmdline: Option<&str>,
        vcpus: u32,
        vcpu_type: &str,
    ) -> Result<MeasurementOutput> {
        // Same as calculate but also read JSON file
        if !self.script_path.exists() {
            anyhow::bail!(
                "Measurement script not found at: {}",
                self.script_path.display()
            );
        }

        if !ovmf_path.exists() {
            anyhow::bail!("OVMF file not found at: {}", ovmf_path.display());
        }

        let mut cmd = Command::new("bash");
        cmd.arg(&self.script_path);
        cmd.arg(ovmf_path);

        if let Some(kernel) = kernel_path {
            if !kernel.exists() {
                anyhow::bail!("Kernel file not found at: {}", kernel.display());
            }
            cmd.arg(kernel);

            if let Some(initrd) = initrd_path {
                if !initrd.exists() {
                    anyhow::bail!("Initrd file not found at: {}", initrd.display());
                }
                cmd.arg(initrd);
            } else {
                cmd.arg("");
            }

            cmd.arg(kernel_cmdline.unwrap_or(""));
        } else {
            cmd.arg("");
            cmd.arg("");
            cmd.arg("");
        }

        cmd.arg(vcpus.to_string());
        cmd.arg(vcpu_type);

        let temp_dir = tempfile::TempDir::new()?;
        cmd.arg(temp_dir.path());

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Measurement calculation failed: {}", stderr);
        }

        // Read both output files
        let measurement = std::fs::read_to_string(temp_dir.path().join("expected-measurement.txt"))?;
        let json_output = std::fs::read_to_string(temp_dir.path().join("expected-measurement.json"))?;

        Ok(MeasurementOutput {
            measurement: measurement.trim().to_string(),
            json_metadata: json_output,
        })
    }
}

#[derive(Debug, Clone)]
pub struct MeasurementOutput {
    pub measurement: String,
    pub json_metadata: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_measurement_calculator_creation() {
        let calculator = MeasurementCalculator::new(Path::new("/home/ubuntu/katana"));
        assert!(calculator.script_path.ends_with("calculate-measurement.sh"));
    }

    // Note: Actual measurement tests require sev-snp-measure tool and valid boot components
    // These would be integration tests run on SEV-SNP hardware
}
