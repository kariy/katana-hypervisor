use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;

pub struct BuildPipeline {
    katana_repo_path: PathBuf,
}

impl BuildPipeline {
    pub fn new(katana_repo_path: PathBuf) -> Self {
        Self { katana_repo_path }
    }

    /// Build VM image (kernel + initrd + OVMF) using katana's build pipeline
    pub fn build_vm_image(&self) -> Result<BootComponents> {
        let build_script = self.katana_repo_path.join("tee/scripts/build-vm-image.sh");

        if !build_script.exists() {
            anyhow::bail!(
                "Build script not found at: {}",
                build_script.display()
            );
        }

        println!("Building VM image (this may take several minutes)...");

        // Execute build script
        let output = Command::new("bash")
            .arg(&build_script)
            .current_dir(&self.katana_repo_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("VM image build failed: {}", stderr);
        }

        // Boot components are typically output to tee/build/
        let build_dir = self.katana_repo_path.join("tee/build");

        let kernel_path = build_dir.join("vmlinuz");
        let initrd_path = build_dir.join("initrd.img");
        let ovmf_path = build_dir.join("ovmf.fd");

        // Verify files exist
        if !kernel_path.exists() {
            anyhow::bail!("Kernel not found at: {}", kernel_path.display());
        }
        if !initrd_path.exists() {
            anyhow::bail!("Initrd not found at: {}", initrd_path.display());
        }
        if !ovmf_path.exists() {
            anyhow::bail!("OVMF not found at: {}", ovmf_path.display());
        }

        Ok(BootComponents {
            kernel_path,
            initrd_path,
            ovmf_path,
        })
    }

    /// Check if boot components already exist
    pub fn boot_components_exist(&self) -> bool {
        let build_dir = self.katana_repo_path.join("tee/build");

        let kernel_exists = build_dir.join("vmlinuz").exists();
        let initrd_exists = build_dir.join("initrd.img").exists();
        let ovmf_exists = build_dir.join("ovmf.fd").exists();

        kernel_exists && initrd_exists && ovmf_exists
    }

    /// Get paths to existing boot components without building
    pub fn get_boot_components(&self) -> Result<BootComponents> {
        let build_dir = self.katana_repo_path.join("tee/build");

        let kernel_path = build_dir.join("vmlinuz");
        let initrd_path = build_dir.join("initrd.img");
        let ovmf_path = build_dir.join("ovmf.fd");

        if !kernel_path.exists() || !initrd_path.exists() || !ovmf_path.exists() {
            anyhow::bail!(
                "Boot components not found. Run build-vm-image first or use --build-image flag."
            );
        }

        Ok(BootComponents {
            kernel_path,
            initrd_path,
            ovmf_path,
        })
    }
}

#[derive(Debug, Clone)]
pub struct BootComponents {
    pub kernel_path: PathBuf,
    pub initrd_path: PathBuf,
    pub ovmf_path: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_pipeline_creation() {
        let pipeline = BuildPipeline::new(PathBuf::from("/home/ubuntu/katana"));
        assert_eq!(pipeline.katana_repo_path, PathBuf::from("/home/ubuntu/katana"));
    }

    #[test]
    fn test_boot_components_paths() {
        let pipeline = BuildPipeline::new(PathBuf::from("/home/ubuntu/katana"));

        // This will likely return false unless you've actually built the components
        let _exists = pipeline.boot_components_exist();

        // No assertion - just verify it doesn't panic
    }

    // Note: Actual build tests require Docker and can take several minutes
    // These would be integration tests marked with #[ignore] by default
}
