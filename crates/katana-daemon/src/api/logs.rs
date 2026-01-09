use crate::{error::ApiError, state::DaemonState};
use axum::{
    extract::{Path, Query},
    response::{sse::{Event, KeepAlive}, Json, Sse},
    Extension,
};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::convert::Infallible;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    #[serde(default = "default_tail")]
    pub tail: usize,
}

fn default_tail() -> usize {
    100
}

#[derive(Debug, Serialize)]
pub struct LogsResponse {
    pub instance_name: String,
    pub lines: Vec<String>,
    pub total_lines: usize,
}

/// Get logs from an instance
pub async fn get_logs(
    Extension(state): Extension<Arc<DaemonState>>,
    Path(name): Path<String>,
    Query(params): Query<LogsQuery>,
) -> ApiResult<Json<LogsResponse>> {
    // Load instance from database
    let instance_state = state.db.get_instance(&name)?;

    let log_path = instance_state
        .serial_log
        .ok_or_else(|| ApiError::NotFound("Instance has no serial log".to_string()))?;

    if !log_path.exists() {
        return Err(ApiError::NotFound(format!(
            "Log file not found: {}. Instance may not have been started yet.",
            log_path.display()
        )));
    }

    // Read log file
    let file = File::open(&log_path).map_err(|e| {
        ApiError::Internal(format!("Failed to open log file: {}", e))
    })?;

    let reader = BufReader::new(file);
    let all_lines: Vec<String> = reader
        .lines()
        .collect::<Result<_, _>>()
        .map_err(|e| ApiError::Internal(format!("Failed to read log file: {}", e)))?;

    let total_lines = all_lines.len();

    // Get last N lines
    let start_idx = if all_lines.len() > params.tail {
        all_lines.len() - params.tail
    } else {
        0
    };

    let lines = all_lines[start_idx..].to_vec();

    Ok(Json(LogsResponse {
        instance_name: name,
        lines,
        total_lines,
    }))
}

fn default_stream_tail() -> usize {
    20
}

#[derive(Debug, Deserialize)]
pub struct StreamLogsQuery {
    #[serde(default = "default_stream_tail")]
    pub tail: usize,
}

/// Stream logs via Server-Sent Events
pub async fn stream_logs(
    Extension(state): Extension<Arc<DaemonState>>,
    Path(name): Path<String>,
    Query(params): Query<StreamLogsQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    // Load instance from database
    let instance_state = state.db.get_instance(&name)?;

    let log_path = instance_state
        .serial_log
        .ok_or_else(|| ApiError::NotFound("Instance has no serial log".to_string()))?;

    // Verify log file exists
    if !log_path.exists() {
        return Err(ApiError::NotFound(format!(
            "Log file not found: {}. Instance may not have been started yet.",
            log_path.display()
        )));
    }

    // Create SSE stream
    let stream = create_log_stream(log_path, params.tail, name);

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

// Helper function to create the actual stream
//
// This function creates a Server-Sent Events (SSE) stream that:
// 1. Sends the last N lines from the log file
// 2. Watches for new content and streams it in real-time
// 3. Detects and handles log file rotation (logrotate-compatible)
// 4. Sends periodic heartbeat events during idle periods
//
// File Rotation Detection:
// Supports standard logrotate patterns via two heuristics:
// - Inode change detection (Unix): Detects move/rename/delete-recreate patterns
// - Truncation detection (All platforms): Detects copytruncate patterns
//
// See inline comments in the rotation detection section for details and limitations.
fn create_log_stream(
    log_path: PathBuf,
    tail: usize,
    instance_name: String,
) -> impl Stream<Item = Result<Event, Infallible>> {
    async_stream::stream! {
        // Send init event
        if let Ok(event) = Event::default()
            .event("init")
            .json_data(json!({"type": "init", "instance": instance_name, "tail": tail}))
        {
            yield Ok(event);
        }

        // Open file once and reuse handle
        let mut file = match File::open(&log_path) {
            Ok(f) => f,
            Err(e) => {
                if let Ok(event) = Event::default()
                    .event("error")
                    .json_data(json!({"error": format!("Failed to open log file: {}", e)}))
                {
                    yield Ok(event);
                }
                return;
            }
        };

        // Send last N lines (efficiently for large files)
        {
            // Get file size
            let file_size = match file.seek(SeekFrom::End(0)) {
                Ok(size) => size,
                Err(_) => 0,
            };

            // For small files (< 1MB), use simple approach
            if file_size < 1_000_000 {
                let _ = file.seek(SeekFrom::Start(0));
                let reader = BufReader::new(&file);
                let lines: Vec<String> = reader
                    .lines()
                    .filter_map(|l| l.ok())
                    .collect();

                let start = lines.len().saturating_sub(tail);
                for line in &lines[start..] {
                    if let Ok(event) = Event::default()
                        .event("log")
                        .json_data(json!({"line": line}))
                    {
                        yield Ok(event);
                    }
                }
            } else {
                // For large files, read backwards to find start position
                let mut newline_count = 0;
                let mut pos = file_size;
                let chunk_size = 8192u64;
                let mut start_pos = 0u64;

                while pos > 0 && newline_count < tail {
                    let read_size = chunk_size.min(pos);
                    pos = pos.saturating_sub(read_size);

                    if file.seek(SeekFrom::Start(pos)).is_ok() {
                        let mut buffer = vec![0u8; read_size as usize];
                        if file.read_exact(&mut buffer).is_ok() {
                            // Count newlines in this chunk (scanning backwards)
                            for (i, &byte) in buffer.iter().enumerate().rev() {
                                if byte == b'\n' {
                                    newline_count += 1;
                                    if newline_count >= tail {
                                        start_pos = pos + i as u64 + 1;
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    if newline_count >= tail {
                        break;
                    }
                }

                // Read from start_pos to end
                if file.seek(SeekFrom::Start(start_pos)).is_ok() {
                    let reader = BufReader::new(&file);
                    for line in reader.lines().filter_map(|l| l.ok()) {
                        if let Ok(event) = Event::default()
                            .event("log")
                            .json_data(json!({"line": line}))
                        {
                            yield Ok(event);
                        }
                    }
                }
            }
        }

        // Track current file position (seek to end)
        let mut last_pos = match file.seek(SeekFrom::End(0)) {
            Ok(pos) => pos,
            Err(e) => {
                if let Ok(event) = Event::default()
                    .event("error")
                    .json_data(json!({"error": format!("Failed to seek file: {}", e)}))
                {
                    yield Ok(event);
                }
                return;
            }
        };

        // Track file inode for rotation detection
        #[cfg(unix)]
        let mut current_inode = file.metadata().ok().map(|m| m.ino());

        #[cfg(not(unix))]
        let mut current_inode: Option<u64> = None;

        // Set up file watcher (inotify on Linux, polling fallback)
        #[cfg(feature = "inotify")]
        let mut watcher = match inotify::Inotify::init() {
            Ok(inotify) => {
                // Add watch for file modifications
                if inotify.watches().add(&log_path, inotify::WatchMask::MODIFY).is_ok() {
                    Some(inotify)
                } else {
                    None
                }
            }
            Err(_) => None,
        };

        #[cfg(feature = "inotify")]
        let mut event_buffer = [0u8; 1024];

        // Track last activity for heartbeat
        let mut last_event_time = std::time::Instant::now();
        let heartbeat_interval = Duration::from_secs(30);

        loop {
            // Send heartbeat if no activity for heartbeat_interval
            if last_event_time.elapsed() >= heartbeat_interval {
                if let Ok(event) = Event::default()
                    .event("heartbeat")
                    .json_data(json!({"timestamp": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()}))
                {
                    yield Ok(event);
                    last_event_time = std::time::Instant::now();
                }
            }

            // Wait for file changes (inotify or polling)
            #[cfg(feature = "inotify")]
            let mut file_modified = false;

            #[cfg(feature = "inotify")]
            if let Some(ref mut w) = watcher {
                // Try to read inotify events (non-blocking)
                match w.read_events(&mut event_buffer) {
                    Ok(events) => {
                        for event in events {
                            if event.mask.contains(inotify::EventMask::MODIFY) {
                                file_modified = true;
                                break;
                            }
                        }
                    }
                    Err(_) => {
                        // If read fails, we'll fall back to time-based check
                    }
                }

                // Sleep briefly - if we got a modify event, check immediately after short sleep
                // Otherwise wait longer before next check
                if file_modified {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                } else {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            } else {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }

            #[cfg(not(feature = "inotify"))]
            tokio::time::sleep(Duration::from_millis(500)).await;

            // Check if file still exists
            // NOTE: If the file is deleted and there's any gap before it's recreated,
            // the stream will exit here. This means rotation patterns with timing gaps
            // (e.g., "mv log.txt log.txt.1; sleep 5; touch log.txt") are not supported.
            // For gapless rotation (atomic rename + create), the rotation detection
            // below will handle it correctly.
            if !log_path.exists() {
                if let Ok(event) = Event::default()
                    .event("error")
                    .json_data(json!({"error": "Log file was deleted"}))
                {
                    yield Ok(event);
                }
                break;
            }

            // Detect file rotation using logrotate-compatible heuristics
            //
            // This implementation supports the two most common logrotate patterns:
            //
            // 1. MOVE/RENAME (default logrotate behavior):
            //    Example: mv log.txt log.txt.1 && touch log.txt
            //    Detection: Inode number changes (Unix only)
            //    - When a file is moved/deleted and recreated, it gets a new inode
            //    - We track the inode and detect when it changes
            //
            // 2. COPYTRUNCATE (logrotate with 'copytruncate' option):
            //    Example: cp log.txt log.txt.1 && > log.txt
            //    Detection: File size becomes smaller than last read position
            //    - Works on all platforms (Unix and Windows)
            //    - Detects when file is truncated or replaced with smaller file
            //
            // LIMITATIONS - These patterns are NOT detected:
            // - Copy without truncate (cp log.txt log.txt.1 # no truncation)
            // - Delayed truncation (logs written between copy and truncate are lost)
            // - Rename with gap (if file doesn't exist, stream exits at line 247-255)
            // - Windows move/rename rotation (no inode check on Windows)
            // - Any custom rotation that doesn't follow logrotate conventions
            let mut file_rotated = false;

            // Heuristic 1: Check for inode change (Unix only)
            // This catches move/rename/delete-recreate rotation patterns
            #[cfg(unix)]
            if let Ok(metadata) = std::fs::metadata(&log_path) {
                let new_inode = metadata.ino();
                if let Some(old_inode) = current_inode {
                    if new_inode != old_inode {
                        file_rotated = true;
                        current_inode = Some(new_inode);
                    }
                }
            }

            // Heuristic 2: Check for truncation (all platforms)
            // This catches copytruncate rotation and file replacement with smaller file
            if let Ok(current_size) = file.seek(SeekFrom::End(0)) {
                if current_size < last_pos {
                    file_rotated = true;
                }
            }

            // Handle file rotation - reopen the file
            if file_rotated {
                if let Ok(new_file) = File::open(&log_path) {
                    file = new_file;
                    last_pos = 0;

                    #[cfg(unix)]
                    {
                        current_inode = file.metadata().ok().map(|m| m.ino());
                    }

                    // Update inotify watch
                    #[cfg(feature = "inotify")]
                    if let Some(ref mut w) = watcher {
                        let _ = w.watches().add(&log_path, inotify::WatchMask::MODIFY);
                    }

                    if let Ok(event) = Event::default()
                        .event("info")
                        .json_data(json!({"message": "Log file rotated, reopened"}))
                    {
                        yield Ok(event);
                        last_event_time = std::time::Instant::now();
                    }
                }
            }

            // Read new lines using the same file handle
            if let Ok(current_size) = file.seek(SeekFrom::End(0)) {
                if current_size > last_pos {
                    if file.seek(SeekFrom::Start(last_pos)).is_ok() {
                        let reader = BufReader::new(&file);

                        for line in reader.lines().filter_map(|l| l.ok()) {
                            if let Ok(event) = Event::default()
                                .event("log")
                                .json_data(json!({"line": line}))
                            {
                                yield Ok(event);
                                last_event_time = std::time::Instant::now();
                            }
                        }

                        last_pos = current_size;
                    }
                }
            }
        }
    }
}
