use anyhow::Result;
use serde::{Deserialize, Serialize};

pub struct AttestationVerifier;

impl AttestationVerifier {
    pub fn new() -> Self {
        Self
    }

    /// Call katana's tee_generateQuote RPC endpoint and verify attestation
    pub async fn verify_attestation(
        &self,
        rpc_url: &str,
        expected_measurement: &str,
    ) -> Result<AttestationResult> {
        // Call RPC endpoint
        let quote_response = self.call_generate_quote(rpc_url).await?;

        // Extract measurement from attestation report at offset 0x90
        let actual_measurement =
            Self::extract_measurement_from_report(&quote_response.quote)?;

        // Compare measurements
        let verified = actual_measurement.eq_ignore_ascii_case(expected_measurement);

        Ok(AttestationResult {
            verified,
            expected_measurement: expected_measurement.to_string(),
            actual_measurement,
            block_number: quote_response.block_number,
            block_hash: quote_response.block_hash,
            state_root: quote_response.state_root,
            quote_hex: quote_response.quote,
        })
    }

    /// Call tee_generateQuote RPC endpoint
    async fn call_generate_quote(&self, url: &str) -> Result<QuoteResponse> {
        let client = reqwest::Client::new();

        let rpc_request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "tee_generateQuote",
            "params": [],
            "id": 1
        });

        let response = client
            .post(url)
            .json(&rpc_request)
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("RPC request failed with status: {}", response.status());
        }

        let json: serde_json::Value = response.json().await?;

        // Check for RPC errors
        if let Some(error) = json.get("error") {
            let error_msg = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error");
            anyhow::bail!("RPC error: {}", error_msg);
        }

        // Extract result
        let result = json
            .get("result")
            .ok_or_else(|| anyhow::anyhow!("No result in RPC response"))?;

        let quote = result
            .get("quote")
            .and_then(|q| q.as_str())
            .ok_or_else(|| anyhow::anyhow!("No quote in response"))?
            .to_string();

        let block_number = result
            .get("blockNumber")
            .and_then(|b| b.as_u64())
            .unwrap_or(0);

        let block_hash = result
            .get("blockHash")
            .and_then(|h| h.as_str())
            .unwrap_or("")
            .to_string();

        let state_root = result
            .get("stateRoot")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();

        Ok(QuoteResponse {
            quote,
            block_number,
            block_hash,
            state_root,
        })
    }

    /// Extract measurement from SEV-SNP attestation report
    /// The measurement is at offset 0x90 (144 bytes), length 48 bytes
    fn extract_measurement_from_report(report_hex: &str) -> Result<String> {
        // Remove 0x prefix if present
        let report_hex = report_hex.trim_start_matches("0x");

        // Convert hex string to bytes
        let report_bytes = hex::decode(report_hex)?;

        // Verify report is large enough
        if report_bytes.len() < 144 + 48 {
            anyhow::bail!(
                "Attestation report too small: {} bytes (expected at least 192)",
                report_bytes.len()
            );
        }

        // Extract measurement at offset 0x90 (144 bytes), length 48 bytes
        let measurement_bytes = &report_bytes[144..144 + 48];

        // Convert to hex string
        Ok(hex::encode(measurement_bytes))
    }
}

impl Default for AttestationVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteResponse {
    pub quote: String,
    pub block_number: u64,
    pub block_hash: String,
    pub state_root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationResult {
    pub verified: bool,
    pub expected_measurement: String,
    pub actual_measurement: String,
    pub block_number: u64,
    pub block_hash: String,
    pub state_root: String,
    pub quote_hex: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_measurement_from_report() {
        // Create a mock report with 1184 bytes (SEV-SNP report size)
        let mut report_bytes = vec![0u8; 1184];

        // Set a recognizable pattern at offset 0x90 (144 bytes)
        for i in 0..48 {
            report_bytes[144 + i] = (i as u8) + 1;
        }

        let report_hex = hex::encode(&report_bytes);

        // Extract measurement
        let measurement = AttestationVerifier::extract_measurement_from_report(&report_hex).unwrap();

        // Verify it extracted the correct 48 bytes
        let expected = hex::encode((1u8..=48).collect::<Vec<u8>>());
        assert_eq!(measurement, expected);
    }

    #[test]
    fn test_extract_measurement_with_0x_prefix() {
        let mut report_bytes = vec![0u8; 1184];
        for i in 0..48 {
            report_bytes[144 + i] = 0xAA;
        }

        let report_hex = format!("0x{}", hex::encode(&report_bytes));

        let measurement = AttestationVerifier::extract_measurement_from_report(&report_hex).unwrap();

        assert_eq!(measurement, "a".repeat(96)); // 48 bytes of 0xAA = 96 hex chars
    }

    #[test]
    fn test_extract_measurement_report_too_small() {
        let report_bytes = vec![0u8; 100]; // Too small
        let report_hex = hex::encode(&report_bytes);

        let result = AttestationVerifier::extract_measurement_from_report(&report_hex);
        assert!(result.is_err());
    }

    // Note: Full integration tests require running katana with SEV-SNP
    // and actual hardware attestation support
}
