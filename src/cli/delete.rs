use crate::{
    instance::{InstanceStatus, StorageManager},
    qemu::VmManager,
    state::StateDatabase,
};
use anyhow::Result;

pub fn execute(
    name: &str,
    force: bool,
    db: &StateDatabase,
    storage: &StorageManager,
    vm_manager: &VmManager,
) -> Result<()> {
    tracing::info!("Deleting instance: {}", name);

    // Load instance from database
    let state = db.get_instance(name)?;

    // Check if running
    if matches!(state.status, InstanceStatus::Running) {
        if !force {
            anyhow::bail!(
                "Instance '{}' is running. Stop it first or use --force to force delete",
                name
            );
        }

        // Force stop
        if let Some(pid) = state.vm_pid {
            println!("Force stopping VM (PID: {})...", pid);
            let _ = vm_manager.kill_vm(pid); // Ignore errors
        }
    }

    // Delete storage
    println!("Deleting storage directory...");
    storage.delete_instance_storage(&state.id)?;

    // Delete from database (this cascades to ports and boot_components)
    db.delete_instance(name)?;

    println!("✓ Instance '{}' deleted successfully", name);

    Ok(())
}
