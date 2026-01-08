/// SEV-SNP (Secure Encrypted Virtualization - Secure Nested Paging) configuration
/// This is already defined in qemu/config.rs and used there.
/// This module re-exports it for convenience and adds helper methods.

pub use crate::qemu::config::SevSnpConfig;

impl SevSnpConfig {
    /// Create default SEV-SNP configuration for AMD EPYC processors
    pub fn default_epyc() -> Self {
        Self {
            cbitpos: 51,                  // C-bit position for AMD EPYC
            reduced_phys_bits: 1,         // Reserved physical address bits
            vcpu_type: "EPYC-v4".to_string(), // CPU model for SEV-SNP
        }
    }

    /// Check if SEV-SNP is available on the system
    pub fn is_available() -> bool {
        // Check for /dev/sev-guest device
        std::path::Path::new("/dev/sev-guest").exists()
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<(), String> {
        // Validate cbitpos (typically 47 or 51 for x86-64)
        if self.cbitpos > 63 {
            return Err(format!("Invalid cbitpos: {} (must be <= 63)", self.cbitpos));
        }

        // Validate reduced_phys_bits
        if self.reduced_phys_bits > 10 {
            return Err(format!(
                "Invalid reduced_phys_bits: {} (must be <= 10)",
                self.reduced_phys_bits
            ));
        }

        // Validate vcpu_type
        if self.vcpu_type.is_empty() {
            return Err("vcpu_type cannot be empty".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_epyc_config() {
        let config = SevSnpConfig::default_epyc();
        assert_eq!(config.cbitpos, 51);
        assert_eq!(config.reduced_phys_bits, 1);
        assert_eq!(config.vcpu_type, "EPYC-v4");
    }

    #[test]
    fn test_validate_config() {
        let mut config = SevSnpConfig::default_epyc();
        assert!(config.validate().is_ok());

        // Test invalid cbitpos
        config.cbitpos = 100;
        assert!(config.validate().is_err());

        // Test invalid reduced_phys_bits
        config.cbitpos = 51;
        config.reduced_phys_bits = 20;
        assert!(config.validate().is_err());

        // Test empty vcpu_type
        config.reduced_phys_bits = 1;
        config.vcpu_type = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_is_available() {
        // This test will pass or fail depending on hardware
        // On non-SEV hardware, should return false
        let _available = SevSnpConfig::is_available();
        // No assertion - just verify it doesn't panic
    }
}
