use anyhow::Result;

use katana_client::Client;

pub async fn execute(client: &Client, name: String) -> Result<()> {
    client.delete_instance(&name).await?;

    println!("✓ Instance '{}' deleted successfully!", name);

    Ok(())
}
