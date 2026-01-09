use serde::{Deserialize, Serialize};

// ============================================================================
// Request Types
// ============================================================================

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateInstanceRequest {
    pub name: String,
    pub vcpus: u32,
    pub memory: String,  // e.g., "4G", "2048M"
    pub storage: String, // e.g., "10G", "5120M"
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default = "default_dev")]
    pub dev: bool,
    #[serde(default)]
    pub tee: bool,
    #[serde(default = "default_vcpu_type")]
    pub vcpu_type: String,
    #[serde(default)]
    pub chain_id: Option<String>,
    #[serde(default)]
    pub block_time: Option<u64>,
    #[serde(default)]
    pub accounts: Option<u16>,
    #[serde(default)]
    pub disable_fee: bool,
    #[serde(default)]
    pub extra_args: Vec<String>,
}

fn default_dev() -> bool {
    true
}

fn default_vcpu_type() -> String {
    "host".to_string()
}

// ============================================================================
// Response Types - Instances
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct InstanceResponse {
    pub id: String,
    pub name: String,
    pub status: String,
    pub config: InstanceConfigResponse,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<EndpointsResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstanceConfigResponse {
    pub vcpus: u32,
    pub memory_mb: u64,
    pub storage_bytes: u64,
    pub rpc_port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics_port: Option<u16>,
    pub tee_mode: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EndpointsResponse {
    pub rpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListInstancesResponse {
    pub instances: Vec<InstanceResponse>,
    pub total: usize,
}

// ============================================================================
// Response Types - Logs
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct LogsResponse {
    pub instance_name: String,
    pub lines: Vec<String>,
    pub total_lines: usize,
}

// ============================================================================
// Response Types - Stats
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct StatsResponse {
    pub instance_name: String,
    pub status: StatusInfo,
    pub config: ConfigInfo,
    pub resources: ResourcesInfo,
    pub network: NetworkInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusInfo {
    pub state: String,
    pub running: bool,
    pub pid: Option<i32>,
    pub uptime: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigInfo {
    pub vcpus: u32,
    pub memory_mb: u64,
    pub rpc_port: u16,
    pub tee_mode: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourcesInfo {
    pub cpu_count: usize,
    pub cpus: Vec<CpuInfo>,
    pub memory_mb: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CpuInfo {
    pub cpu_index: u64,
    pub thread_id: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub rpc_url: String,
    pub health_url: String,
}

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
}
