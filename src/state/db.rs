use crate::{
    instance::{InstanceConfig, InstanceState, InstanceStatus},
    HypervisorError, Result,
};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS instances (
    id TEXT PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    status TEXT NOT NULL,
    config_json TEXT NOT NULL,
    vm_pid INTEGER,
    qmp_socket TEXT,
    serial_log TEXT,
    tee_mode BOOLEAN NOT NULL,
    expected_measurement TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS ports (
    port INTEGER PRIMARY KEY,
    instance_id TEXT NOT NULL,
    port_type TEXT NOT NULL,
    FOREIGN KEY (instance_id) REFERENCES instances(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS boot_components (
    instance_id TEXT NOT NULL,
    component_type TEXT NOT NULL,
    file_path TEXT NOT NULL,
    sha256_hash TEXT NOT NULL,
    PRIMARY KEY (instance_id, component_type),
    FOREIGN KEY (instance_id) REFERENCES instances(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_instances_status ON instances(status);
CREATE INDEX IF NOT EXISTS idx_ports_instance ON ports(instance_id);
"#;

#[derive(Clone)]
pub struct StateDatabase {
    conn: Arc<Mutex<Connection>>,
}

impl StateDatabase {
    pub fn new(path: &Path) -> Result<Self> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;

        // Enable foreign keys
        conn.execute("PRAGMA foreign_keys = ON", [])?;

        // Create schema
        conn.execute_batch(SCHEMA_SQL)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn save_instance(&self, state: &InstanceState) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        let status_str = serde_json::to_string(&state.status)?;
        let config_json = serde_json::to_string(&state.config)?;

        let qmp_socket_str = state.qmp_socket.as_ref().map(|p| p.to_string_lossy().to_string());
        let serial_log_str = state.serial_log.as_ref().map(|p| p.to_string_lossy().to_string());

        // Check if instance exists by ID
        let exists: bool = conn.query_row(
            "SELECT COUNT(*) FROM instances WHERE id = ?1",
            [&state.id],
            |row| row.get::<_, i64>(0).map(|c| c > 0)
        )?;

        if exists {
            // Update existing instance
            conn.execute(
                "UPDATE instances
                 SET name = ?2, status = ?3, config_json = ?4, vm_pid = ?5, qmp_socket = ?6,
                     serial_log = ?7, tee_mode = ?8, expected_measurement = ?9, updated_at = ?10
                 WHERE id = ?1",
                params![
                    state.id,
                    state.name,
                    status_str,
                    config_json,
                    state.vm_pid,
                    qmp_socket_str,
                    serial_log_str,
                    state.config.tee_mode,
                    state.config.expected_measurement,
                    chrono::Utc::now().timestamp(), // Always update timestamp on save
                ],
            )?;
        } else {
            // Insert new instance
            conn.execute(
                "INSERT INTO instances
                 (id, name, status, config_json, vm_pid, qmp_socket, serial_log, tee_mode, expected_measurement, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    state.id,
                    state.name,
                    status_str,
                    config_json,
                    state.vm_pid,
                    qmp_socket_str,
                    serial_log_str,
                    state.config.tee_mode,
                    state.config.expected_measurement,
                    state.created_at,
                    state.updated_at,
                ],
            )?;
        }

        Ok(())
    }

    pub fn get_instance(&self, name: &str) -> Result<InstanceState> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, name, status, config_json, vm_pid, qmp_socket, serial_log, created_at, updated_at
             FROM instances
             WHERE name = ?1"
        )?;

        let state = stmt.query_row([name], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let status_str: String = row.get(2)?;
            let config_json: String = row.get(3)?;
            let vm_pid: Option<i32> = row.get(4)?;
            let qmp_socket_str: Option<String> = row.get(5)?;
            let serial_log_str: Option<String> = row.get(6)?;
            let created_at: i64 = row.get(7)?;
            let updated_at: i64 = row.get(8)?;

            let status: InstanceStatus = serde_json::from_str(&status_str)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let config: InstanceConfig = serde_json::from_str(&config_json)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

            let qmp_socket = qmp_socket_str.map(|s| std::path::PathBuf::from(s));
            let serial_log = serial_log_str.map(|s| std::path::PathBuf::from(s));

            Ok(InstanceState {
                id,
                name,
                status,
                config,
                vm_pid,
                qmp_socket,
                serial_log,
                created_at,
                updated_at,
            })
        }).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => HypervisorError::InstanceNotFound(name.to_string()),
            e => HypervisorError::Database(e),
        })?;

        Ok(state)
    }

    pub fn get_instance_by_id(&self, id: &str) -> Result<InstanceState> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, name, status, config_json, vm_pid, qmp_socket, serial_log, created_at, updated_at
             FROM instances
             WHERE id = ?1"
        )?;

        let state = stmt.query_row([id], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let status_str: String = row.get(2)?;
            let config_json: String = row.get(3)?;
            let vm_pid: Option<i32> = row.get(4)?;
            let qmp_socket_str: Option<String> = row.get(5)?;
            let serial_log_str: Option<String> = row.get(6)?;
            let created_at: i64 = row.get(7)?;
            let updated_at: i64 = row.get(8)?;

            let status: InstanceStatus = serde_json::from_str(&status_str)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let config: InstanceConfig = serde_json::from_str(&config_json)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

            let qmp_socket = qmp_socket_str.map(|s| std::path::PathBuf::from(s));
            let serial_log = serial_log_str.map(|s| std::path::PathBuf::from(s));

            Ok(InstanceState {
                id,
                name,
                status,
                config,
                vm_pid,
                qmp_socket,
                serial_log,
                created_at,
                updated_at,
            })
        }).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => HypervisorError::InstanceNotFound(id.to_string()),
            e => HypervisorError::Database(e),
        })?;

        Ok(state)
    }

    pub fn list_instances(&self) -> Result<Vec<InstanceState>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, name, status, config_json, vm_pid, qmp_socket, serial_log, created_at, updated_at
             FROM instances
             ORDER BY created_at DESC"
        )?;

        let instances = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let status_str: String = row.get(2)?;
            let config_json: String = row.get(3)?;
            let vm_pid: Option<i32> = row.get(4)?;
            let qmp_socket_str: Option<String> = row.get(5)?;
            let serial_log_str: Option<String> = row.get(6)?;
            let created_at: i64 = row.get(7)?;
            let updated_at: i64 = row.get(8)?;

            let status: InstanceStatus = serde_json::from_str(&status_str)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let config: InstanceConfig = serde_json::from_str(&config_json)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

            let qmp_socket = qmp_socket_str.map(|s| std::path::PathBuf::from(s));
            let serial_log = serial_log_str.map(|s| std::path::PathBuf::from(s));

            Ok(InstanceState {
                id,
                name,
                status,
                config,
                vm_pid,
                qmp_socket,
                serial_log,
                created_at,
                updated_at,
            })
        })?;

        let mut result = Vec::new();
        for instance in instances {
            result.push(instance?);
        }

        Ok(result)
    }

    pub fn delete_instance(&self, name: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        let rows_affected = conn.execute("DELETE FROM instances WHERE name = ?1", [name])?;

        if rows_affected == 0 {
            return Err(HypervisorError::InstanceNotFound(name.to_string()));
        }

        Ok(())
    }

    pub fn allocate_port(&self, instance_id: &str, port: u16, port_type: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO ports (port, instance_id, port_type) VALUES (?1, ?2, ?3)",
            params![port, instance_id, port_type],
        )?;

        Ok(())
    }

    pub fn get_allocated_ports(&self) -> Result<Vec<u16>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare("SELECT port FROM ports ORDER BY port")?;
        let ports = stmt.query_map([], |row| row.get(0))?;

        let mut result = Vec::new();
        for port in ports {
            result.push(port?);
        }

        Ok(result)
    }

    pub fn instance_exists(&self, name: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM instances WHERE name = ?1",
            [name],
            |row| row.get(0)
        )?;
        Ok(count > 0)
    }
}

#[cfg(test)]
#[path = "db_tests.rs"]
mod db_tests;
