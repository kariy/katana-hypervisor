use anyhow::Result;

use crate::{config::OutputFormat, format, models::ToJsonValue};
use katana_client::Client;

pub async fn execute(client: &Client, name: String, output_format: &OutputFormat) -> Result<()> {
    let response = client.get_instance(&name).await?;

    match output_format {
        OutputFormat::Json => format::print_json(&response.to_json_value()),
        OutputFormat::Table => format::print_instance_details(&response.to_json_value()),
    }

    Ok(())
}
