use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Table};
use serde_json::Value;

pub fn print_json(value: &Value) {
    println!("{}", serde_json::to_string_pretty(value).unwrap());
}

pub fn print_instance_list(instances: &[Value]) {
    if instances.is_empty() {
        println!("No instances found.");
        return;
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec!["NAME", "STATUS", "VCPUS", "MEMORY", "RPC PORT"]);

    for instance in instances {
        let name = instance["name"].as_str().unwrap_or("N/A");
        let status = instance["status"].as_str().unwrap_or("N/A");
        let vcpus = instance["config"]["vcpus"].as_u64().unwrap_or(0);
        let memory_mb = instance["config"]["memory_mb"].as_u64().unwrap_or(0);
        let rpc_port = instance["config"]["rpc_port"].as_u64().unwrap_or(0);

        table.add_row(vec![
            name.to_string(),
            status.to_string(),
            vcpus.to_string(),
            format!("{} MB", memory_mb),
            rpc_port.to_string(),
        ]);
    }

    println!("{table}");
}

pub fn print_instance_details(instance: &Value) {
    let name = instance["name"].as_str().unwrap_or("N/A");
    let id = instance["id"].as_str().unwrap_or("N/A");
    let status = instance["status"].as_str().unwrap_or("N/A");
    let vcpus = instance["config"]["vcpus"].as_u64().unwrap_or(0);
    let memory_mb = instance["config"]["memory_mb"].as_u64().unwrap_or(0);
    let storage_gb = instance["config"]["storage_bytes"].as_u64().unwrap_or(0) / 1_000_000_000;
    let rpc_port = instance["config"]["rpc_port"].as_u64().unwrap_or(0);
    let tee_mode = instance["config"]["tee_mode"].as_bool().unwrap_or(false);
    let created_at = instance["created_at"].as_str().unwrap_or("N/A");

    println!("Instance: {}", name);
    println!("  ID:         {}", id);
    println!("  Status:     {}", status);
    println!("  vCPUs:      {}", vcpus);
    println!("  Memory:     {} MB", memory_mb);
    println!("  Storage:    {} GB", storage_gb);
    println!("  RPC Port:   {}", rpc_port);
    println!("  TEE Mode:   {}", if tee_mode { "enabled" } else { "disabled" });
    println!("  Created:    {}", created_at);

    if let Some(endpoints) = instance.get("endpoints") {
        println!("\nEndpoints:");
        if let Some(rpc) = endpoints.get("rpc") {
            println!("  RPC: {}", rpc.as_str().unwrap_or("N/A"));
        }
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
