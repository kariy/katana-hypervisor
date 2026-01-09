// Re-export shared models from katana-models
pub use katana_models::*;

// Local helper methods for CLI formatting
pub trait ToJsonValue {
    fn to_json_value(&self) -> serde_json::Value;
}

impl ToJsonValue for InstanceResponse {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}
