use anyhow::Result;
use serde_json::Value;

use katana_client::Client;

pub async fn execute(
    client: &Client,
    name: String,
    tail: Option<usize>,
    follow: bool,
) -> Result<()> {
    if follow {
        // Stream mode
        println!("Streaming logs for '{}' (Ctrl+C to exit)...\n", name);

        client
            .stream_logs(&name, tail, |event_type, data| {
                match event_type.as_str() {
                    "init" => {
                        // Initial connection message (optional to display)
                    }
                    "log" => {
                        // Parse JSON and print log line
                        if let Ok(json) = serde_json::from_str::<Value>(&data) {
                            if let Some(line) = json["line"].as_str() {
                                println!("{}", line);
                            }
                        }
                    }
                    "error" => {
                        eprintln!("\nError: {}", data);
                    }
                    _ => {}
                }
            })
            .await?;
    } else {
        // Non-streaming mode (existing behavior)
        // Fetch logs from daemon
        let response = client.get_logs(&name, tail).await?;

        // Display logs
        if response.lines.is_empty() {
            println!("No logs available for instance '{}'", name);
            println!("(Total lines in log file: {})", response.total_lines);
        } else {
            // Print logs
            for line in &response.lines {
                println!("{}", line);
            }

            // Print summary if we didn't show all lines
            if response.total_lines > response.lines.len() {
                println!();
                println!(
                    "Showing last {} of {} total lines",
                    response.lines.len(),
                    response.total_lines
                );
            }
        }
    }

    Ok(())
}
