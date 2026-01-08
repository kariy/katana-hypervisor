use crate::{qemu::VmManager, state::StateDatabase};
use anyhow::Result;

pub fn execute(db: &StateDatabase, vm_manager: &VmManager) -> Result<()> {
    // Get all instances
    let instances = db.list_instances()?;

    if instances.is_empty() {
        println!("No instances found.");
        println!("\nCreate one with: katana-hypervisor create <name>");
        return Ok(());
    }

    // Print header
    println!("{:<20} {:<12} {:<8} {:<10} {:<10} {:<8}", "NAME", "STATUS", "VCPUS", "MEMORY", "PORT", "PID");
    println!("{}", "-".repeat(80));

    // Print instances
    for state in instances {
        let status_str = match state.status {
            crate::instance::InstanceStatus::Running => {
                // Check if process is actually running
                if let Some(pid) = state.vm_pid {
                    if vm_manager.is_process_running(pid) {
                        "running".to_string()
                    } else {
                        "stopped*".to_string() // Process died
                    }
                } else {
                    format!("{}", state.status)
                }
            }
            _ => format!("{}", state.status),
        };

        let pid_str = state.vm_pid.map(|p| p.to_string()).unwrap_or("-".to_string());

        println!(
            "{:<20} {:<12} {:<8} {:<10} {:<10} {:<8}",
            state.name,
            status_str,
            state.config.vcpus,
            format!("{}M", state.config.memory_mb),
            state.config.rpc_port,
            pid_str
        );
    }

    println!();

    Ok(())
}
