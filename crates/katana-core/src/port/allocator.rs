use crate::{state::StateDatabase, HypervisorError, Result};
use port_scanner::local_port_available;

pub struct PortAllocator {
    db: StateDatabase,
}

impl PortAllocator {
    pub fn new(db: StateDatabase) -> Self {
        Self { db }
    }

    /// Allocate the next available port starting from base_port
    pub fn allocate_port(&self, base_port: u16) -> Result<u16> {
        // Get all allocated ports from database
        let allocated_ports = self.db.get_allocated_ports()?;

        // Find the next available port
        let mut candidate = base_port;
        let max_attempts = 1000;

        for _ in 0..max_attempts {
            // Check if port is not in database
            if !allocated_ports.contains(&candidate) {
                // Check if port is actually available on the system
                if local_port_available(candidate) {
                    return Ok(candidate);
                }
            }
            candidate += 1;
        }

        Err(HypervisorError::NoPortsAvailable)
    }

    /// Check if a specific port is available
    pub fn is_port_available(&self, port: u16) -> Result<bool> {
        let allocated_ports = self.db.get_allocated_ports()?;

        if allocated_ports.contains(&port) {
            return Ok(false);
        }

        Ok(local_port_available(port))
    }
}

#[cfg(test)]
#[path = "allocator_tests.rs"]
mod allocator_tests;
