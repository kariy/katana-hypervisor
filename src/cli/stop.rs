use crate::{instance::InstanceStatus, qemu::VmManager, state::StateDatabase};
use anyhow::Result;

pub fn execute(name: &str, db: &StateDatabase, vm_manager: &VmManager) -> Result<()> {
    tracing::info!("Stopping instance: {}", name);

    // Load instance from database
    let mut state = db.get_instance(name)?;

    // Check state
    match state.status {
        InstanceStatus::Stopped => {
            println!("Instance '{}' is already stopped", name);
            return Ok(());
        }
        InstanceStatus::Stopping => {
            println!("Instance '{}' is already stopping", name);
            return Ok(());
        }
        InstanceStatus::Running => {}
        _ => {
            anyhow::bail!("Instance '{}' is not running (status: {})", name, state.status);
        }
    }

    let pid = state
        .vm_pid
        .ok_or_else(|| anyhow::anyhow!("Instance has no PID"))?;

    // Update status to stopping
    state.update_status(InstanceStatus::Stopping);
    db.save_instance(&state)?;

    println!("Stopping VM (PID: {})...", pid);

    // Stop VM gracefully with 30 second timeout
    vm_manager.stop_vm(pid, 30)?;

    // Update state
    state.vm_pid = None;
    state.update_status(InstanceStatus::Stopped);
    db.save_instance(&state)?;

    println!("✓ Instance '{}' stopped successfully", name);

    Ok(())
}
